//! 熱の射影: `thermal.steady`・`thermal.transient`。節点は `analysis.thermal.nodes`（節点 → 部品）でまとめる。
//! 熱の計算（入熱・放射・解法・電力 → 発熱の変換・表面材の混ぜ方・温度範囲の判定）は sat-analysis が行う。
//! ここは部品と設定の値を集めるだけ（ネットワークは [`network`]）。既定値（プリセットの環境・解析マージン）も解析が持つ。

mod network;

use crate::context::{Context, mode_def};
use crate::missing::MissingInputs;
use crate::parts::state_values;
use crate::quantity::setting_opt;
use crate::timeline;
use fune_model::Target;
use fune_model::design::setting_dims as dims;
use fune_model::design::{AnalysisSystem, ThermalCaseSettings, ThermalPresetKind, ThermalSettings};
use fune_model::path::SettingPath;
use network::network;
use sat_analysis::thermal::network::{ThermalModeLoads, ThermalModePart};
use sat_analysis::thermal::planet::planet_environment;
use sat_analysis::thermal::steady::ThermalSteadyCase;
use sat_analysis::thermal::transient::{ThermalHeatPhase, ThermalInitial};
use sat_analysis::thermal::{
    ThermalEnvironment, ThermalExposure, ThermalMarginPolicy, ThermalPreset, ThermalSteadyInput,
    ThermalTransientInput,
};
use sat_astro::ephem::CentralBody;
use sat_units::typed::{Fraction, Power};
use std::collections::BTreeMap;

fn settings<'a>(ctx: &'a Context<'_>, m: &mut MissingInputs) -> Option<&'a ThermalSettings> {
    let s = ctx.design.analysis.thermal.as_ref();
    m.need(
        s,
        thermal(&[]),
        "Thermal analysis settings (analysis/thermal.toml)",
    )
}

/// `analysis.thermal.<…>`（`analysis/thermal.toml` のキー）。
pub(crate) fn thermal(keys: &[&str]) -> SettingPath {
    AnalysisSystem::Thermal.setting_path().child(keys)
}

/// 部品 → 節点。
pub(crate) fn node_of(s: &ThermalSettings) -> BTreeMap<&str, &str> {
    s.nodes
        .iter()
        .flat_map(|(node, parts)| parts.iter().map(move |p| (p.as_str(), node.as_str())))
        .collect()
}

/// モードでの内部発熱の源（部品の平均電力・RF 出力・heat_fraction。発熱への変換は解析の中）。
/// 熱を出す節点は `[heat_node]` に書いた節点、無ければ部品の属する節点。節点に入っていない部品も `[heat_node]` に書けば源になる。
pub fn loads(
    ctx: &Context<'_>,
    s: &ThermalSettings,
    mode: Option<&str>,
) -> Option<ThermalModeLoads> {
    let mode = mode.and_then(|id| ctx.design.modes.get(id))?;
    let mut sources = node_of(s);
    for (pid, node) in &s.heat_node {
        sources.insert(pid.as_str(), node.as_str());
    }
    let parts = sources
        .iter()
        .filter_map(|(pid, node)| {
            let p = ctx.resolved.parts.get(*pid)?;
            // 消費電力を持つ部品だけが発熱の源（構体やパネルは渡さない。電源の mode_load と同じ選び方）
            Some(ThermalModePart {
                id: p.id.clone(),
                node: node.to_string(),
                count: crate::parts::count_of(p),
                redundancy: crate::parts::redundancy(p),
                states: p.states.clone(),
                power: state_values::<Power>(ctx, p, "power")?,
                rf_output: state_values::<Power>(ctx, p, "rf_output").unwrap_or_default(),
                heat_fraction: ctx.scalar(p, "heat_fraction"),
            })
        })
        .collect();
    Some(ThermalModeLoads {
        mode: mode_def(mode),
        parts,
    })
}

/// プリセットの対応（値は解析の `thermal::preset` が持つ）。
fn preset(p: ThermalPresetKind) -> ThermalPreset {
    match p {
        ThermalPresetKind::Hot => ThermalPreset::Hot,
        ThermalPresetKind::Cold => ThermalPreset::Cold,
        ThermalPresetKind::SafeMode => ThermalPreset::SafeMode,
    }
}

/// 解析マージンの方針（`[margin]`。書かれていない値は解析の既定値）。
fn margin(s: &ThermalSettings, m: &mut MissingInputs) -> ThermalMarginPolicy {
    let Some(policy) = &s.margin else {
        return ThermalMarginPolicy::default();
    };
    ThermalMarginPolicy {
        uncertainty: setting_opt(
            &dims::THERMAL_MARGIN_UNCERTAINTY,
            &[],
            policy.uncertainty.as_ref(),
            m,
        ),
        warning_band: setting_opt(
            &dims::THERMAL_MARGIN_WARNING_BAND,
            &[],
            policy.warning_band.as_ref(),
            m,
        ),
    }
}

/// 環境条件 `id`（`[case.<id>]`）の入熱の環境。中心天体の環境の既定値を解析が持たなければ
/// （`sat_analysis::thermal::planet`）、アルベドと赤外放射を不足入力にする。太陽距離の慣例値の無い天体では、
/// プリセットの 1 AU の太陽光を使えないので太陽光の強さも求める。
fn environment(
    id: &str,
    c: &ThermalCaseSettings,
    central: Option<CentralBody>,
    m: &mut MissingInputs,
) -> ThermalEnvironment {
    if let Some(body) = central {
        let at = |key: &str| thermal(&["case", id, key]);
        if planet_environment(body).is_none() {
            if c.albedo.is_none() {
                m.push(at("albedo"), format!("Albedo of central body {body:?}"));
            }
            if c.planet_ir.is_none() {
                m.push(
                    at("planet_ir"),
                    format!("Infrared emission of central body {body:?}"),
                );
            }
        }
        if c.preset.is_some() && c.solar_flux.is_none() && body.nominal_sun_distance_m().is_none() {
            m.push(
                at("solar_flux"),
                "Solar flux (the preset value is for 1 AU)",
            );
        }
    }
    // 書かれていない値は None のまま渡す（既定値は解析が持つ）
    ThermalEnvironment {
        preset: c.preset.map(preset),
        solar_flux: setting_opt(
            &dims::THERMAL_CASE_SOLAR_FLUX,
            &[id],
            c.solar_flux.as_ref(),
            m,
        ),
        albedo: c.albedo.map(Fraction::new),
        planet_ir: setting_opt(
            &dims::THERMAL_CASE_PLANET_IR,
            &[id],
            c.planet_ir.as_ref(),
            m,
        ),
        space_temperature: None,
    }
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
        let mode = c.mode.as_deref();
        let Some(mut t) = timeline::timeline(ctx, mode, Some(1), &mut m) else {
            continue;
        };
        if c.beta.is_some() {
            t.sampling.beta = setting_opt(&dims::THERMAL_CASE_BETA, &[id], c.beta.as_ref(), &mut m);
        }
        cases.push(ThermalSteadyCase {
            id: id.clone(),
            label: c.label.clone().unwrap_or_else(|| id.clone()),
            environment: environment(id, c, Some(t.orbit().central), &mut m),
            exposure: ThermalExposure::OrbitAverage { timeline: t },
            loads: Vec::new(),
            mode_loads: loads(ctx, s, mode),
        });
    }
    if s.cases.is_empty() {
        m.push(
            thermal(&["case"]),
            "Thermal environment conditions ([case.<name>])",
        );
    }
    let margin = margin(s, &mut m);
    m.finish(|| {
        Some(ThermalSteadyInput {
            network: net,
            cases,
            margin,
        })
    })
}

/// 非定常熱の条件は明示 ID、または唯一の条件から選ぶ。
fn transient_case<'a>(
    ctx: &Context<'_>,
    s: &'a ThermalSettings,
    m: &mut MissingInputs,
) -> Option<(&'a String, &'a ThermalCaseSettings)> {
    let selected = ctx.case.and_then(|c| c.thermal_case.as_deref());
    let chosen = match selected {
        Some(id) => s.cases.get_key_value(id),
        None if s.cases.len() == 1 => s.cases.iter().next(),
        None => None,
    };
    if chosen.is_none() {
        let target = ctx.case.map_or_else(
            || thermal(&["case"]).into(),
            |c| Target::case(c.id.as_str(), Some("thermal_case")),
        );
        m.push(
            target,
            "Cannot pick the thermal environment conditions uniquely. Specify an existing thermal_case",
        );
    }
    chosen
}

/// `thermal.transient`: 明示した熱条件（省略時は唯一の条件）で `orbits` 周。
pub fn transient(ctx: &Context<'_>) -> Result<ThermalTransientInput, MissingInputs> {
    let mut m = MissingInputs::new();
    let Some(s) = settings(ctx, &mut m) else {
        return Err(m);
    };
    let net = network(ctx, s, &mut m);
    let Some((id, c)) = transient_case(ctx, s, &mut m) else {
        return Err(m);
    };
    let mode = c.mode.as_deref();
    let t = timeline::timeline(ctx, mode, s.orbits, &mut m).map(|mut t| {
        if c.beta.is_some() {
            t.sampling.beta = setting_opt(&dims::THERMAL_CASE_BETA, &[id], c.beta.as_ref(), &mut m);
        }
        if let Some(step) = setting_opt(&dims::THERMAL_TIME_STEP, &[], s.time_step.as_ref(), &mut m)
        {
            t.sampling.step = step;
        }
        t
    });
    let initial = match setting_opt(
        &dims::THERMAL_INITIAL_TEMPERATURE,
        &[],
        s.initial_temperature.as_ref(),
        &mut m,
    ) {
        Some(temperature) => ThermalInitial::Uniform { temperature },
        None => ThermalInitial::OrbitAverageSteady,
    };
    let central = t.as_ref().map(|t| t.orbit().central);
    let environment = environment(id, c, central, &mut m);
    let margin = margin(s, &mut m);
    let phases = vec![ThermalHeatPhase {
        label: mode.unwrap_or("").to_owned(),
        duration: None,
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
            margin,
        })
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 設定のプリセットの綴り（fune-model）と解析のプリセット（sat-analysis）の綴りが一致する。
    #[test]
    fn preset_spellings_match_the_analysis() {
        for &kind in ThermalPresetKind::ALL {
            let model = serde_json::to_value(kind).unwrap();
            let analysis = serde_json::to_value(preset(kind)).unwrap();
            assert_eq!(model, analysis);
        }
    }
}
