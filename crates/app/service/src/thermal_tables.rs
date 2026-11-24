//! 熱の結合・表面の編集の表（`POST /thermal/tables`）。節点の対ごとの結合（解析が使った値と出どころ）、表面材、外面ごとの
//! 面積割合、節点の温度の最低・最高を作る。**値・出どころの表示名・割合の合計・節点の一覧はここだけ**で、Web は並べるだけ。
//! 下書き（`req.ops`）は保存と同じ Op を書き込まずに当てる（`preview::snapshot_with`）ので、同じ表を下書きの前後で問い合わせられる。
//! 編集の入口は `edit::thermal_set_ops`（`CommandText::ThermalSet`）。

use crate::analysis::labels::{coupling_kind, coupling_source};
use crate::analysis::{TypedRun, run_typed_in};
use crate::dto::{
    ThermalCoatingRow, ThermalContactRow, ThermalCouplingKind, ThermalCouplingKindView,
    ThermalCouplingRow, ThermalCouplingsTable, ThermalFractionRow, ThermalNodeTemperature,
    ThermalSurfaceRow, ThermalTablesRequest, ThermalTablesView, ThermalUniformOptics,
    ThermalViewFactorRow,
};
use crate::error::Result;
use crate::query::settings::number;
use crate::query::values::authored_quantity;
use crate::units::quantity_auto;
use crate::workspace::Workspace;
use fune_model::design::ThermalSettings;
use fune_model::value::Quantity;
use sat_analysis::thermal::network::ThermalCouplingSource;
use sat_analysis::thermal::{ThermalSteady, ThermalSteadyInput, ThermalSteadyOutput};
use std::collections::{BTreeMap, BTreeSet};

/// 熱の結合・表面の表を作る。`ops` が通らなければ、保存したときと同じエラーを返す。
pub fn thermal_tables(ws: &Workspace, req: &ThermalTablesRequest) -> Result<ThermalTablesView> {
    let snapshot = crate::preview::snapshot_with(ws, &req.ops)?;
    let settings = snapshot.design.analysis.thermal.clone().unwrap_or_default();
    let run: TypedRun<ThermalSteady> = run_typed_in(&snapshot, req.case.as_deref())?;
    Ok(ThermalTablesView {
        couplings: couplings_table(&settings, &run),
        coatings: coating_rows(&settings),
        surfaces: surface_rows(&settings, run.input.as_ref()),
        temperatures: run.output.as_ref().map(temperatures).unwrap_or_default(),
    })
}

fn couplings_table(s: &ThermalSettings, run: &TypedRun<ThermalSteady>) -> ThermalCouplingsTable {
    let kinds = [
        ThermalCouplingKind::Conductive,
        ThermalCouplingKind::Radiative,
    ]
    .map(|kind| ThermalCouplingKindView {
        kind,
        label: coupling_kind(kind),
        dimension: kind_dimension(kind).to_owned(),
    })
    .to_vec();
    let (Some(input), Some(output)) = (&run.input, &run.output) else {
        return ThermalCouplingsTable {
            available: false,
            kinds,
            nodes: Vec::new(),
            rows: Vec::new(),
        };
    };
    let used = &output.couplings;
    let conductive = used.conductive.iter().map(|c| {
        let contact = c.from_contact.as_ref().map(|f| ThermalContactRow {
            area: quantity_auto(f.area.value(), "Area"),
            mounts: f.contact_mounts,
            fixed_mounts: f.fixed_mounts,
        });
        let pair = Pair::of(&c.a, &c.b);
        row(
            ThermalCouplingKind::Conductive,
            pair.written(&s.conductive),
            c.conductance.value(),
            c.source,
            Detail {
                contact,
                view_factor: None,
            },
        )
    });
    let radiative = used.radiative.iter().map(|r| {
        let pair = Pair::of(&r.a, &r.b);
        row(
            ThermalCouplingKind::Radiative,
            pair.written(&s.radiative),
            r.exchange_area.value(),
            r.source,
            Detail {
                contact: None,
                view_factor: r.from_view_factors.as_ref().map(|f| ThermalViewFactorRow {
                    area_factor: quantity_auto(f.area_factor.value(), "Area"),
                    face_pairs: f.face_pairs,
                }),
            },
        )
    });
    ThermalCouplingsTable {
        available: true,
        kinds,
        nodes: input.network.nodes.iter().map(|n| n.id.clone()).collect(),
        rows: conductive.chain(radiative).collect(),
    }
}

/// 結合の値の次元の名前（伝導は W/K、放射は有効放射交換面積 m²）。
fn kind_dimension(kind: ThermalCouplingKind) -> &'static str {
    match kind {
        ThermalCouplingKind::Conductive => "ThermalConductance",
        ThermalCouplingKind::Radiative => "Area",
    }
}

/// 節点の対（解析の出力は ID の昇順）と、設計に書いた向き・値。
struct Pair<'a> {
    a: &'a str,
    b: &'a str,
}

/// 設計に書いてある 1 つの結合（書いた向きの両端と値）。
struct Written<'a> {
    a: &'a str,
    b: &'a str,
    value: &'a Quantity,
}

impl<'a> Pair<'a> {
    fn of(a: &'a str, b: &'a str) -> Self {
        Self { a, b }
    }

    /// 設計に書いた同じ対（どちらの向きでも。無ければ None）。
    fn written(
        &self,
        table: &'a BTreeMap<String, BTreeMap<String, Quantity>>,
    ) -> (Pair<'a>, Option<Written<'a>>) {
        for (a, b) in [(self.a, self.b), (self.b, self.a)] {
            if let Some((b, value)) = table.get(a).and_then(|m| m.get_key_value(b)) {
                let a = table.get_key_value(a).map_or(a, |(k, _)| k.as_str());
                return (Pair { a, b }, Some(Written { a, b, value }));
            }
        }
        (
            Pair {
                a: self.a,
                b: self.b,
            },
            None,
        )
    }
}

/// 出どころごとの内訳（接触面積・形態係数）。
struct Detail {
    contact: Option<ThermalContactRow>,
    view_factor: Option<ThermalViewFactorRow>,
}

fn row(
    kind: ThermalCouplingKind,
    (pair, written): (Pair<'_>, Option<Written<'_>>),
    si: f64,
    source: ThermalCouplingSource,
    detail: Detail,
) -> ThermalCouplingRow {
    let explicit = source == ThermalCouplingSource::Explicit;
    let dimension = kind_dimension(kind);
    let value = match (&written, explicit) {
        (Some(w), true) => authored_quantity(w.value, Some(dimension)),
        _ => quantity_auto(si, dimension),
    };
    // 書いた結合は設計の向き（取り消しと書き換えの宛先）で出す
    let (a, b) = written.as_ref().map_or((pair.a, pair.b), |w| (w.a, w.b));
    ThermalCouplingRow {
        kind,
        a: a.to_owned(),
        b: b.to_owned(),
        value,
        source,
        source_label: coupling_source(source),
        explicit,
        contact: detail.contact,
        view_factor: detail.view_factor,
    }
}

fn coating_rows(s: &ThermalSettings) -> Vec<ThermalCoatingRow> {
    s.coating
        .iter()
        .map(|(id, c)| ThermalCoatingRow {
            id: id.clone(),
            label: c.label.clone(),
            absorptivity: c.absorptivity.map(number),
            emissivity: c.emissivity.map(number),
            used_by: s
                .surface
                .iter()
                .filter(|(_, shares)| shares.contains_key(id))
                .map(|(part, _)| part.clone())
                .collect(),
        })
        .collect()
}

/// 外面の行: 解析の入力に載る面（部品の ID 順）と、表面材を書いた部品（入力に載らなくても出す。直せるように）。
fn surface_rows(s: &ThermalSettings, input: Option<&ThermalSteadyInput>) -> Vec<ThermalSurfaceRow> {
    let from_input: BTreeMap<&str, _> = input
        .map(|i| {
            i.network
                .surfaces
                .iter()
                .map(|f| (f.id.as_str(), f))
                .collect()
        })
        .unwrap_or_default();
    let parts: BTreeSet<&str> = from_input
        .keys()
        .copied()
        .chain(s.surface.keys().map(String::as_str))
        .collect();
    parts
        .into_iter()
        .map(|part| {
            let face = from_input.get(part);
            let shares = s.surface.get(part);
            let uniform = match (shares, face) {
                (None, Some(f)) => {
                    f.absorptivity
                        .zip(f.emissivity)
                        .map(|(a, e)| ThermalUniformOptics {
                            absorptivity: number(a),
                            emissivity: number(e),
                        })
                }
                _ => None,
            };
            let fractions: Vec<ThermalFractionRow> = shares
                .map(|m| {
                    m.iter()
                        .map(|(coating, f)| ThermalFractionRow {
                            coating: coating.clone(),
                            fraction: number(*f),
                        })
                        .collect()
                })
                .unwrap_or_default();
            ThermalSurfaceRow {
                part: part.to_owned(),
                node: face.map(|f| f.node.clone()),
                area: face.map(|f| quantity_auto(f.area.value(), "Area")),
                uniform,
                total: number(shares.map_or(0.0, |m| m.values().sum())),
                fractions,
            }
        })
        .collect()
}

fn temperatures(o: &ThermalSteadyOutput) -> Vec<ThermalNodeTemperature> {
    o.envelope
        .iter()
        .map(|e| ThermalNodeTemperature {
            node: e.node.clone(),
            min: quantity_auto(e.min.value(), "Temperature"),
            max: quantity_auto(e.max.value(), "Temperature"),
            verdict: e.verdict,
        })
        .collect()
}
