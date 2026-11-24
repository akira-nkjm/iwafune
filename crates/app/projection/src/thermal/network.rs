//! 熱ネットワークの射影: 節点（部品の質量・比熱・3 種の温度範囲・電源の状態を数える冗長と語彙）、取付の熱結合、外面（面積・法線・α/ε か表面材）。
//! 範囲の共通部分・表面材の混ぜ方・熱結合の計算は sat-analysis が行う。ここは部品と設定の値を集めるだけ。

use super::{thermal, thermal_nodes};
use crate::context::Context;
use crate::missing::MissingInputs;
use crate::parts::direction_body;
use crate::quantity::{setting, setting_opt};
use fune_model::Target;
use fune_model::design::setting_dims as dims;
use fune_model::design::setting_dims::SettingQuantity;
use fune_model::design::{ThermalNodes, ThermalSettings};
use fune_model::resolve::{ResolvedPart, SiValue};
use fune_model::value::Quantity;
use sat_analysis::structure::interference::InterferenceBox;
use sat_analysis::thermal::network::{
    ThermalCoating, ThermalConductiveLink, ThermalContact, ThermalMountBoxes, ThermalMounts,
    ThermalNetwork, ThermalNode, ThermalNodePart, ThermalRadiativeLink, ThermalSurface,
};
use sat_analysis::thermal::{ThermalBoxSize, ThermalEnclosure};
use sat_analysis::thermal::{
    ThermalRangeKind, ThermalRangeSide, ThermalRanges, ThermalRequiredBound, ThermalRequiredRange,
    ThermalTempRange,
};
use sat_units::typed::Qty;
use std::collections::BTreeMap;

/// 温度範囲の属性: 保管 = `temp_survival`（非動作の範囲）、動作 = `temp_op`、性能 = `temp_perf`。
const STORAGE_RANGE: &str = "temp_survival";
const OPERATING_RANGE: &str = "temp_op";
const PERFORMANCE_RANGE: &str = "temp_perf";

/// 熱ネットワーク（節点・取付の熱結合・外面）。
pub(super) fn network(
    ctx: &Context<'_>,
    s: &ThermalSettings,
    analysis: &str,
    m: &mut MissingInputs,
) -> ThermalNetwork {
    let map = thermal_nodes(ctx, s);
    for part in map.clashes() {
        m.push(
            thermal(&["nodes", part]),
            format!(
                "Node `{part}` has the name of part `{part}`, which is not one of its parts \
                 (every part not listed in [nodes] is a node named after the part)"
            ),
        );
    }
    let mounts = links(ctx, s, &map, m);
    ThermalNetwork {
        nodes: nodes(ctx, &map, analysis, m),
        mounts,
        conductive: conductive(s, m),
        radiative: radiative(s, m),
        enclosure: enclosure(ctx, s),
        surfaces: surfaces(ctx, s, &map, m),
    }
}

/// 明示の伝導（`[conductive.<節点>] <節点> = …`。優先順位と置き換えは解析が決める）。
fn conductive(s: &ThermalSettings, m: &mut MissingInputs) -> Vec<ThermalConductiveLink> {
    pairs(&s.conductive, &dims::THERMAL_CONDUCTIVE, m)
        .into_iter()
        .map(|(a, b, conductance)| ThermalConductiveLink { a, b, conductance })
        .collect()
}

/// 明示の放射（`[radiative.<節点>] <節点> = …`）。
fn radiative(s: &ThermalSettings, m: &mut MissingInputs) -> Vec<ThermalRadiativeLink> {
    pairs(&s.radiative, &dims::THERMAL_RADIATIVE, m)
        .into_iter()
        .map(|(a, b, exchange_area)| ThermalRadiativeLink {
            a,
            b,
            exchange_area,
        })
        .collect()
}

/// 節点対の表（節点 → 節点 → 量）を (a, b, 型付きの量) に直す（次元が合わない量は不足入力に記録して飛ばす）。
fn pairs<D: sat_units::typed::Dimension>(
    table: &BTreeMap<String, BTreeMap<String, Quantity>>,
    key: &SettingQuantity<D>,
    m: &mut MissingInputs,
) -> Vec<(String, String, Qty<D>)> {
    let mut out = Vec::new();
    for (a, row) in table {
        for (b, q) in row {
            if let Some(v) = setting_opt(key, &[a, b], Some(q), m) {
                out.push((a.clone(), b.clone(), v));
            }
        }
    }
    out
}

/// 箱型の外殻（`[enclosure] emissivity` を書いたときだけ）。箱の寸法は機体の根（取付を持たない外形のある部品）の `size`。
/// 6 面と節点の対応と、使えるかの判断は解析が行う。
fn enclosure(ctx: &Context<'_>, s: &ThermalSettings) -> Option<ThermalEnclosure> {
    let emissivity = s.enclosure.as_ref()?.emissivity?;
    let mut roots = crate::outline::candidates(ctx.resolved)
        .into_iter()
        .filter_map(|p| Some((p, ctx.vec3(p, "size")?)));
    let size = match (roots.next(), roots.next()) {
        (Some((root, size)), None) => {
            let [x, y, z] = crate::outline::body_extents(ctx.resolved, &root.id, size);
            Some(ThermalBoxSize {
                x: Qty::from_raw(x),
                y: Qty::from_raw(y),
                z: Qty::from_raw(z),
            })
        }
        _ => None,
    };
    Some(ThermalEnclosure { size, emissivity })
}

/// 節点（部品ごとの質量・比熱・温度範囲。熱容量と範囲の共通部分は解析が求める）。節点は既定で 1 部品 1 節点、
/// `[nodes]` に書いた節点は複数の部品をまとめる（`ThermalSettings::node_map`）。質量か比熱が無い部品は、
/// まとめた節点の部品と同じく不足入力にする（比熱は属性の既定値がある）。
fn nodes(
    ctx: &Context<'_>,
    map: &ThermalNodes,
    analysis: &str,
    m: &mut MissingInputs,
) -> Vec<ThermalNode> {
    map.iter()
        .map(|(id, def)| ThermalNode {
            id: id.to_owned(),
            parts: def
                .parts
                .iter()
                .filter_map(|pid| node_part(ctx, id, pid, m))
                .collect(),
            per_part: !def.lumped,
            fixed_temperature: None,
            required: required_ranges(ctx, analysis, id),
        })
        .collect()
}

/// 節点の温度の要求（`range_low:<節点>:<種類>` の `at_least` と `range_high:<節点>:<種類>` の `at_most`）。
/// 部品の範囲に重ねて厳しくするだけで、広げない（重ね方は解析が持つ）。判定のキーの書式は解析が持つ。
fn required_ranges(
    ctx: &Context<'_>,
    analysis: &str,
    node: &str,
) -> BTreeMap<String, ThermalRequiredRange> {
    let bound = |side: ThermalRangeSide, kind: ThermalRangeKind| {
        let key = sat_checkkey::thermal_range(side.key_prefix(), node, kind.key());
        ctx.criterion(analysis, &key).map(|c| ThermalRequiredBound {
            limit: Qty::from_raw(c.si),
            requirement: c.requirement,
        })
    };
    ThermalRangeKind::ALL
        .into_iter()
        .filter_map(|kind| {
            let range = ThermalRequiredRange {
                min: bound(ThermalRangeSide::Low, kind),
                max: bound(ThermalRangeSide::High, kind),
            };
            (range != ThermalRequiredRange::default()).then(|| (kind.key().to_owned(), range))
        })
        .collect()
}

/// 節点に入る部品 1 つ（1 個あたりの質量・比熱と員数、モードでの電源の状態を数える冗長の方式と語彙）。
fn node_part(
    ctx: &Context<'_>,
    node: &str,
    pid: &str,
    m: &mut MissingInputs,
) -> Option<ThermalNodePart> {
    let Some(p) = ctx.resolved.parts.get(pid) else {
        m.push(thermal(&["nodes", node]), format!("Part {pid} not found"));
        return None;
    };
    // 質量も比熱も、無ければ不足入力に記録する（片方が無くてももう片方を調べる）
    let mass = m.need(
        ctx.scalar(p, "mass"),
        Target::attr(pid, "mass"),
        "Mass (heat capacity of the thermal node)",
    );
    let c = m.need(
        ctx.scalar(p, "specific_heat"),
        Target::attr(pid, "specific_heat"),
        "Specific heat (heat capacity of the thermal node)",
    );
    Some(ThermalNodePart {
        id: pid.to_owned(),
        mass: Qty::from_raw(mass?),
        count: p.count_or_one(),
        specific_heat: Qty::from_raw(c?),
        ranges: ranges(ctx, p),
        redundancy: crate::parts::redundancy(p),
        states: p.states.clone(),
    })
}

/// 部品の 3 種の温度範囲（書かれていない種類は `None`）。
fn ranges(ctx: &Context<'_>, p: &ResolvedPart) -> ThermalRanges {
    let range = |attr: &str| match ctx.attr(p, attr) {
        Some(SiValue::Range { min, max }) => Some(ThermalTempRange {
            min: Qty::from_raw(*min),
            max: Qty::from_raw(*max),
        }),
        _ => None,
    };
    ThermalRanges {
        storage: range(STORAGE_RANGE),
        operating: range(OPERATING_RANGE),
        performance: range(PERFORMANCE_RANGE),
    }
}

/// 取付の両端を節点へ対応させ、単体コンダクタンスとともに解析へ渡す。
fn links(
    ctx: &Context<'_>,
    s: &ThermalSettings,
    node: &ThermalNodes,
    m: &mut MissingInputs,
) -> Option<ThermalMounts> {
    let g = setting(
        &dims::THERMAL_CONTACT_CONDUCTANCE,
        &[],
        s.contact_conductance.as_ref(),
        "Contact thermal conductance of the mount",
        m,
    );
    let r = setting_opt(
        &dims::THERMAL_RADIATIVE_COUPLING,
        &[],
        s.radiative_coupling.as_ref(),
        m,
    );
    let per_area = setting_opt(
        &dims::THERMAL_CONTACT_CONDUCTANCE_PER_AREA,
        &[],
        s.contact_conductance_per_area.as_ref(),
        m,
    );
    // 取付の両端の部品（取り付ける部品・取付先）と節点。両端が節点に入る取付だけ
    let rows: Vec<([&str; 2], [String; 2])> = ctx
        .resolved
        .parts
        .values()
        .filter_map(|p| {
            let on = p.mount.as_ref()?.on.as_str();
            Some((
                [p.id.as_str(), on],
                [
                    node.node_of(&p.id)?.to_owned(),
                    node.node_of(on)?.to_owned(),
                ],
            ))
        })
        .collect();
    let contact = per_area.map(|conductance_per_area| ThermalContact {
        conductance_per_area,
        mounts: mount_boxes(ctx, rows.iter().map(|(parts, _)| *parts)),
    });
    Some(ThermalMounts {
        mounts: rows.into_iter().map(|(_, nodes)| nodes).collect(),
        conductance: g?,
        exchange_area: r,
        contact,
    })
}

/// 取付ごとの両端の部品の箱（機体座標。寸法を持たない部品は `None`）。箱の置き方は `structure::boxes`、接触面積の計算は解析が行う。
fn mount_boxes<'a>(
    ctx: &Context<'_>,
    mounts: impl Iterator<Item = [&'a str; 2]>,
) -> Vec<ThermalMountBoxes> {
    let all = crate::structure::boxes(ctx).boxes;
    let find = |id: &str| -> Option<InterferenceBox> {
        all.iter().find(|b| b.id == id).cloned().map(Into::into)
    };
    mounts
        .map(|parts| ThermalMountBoxes {
            parts: parts.map(str::to_owned),
            boxes: parts.map(find),
        })
        .collect()
}

/// 外面: 節点に入る（既定はどの部品も節点に入る）部品のうち、面積と光学特性（設定の表面材 `[surface.<部品>]`、無ければ部品の吸収率・放射率）を持つ部品
/// （員数を面積に掛ける）。表面材を書いた部品は部品の α/ε を使わない。
fn surfaces(
    ctx: &Context<'_>,
    s: &ThermalSettings,
    node: &ThermalNodes,
    m: &mut MissingInputs,
) -> Vec<ThermalSurface> {
    let mut out = Vec::new();
    for p in ctx.resolved.parts.values() {
        let (Some(n), Some(area)) = (node.node_of(&p.id), ctx.scalar(p, "area")) else {
            continue;
        };
        let coatings = coatings(s, &p.id, m);
        let (absorptivity, emissivity) = match &coatings {
            Some(_) => (None, None),
            // α/ε を持たない部品は外面ではない（面積を持つだけの部品、例えばパネルの面に載る太陽電池は、
            // 面を担うパネルの表面材で入熱を数える）。ここで不足入力にすると、外面でない部品まで止めてしまう
            None => match (ctx.scalar(p, "absorptivity"), ctx.scalar(p, "emissivity")) {
                (Some(a), Some(e)) => (Some(a), Some(e)),
                _ => continue,
            },
        };
        let Some(normal) = direction_body(ctx, p, "normal", "Outer-surface normal (part frame)", m)
        else {
            continue;
        };
        out.push(ThermalSurface {
            id: p.id.clone(),
            node: n.to_owned(),
            area: Qty::from_raw(area),
            count: p.count_or_one(),
            normal,
            absorptivity,
            emissivity,
            coatings: coatings.unwrap_or_default(),
        });
    }
    out
}

/// 部品の表面材（`[surface.<部品>]` の面積割合と `[coating.<名前>]` の α/ε）。書かれていなければ `None`。
/// 表面材が無い・α/ε が足りないものは不足入力に記録する。
fn coatings(s: &ThermalSettings, part: &str, m: &mut MissingInputs) -> Option<Vec<ThermalCoating>> {
    let shares = s.surface.get(part)?;
    let mut out = Vec::with_capacity(shares.len());
    for (id, f) in shares {
        let c = s.coating.get(id);
        let optics = c.and_then(|c| c.absorptivity.zip(c.emissivity));
        let Some((absorptivity, emissivity)) = optics else {
            m.push(
                thermal(&["coating", id]),
                "Surface material absorptivity and emissivity",
            );
            continue;
        };
        out.push(ThermalCoating {
            id: id.clone(),
            area_fraction: *f,
            absorptivity,
            emissivity,
        });
    }
    Some(out)
}
