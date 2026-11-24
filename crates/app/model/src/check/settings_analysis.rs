//! `analysis/` の設定の検査（量の次元・割合の範囲・モードと部品と地上局の参照）。
//! 量の次元は系ごとのファイルの窓口が設定の次元の表で見る（[`Scope::dimensions`]）。

use super::Ctx;
use super::settings::{Scope, scope};
use crate::design::{
    AnalysisSystem, CommLinkSettings, CommSettings, DEFAULT_SEQUENCE, Design, LinkDirection,
    PowerSettings, SettingEntry,
};

/// `analysis/` の全体。`entries` は `Design::setting_entries`。
pub(super) fn check_analysis(ctx: &mut Ctx, entries: &[SettingEntry]) {
    // 書かれていない系のファイルには葉が無いので、すべての系を回してよい
    for system in AnalysisSystem::ALL {
        scope(ctx, system.setting_path()).dimensions(entries);
    }
    let design = ctx.design;
    if let Some(id) = design
        .analysis
        .common
        .as_ref()
        .and_then(|c| c.default_case.as_deref())
    {
        scope(ctx, crate::path::SettingPath::analysis_common()).reference(
            &["default_case"],
            design.cases.contains_key(id),
            "ref.case",
            format!("Analysis case `{id}`"),
        );
    }
    check_power(ctx);
    check_power_sizing(ctx);
    check_aocs(ctx);
    check_cdh(ctx);
    check_comm(ctx);
    super::settings_thermal::check_thermal(ctx);
    super::settings_structure::check_structure(ctx);
    check_propulsion(ctx);
}

fn check_propulsion(ctx: &mut Ctx) {
    let design = ctx.design;
    let Some(p) = &design.analysis.propulsion else {
        return;
    };
    let mut s = scope(ctx, AnalysisSystem::Propulsion.setting_path());
    s.fraction(&["dv_margin"], p.dv_margin);
    s.fraction(&["residual_fraction"], p.residual_fraction);
    if let Some(w) = &p.launch_window {
        for (key, steps) in [
            ("departure_steps", w.departure_steps),
            ("arrival_steps", w.arrival_steps),
        ] {
            if steps.is_some_and(|n| n < 2) {
                s.error(
                    &["launch_window", key],
                    "setting.range",
                    "The number of launch window grid points per axis must be at least 2".into(),
                );
            }
        }
    }
}

fn mode_exists(design: &Design, mode: &str) -> bool {
    design.modes.contains_key(mode)
}

fn check_power_sizing(ctx: &mut Ctx) {
    let Some(p) = ctx
        .design
        .analysis
        .power
        .as_ref()
        .and_then(|p| p.sizing.as_ref())
    else {
        return;
    };
    let mut s = scope(ctx, AnalysisSystem::Power.setting_path());
    for (key, value) in [
        ("sunlit_efficiency", p.sunlit_efficiency),
        ("eclipse_efficiency", p.eclipse_efficiency),
        ("battery_efficiency", p.battery_efficiency),
        ("annual_degradation", p.annual_degradation),
        ("inherent_degradation", p.inherent_degradation),
    ] {
        s.fraction(&["sizing", key], value);
    }
    if p.battery_strings == Some(0) {
        s.error(
            &["sizing", "battery_strings"],
            "setting.range",
            "The number of battery strings must be positive".into(),
        );
    }
}

fn check_power(ctx: &mut Ctx) {
    let design = ctx.design;
    let Some(p) = &design.analysis.power else {
        return;
    };
    let mut s = scope(ctx, AnalysisSystem::Power.setting_path());
    s.fraction(&["dod_max"], p.dod_max);
    s.fraction(&["initial_soc"], p.initial_soc);
    s.fraction(&["conversion_efficiency"], p.conversion_efficiency);
    for (mode, share) in &p.mode_share {
        let keys = ["mode_share", mode.as_str()];
        s.reference(
            &keys,
            mode_exists(design, mode),
            "ref.mode",
            format!("Mode `{mode}`"),
        );
        s.fraction(&keys, Some(*share));
    }
    let total: f64 = p.mode_share.values().sum();
    // 解析（cdh.data_budget）と同じ規則: 1 を超えればエラー、1 未満なら足りない時間はデータを出さないとみなして警告
    if !p.mode_share.is_empty() && total > 1.0 + super::SHARE_TOLERANCE {
        s.error(
            &["mode_share"],
            "setting.share_sum",
            format!("The mode shares add up to more than 1 ({total})"),
        );
    } else if !p.mode_share.is_empty() && total < 1.0 - super::SHARE_TOLERANCE {
        s.warning(
            &["mode_share"],
            "setting.share_sum",
            format!("The mode shares add up to less than 1 ({total}). The missing time is treated as no data output"),
        );
    }
    for seg in &p.sequence {
        let exists = mode_exists(design, &seg.mode);
        s.reference(
            &["sequence"],
            exists,
            "ref.mode",
            format!("Mode `{}`", seg.mode),
        );
    }
    if let Some(mode) = &p.base_mode {
        s.reference(
            &["base_mode"],
            mode_exists(design, mode),
            "ref.mode",
            format!("Mode `{mode}`"),
        );
    }
    check_sequences(&mut s, design, p);
}

/// 名前付きのモードの並び: 区間・モードの参照・従来の `[[sequence]]` との重なり・使う並びの存在。
fn check_sequences(s: &mut Scope<'_, '_>, design: &Design, p: &PowerSettings) {
    if !p.sequence.is_empty() && p.sequences.contains_key(DEFAULT_SEQUENCE) {
        s.error(
            &["sequence"],
            "setting.sequence_conflict",
            format!(
                "`[[sequence]]` and `[sequences.{DEFAULT_SEQUENCE}]` are both written. `[[sequence]]` is the sequence named `{DEFAULT_SEQUENCE}`; keep only one"
            ),
        );
    }
    for (name, seq) in &p.sequences {
        if seq.segments.is_empty() {
            s.error(
                &["sequences", name.as_str(), "segments"],
                "setting.sequence_empty",
                format!("Sequence `{name}` has no segments"),
            );
        }
        for seg in &seq.segments {
            s.reference(
                &["sequences", name.as_str(), "segments"],
                mode_exists(design, &seg.mode),
                "ref.mode",
                format!("Mode `{}`", seg.mode),
            );
        }
    }
    if let Some(active) = &p.active_sequence {
        s.reference(
            &["active_sequence"],
            p.sequence(active).is_some(),
            "ref.sequence",
            format!("Sequence `{active}`"),
        );
    }
}

fn check_aocs(ctx: &mut Ctx) {
    let design = ctx.design;
    let Some(a) = &design.analysis.aocs else {
        return;
    };
    let mut s = scope(ctx, AnalysisSystem::Aocs.setting_path());
    s.fraction(&["usable_momentum_fraction"], a.usable_momentum_fraction);
    s.fraction(&["unload_dipole_fraction"], a.unload_dipole_fraction);
    for mode in a.law.keys() {
        let exists = mode_exists(design, mode);
        s.reference(&["law", mode], exists, "ref.mode", format!("Mode `{mode}`"));
    }
}

fn check_cdh(ctx: &mut Ctx) {
    let design = ctx.design;
    let Some(c) = &design.analysis.cdh else {
        return;
    };
    let mut s = scope(ctx, AnalysisSystem::Cdh.setting_path());
    s.fraction(&["storage_margin"], c.storage_margin);
    s.fraction(&["compression_ratio"], c.compression_ratio);
}

fn check_comm(ctx: &mut Ctx) {
    let design = ctx.design;
    let Some(c) = &design.analysis.comm else {
        return;
    };
    let mut s = scope(ctx, AnalysisSystem::Comm.setting_path());
    for station in &c.stations {
        let exists = design.conditions.ground.contains_key(station);
        s.reference(
            &["stations"],
            exists,
            "ref.station",
            format!("Ground station `{station}` (conditions/ground/)"),
        );
    }
    for (name, link) in &c.links {
        check_link(&mut s, design, &c.stations, name, link);
    }
    if let Some(name) = &c.data_downlink {
        check_data_downlink(&mut s, c, name);
    }
}

/// `data_downlink` は、定義した下り回線を指すこと。
fn check_data_downlink(s: &mut Scope, c: &CommSettings, name: &str) {
    match c.links.get(name) {
        None => s.error(
            &["data_downlink"],
            "ref.link",
            format!("Link `{name}` is not defined"),
        ),
        Some(l) if l.direction != LinkDirection::Down => s.error(
            &["data_downlink"],
            "setting.link_direction",
            format!("Link `{name}` is not a downlink (direction = \"down\")"),
        ),
        Some(_) => {}
    }
}

fn check_link(
    s: &mut Scope,
    design: &Design,
    stations: &[String],
    name: &str,
    link: &CommLinkSettings,
) {
    let exists = design.parts.contains_key(link.part.as_str());
    s.reference(
        &["link", name, "part"],
        exists,
        "ref.part",
        format!("Part `{}`", link.part),
    );
    if let Some(antenna) = &link.antenna {
        s.reference(
            &["link", name, "antenna"],
            design.parts.contains_key(antenna.as_str()),
            "ref.part",
            format!("Part `{antenna}`"),
        );
    }
    for station in stations {
        let Some(g) = design.conditions.ground.get(station) else {
            continue;
        };
        if !g.links.contains_key(&link.band) {
            s.warning(
                &["link", name, "band"],
                "ref.band",
                format!(
                    "Ground station `{station}` has no performance for band `{}`",
                    link.band
                ),
            );
        }
    }
}
