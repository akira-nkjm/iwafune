//! 熱の射影: `thermal.steady`・`thermal.transient`。節点は `analysis.thermal.nodes`（節点 → 部品）でまとめる。
//! 熱の計算（入熱・放射・解法・発熱 = 電力 × heat_fraction）は sat-analysis が行う。ここは部品の値を節点に集めるだけ。

use crate::context::{Context, mode_def};
use crate::missing::MissingInputs;
use crate::parts::direction_body;
use crate::quantity::{qty, qty_opt};
use crate::timeline;
use fune_model::Target;
use fune_model::design::{ThermalCaseSettings, ThermalSettings};
use fune_model::path::SettingPath;
use fune_model::resolve::SiValue;
use sat_analysis::thermal::network::{
    ThermalModeLoads, ThermalModePart, ThermalMounts, ThermalNetwork, ThermalNode, ThermalNodePart,
    ThermalSurface, ThermalTempRange,
};
use sat_analysis::thermal::steady::ThermalSteadyCase;
use sat_analysis::thermal::transient::{ThermalHeatPhase, ThermalInitial};
use sat_analysis::thermal::{
    ThermalEnvironment, ThermalExposure, ThermalSteadyInput, ThermalTransientInput,
};
use sat_units::typed::{Area, Irradiance, Qty, Temperature, ThermalConductance, Time};
use std::collections::BTreeMap;

fn settings<'a>(ctx: &'a Context<'_>, m: &mut MissingInputs) -> Option<&'a ThermalSettings> {
    let s = ctx.design.analysis.thermal.as_ref();
    m.need(s, thermal(&[]), "熱の解析の設定（analysis/thermal.toml）")
}

/// `analysis.thermal.<…>`（`analysis/thermal.toml` のキー）。
fn thermal(keys: &[&str]) -> SettingPath {
    SettingPath::analysis(&["thermal"]).child(keys)
}

/// 部品 → 節点。
fn node_of(s: &ThermalSettings) -> BTreeMap<&str, &str> {
    s.nodes
        .iter()
        .flat_map(|(node, parts)| parts.iter().map(move |p| (p.as_str(), node.as_str())))
        .collect()
}

/// 節点（質量の合計、熱容量の質量平均の比熱、動作温度範囲の共通部分）。
fn nodes(ctx: &Context<'_>, s: &ThermalSettings, m: &mut MissingInputs) -> Vec<ThermalNode> {
    s.nodes
        .iter()
        .map(|(id, members)| ThermalNode {
            id: id.clone(),
            parts: members
                .iter()
                .filter_map(|pid| node_part(ctx, id, pid, m))
                .collect(),
            fixed_temperature: None,
        })
        .collect()
}

/// 節点に入る部品 1 つ（1 個あたりの質量・比熱と員数。熱容量の合計と動作範囲の共通部分は解析が求める）。
fn node_part(
    ctx: &Context<'_>,
    node: &str,
    pid: &str,
    m: &mut MissingInputs,
) -> Option<ThermalNodePart> {
    let Some(p) = ctx.resolved.parts.get(pid) else {
        m.push(
            thermal(&["nodes", node]),
            format!("部品 {pid} がありません"),
        );
        return None;
    };
    let mass = p.scalar("mass")?;
    let c = m.need(
        p.scalar("specific_heat"),
        Target::attr(pid, "specific_heat"),
        "比熱（熱の節点の熱容量）",
    )?;
    let operating = match p.value("temp_op") {
        Some(SiValue::Range { min, max }) => Some(ThermalTempRange {
            min: Qty::from_raw(*min),
            max: Qty::from_raw(*max),
        }),
        _ => None,
    };
    Some(ThermalNodePart {
        id: pid.to_owned(),
        mass: Qty::from_raw(mass),
        count: crate::parts::count_of(p),
        specific_heat: Qty::from_raw(c),
        operating,
    })
}

/// 取付の両端を節点へ対応させ、単体コンダクタンスとともに解析へ渡す。
fn links(ctx: &Context<'_>, s: &ThermalSettings, m: &mut MissingInputs) -> Option<ThermalMounts> {
    let node = node_of(s);
    let g = qty::<ThermalConductance>(
        s.contact_conductance.as_ref(),
        thermal(&["contact_conductance"]),
        "取付の接触熱コンダクタンス",
        m,
    );
    let r = qty_opt::<Area>(
        s.radiative_coupling.as_ref(),
        thermal(&["radiative_coupling"]),
        m,
    );
    let mounts = ctx
        .resolved
        .parts
        .values()
        .filter_map(|p| {
            let mount = p.mount.as_ref()?;
            Some([
                node.get(p.id.as_str())?.to_string(),
                node.get(mount.on.as_str())?.to_string(),
            ])
        })
        .collect();
    Some(ThermalMounts {
        mounts,
        conductance: g?,
        exchange_area: r,
    })
}

/// 外面: 面積・法線・吸収率・放射率をすべて持つ部品（員数を面積に掛ける）。
fn surfaces(ctx: &Context<'_>, s: &ThermalSettings, m: &mut MissingInputs) -> Vec<ThermalSurface> {
    let node = node_of(s);
    let mut out = Vec::new();
    for p in ctx.resolved.parts.values() {
        let Some(n) = node.get(p.id.as_str()) else {
            continue;
        };
        let (Some(area), Some(a), Some(e)) = (
            p.scalar("area"),
            p.scalar("absorptivity"),
            p.scalar("emissivity"),
        ) else {
            continue;
        };
        let Some(normal) = direction_body(ctx, p, "normal", "外面の法線（部品座標）", m)
        else {
            continue;
        };
        out.push(ThermalSurface {
            id: p.id.clone(),
            node: (*n).to_owned(),
            area: Qty::from_raw(area),
            count: crate::parts::count_of(p),
            normal,
            absorptivity: a,
            emissivity: e,
        });
    }
    out
}

fn network(ctx: &Context<'_>, s: &ThermalSettings, m: &mut MissingInputs) -> ThermalNetwork {
    let mounts = links(ctx, s, m);
    ThermalNetwork {
        nodes: nodes(ctx, s, m),
        mounts,
        conductive: Vec::new(),
        radiative: Vec::new(),
        surfaces: surfaces(ctx, s, m),
    }
}

/// モードでの内部発熱の源（部品の平均電力と heat_fraction。掛け算は解析の中）。
fn loads(ctx: &Context<'_>, s: &ThermalSettings, mode: Option<&str>) -> Option<ThermalModeLoads> {
    let mode = mode.and_then(|id| ctx.design.modes.get(id))?;
    let parts = node_of(s)
        .iter()
        .filter_map(|(pid, node)| {
            let p = ctx.resolved.parts.get(*pid)?;
            // 消費電力を持つ部品だけが発熱の源（構体やパネルは渡さない。電源の mode_load と同じ選び方）
            let (power, by_state) = match p.value("power")? {
                SiValue::Scalar(s) => (Some(Qty::from_raw(s.value)), BTreeMap::new()),
                SiValue::ByState(map) => (
                    None,
                    map.iter()
                        .map(|(k, v)| (k.clone(), Qty::from_raw(v.value)))
                        .collect(),
                ),
                _ => return None,
            };
            Some(ThermalModePart {
                id: p.id.clone(),
                node: node.to_string(),
                count: crate::parts::count_of(p),
                power,
                by_state,
                heat_fraction: p.scalar("heat_fraction"),
            })
        })
        .collect();
    Some(ThermalModeLoads {
        mode: mode_def(mode),
        parts,
    })
}

fn environment(
    c: &ThermalCaseSettings,
    path: &SettingPath,
    m: &mut MissingInputs,
) -> ThermalEnvironment {
    let mut env = ThermalEnvironment {
        solar_flux: qty_opt::<Irradiance>(c.solar_flux.as_ref(), path.child(&["solar_flux"]), m),
        ..Default::default()
    };
    if let Some(a) = c.albedo {
        env.albedo = a;
    }
    if let Some(ir) = qty_opt::<Irradiance>(c.earth_ir.as_ref(), path.child(&["earth_ir"]), m) {
        env.earth_ir = ir;
    }
    env
}

/// `thermal.steady`: `analysis.thermal` の環境条件ごとに軌道平均の 1 ケース。
pub fn steady(ctx: &Context<'_>) -> Result<ThermalSteadyInput, MissingInputs> {
    let mut m = MissingInputs::new();
    let Some(s) = settings(ctx, &mut m) else {
        return Err(m);
    };
    let net = network(ctx, s, &mut m);
    let mut cases = Vec::new();
    for (id, c) in &s.cases {
        let path = thermal(&["case", id]);
        let mode = c.mode.as_deref();
        let Some(mut t) = timeline::timeline(ctx, mode, 1, &mut m) else {
            continue;
        };
        t.beta = qty_opt(c.beta.as_ref(), path.child(&["beta"]), &mut m);
        cases.push(ThermalSteadyCase {
            id: id.clone(),
            label: c.label.clone().unwrap_or_else(|| id.clone()),
            environment: environment(c, &path, &mut m),
            exposure: ThermalExposure::OrbitAverage { timeline: t },
            loads: Vec::new(),
            mode_loads: loads(ctx, s, mode),
        });
    }
    if s.cases.is_empty() {
        m.push(thermal(&["case"]), "熱の環境条件（[case.<名前>]）");
    }
    m.finish(|| {
        Some(ThermalSteadyInput {
            network: net,
            cases,
        })
    })
}

/// `thermal.transient`: 解析ケースの熱の条件（無ければ最初の条件）で `orbits` 周。
pub fn transient(ctx: &Context<'_>) -> Result<ThermalTransientInput, MissingInputs> {
    let mut m = MissingInputs::new();
    let Some(s) = settings(ctx, &mut m) else {
        return Err(m);
    };
    let net = network(ctx, s, &mut m);
    let chosen = ctx
        .case
        .and_then(|c| c.thermal_case.as_deref())
        .and_then(|id| s.cases.get_key_value(id));
    let Some((id, c)) = chosen.or_else(|| s.cases.iter().next()) else {
        m.push(thermal(&["case"]), "熱の環境条件（[case.<名前>]）");
        return Err(m);
    };
    let path = thermal(&["case", id]);
    let mode = c.mode.as_deref();
    let t = timeline::timeline(
        ctx,
        mode,
        s.orbits.unwrap_or(timeline::DEFAULT_ORBITS),
        &mut m,
    )
    .map(|mut t| {
        t.beta = qty_opt(c.beta.as_ref(), path.child(&["beta"]), &mut m);
        if let Some(step) = qty_opt::<Time>(s.time_step.as_ref(), thermal(&["time_step"]), &mut m) {
            t.step = step;
        }
        t
    });
    let initial = match qty_opt::<Temperature>(
        s.initial_temperature.as_ref(),
        thermal(&["initial_temperature"]),
        &mut m,
    ) {
        Some(temperature) => ThermalInitial::Uniform { temperature },
        None => ThermalInitial::OrbitAverageSteady,
    };
    let environment = environment(c, &path, &mut m);
    let phases = vec![ThermalHeatPhase {
        label: mode.unwrap_or("").to_owned(),
        duration: Qty::from_raw(sat_astro::constants::SECONDS_PER_DAY),
        loads: Vec::new(),
        mode_loads: loads(ctx, s, mode),
    }];
    m.finish(|| {
        Some(ThermalTransientInput {
            network: net,
            timeline: t?,
            environment,
            phases,
            initial,
            max_step: None,
        })
    })
}
