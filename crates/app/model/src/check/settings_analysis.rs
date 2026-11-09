//! `analysis/` の設定の検査（量の次元・割合の範囲・モードと部品と地上局の参照）。

use super::Ctx;
use super::settings::{Scope, scope};
use crate::design::{CommLinkSettings, Design, MaterialSettings, ThermalSettings};
use crate::path::SettingPath;

/// `analysis/` の全体。
pub(super) fn check_analysis(ctx: &mut Ctx) {
    check_power(ctx);
    check_attitude(ctx);
    check_cdh(ctx);
    check_comm(ctx);
    check_thermal(ctx);
    check_structure(ctx);
}

fn mode_exists(design: &Design, mode: &str) -> bool {
    design.modes.contains_key(mode)
}

fn check_power(ctx: &mut Ctx) {
    let design = ctx.design;
    let Some(p) = &design.analysis.power else {
        return;
    };
    let mut s = scope(ctx, SettingPath::analysis(&["power"]));
    s.fraction(&["dod_max"], p.dod_max);
    s.fraction(&["initial_soc"], p.initial_soc);
    s.fraction(&["conversion_efficiency"], p.conversion_efficiency);
    for (mode, share) in &p.mode_share {
        let keys = ["mode_share", mode.as_str()];
        s.reference(
            &keys,
            mode_exists(design, mode),
            "ref.mode",
            format!("モード `{mode}`"),
        );
        s.fraction(&keys, Some(*share));
    }
    let total: f64 = p.mode_share.values().sum();
    // 解析（cdh.data_budget）と同じ規則: 1 を超えればエラー、1 未満なら足りない時間はデータを出さないとみなして警告
    if !p.mode_share.is_empty() && total > 1.0 + super::SHARE_TOLERANCE {
        s.error(
            &["mode_share"],
            "setting.share_sum",
            format!("モードの割合の合計が 1 を超えています（{total}）"),
        );
    } else if !p.mode_share.is_empty() && total < 1.0 - super::SHARE_TOLERANCE {
        s.warning(
            &["mode_share"],
            "setting.share_sum",
            format!("モードの割合の合計が 1 未満です（{total}）。足りない時間はデータを出さないとみなします"),
        );
    }
    for seg in &p.sequence {
        let exists = mode_exists(design, &seg.mode);
        s.reference(
            &["sequence"],
            exists,
            "ref.mode",
            format!("モード `{}`", seg.mode),
        );
        s.qty(&["sequence"], Some(&seg.duration), "Time");
    }
}

fn check_attitude(ctx: &mut Ctx) {
    let design = ctx.design;
    let Some(a) = &design.analysis.attitude else {
        return;
    };
    let mut s = scope(ctx, SettingPath::analysis(&["attitude"]));
    s.vector(&["cp_offset_aero"], a.cp_offset_aero.as_ref(), "Length");
    s.vector(&["cp_offset_srp"], a.cp_offset_srp.as_ref(), "Length");
    s.vector(
        &["residual_dipole"],
        a.residual_dipole.as_ref(),
        "MagneticDipoleMoment",
    );
    s.fraction(&["usable_momentum_fraction"], a.usable_momentum_fraction);
    s.fraction(&["unload_dipole_fraction"], a.unload_dipole_fraction);
    for mode in a.law.keys() {
        let exists = mode_exists(design, mode);
        s.reference(
            &["law", mode],
            exists,
            "ref.mode",
            format!("モード `{mode}`"),
        );
    }
}

fn check_cdh(ctx: &mut Ctx) {
    let design = ctx.design;
    let Some(c) = &design.analysis.cdh else {
        return;
    };
    let mut s = scope(ctx, SettingPath::analysis(&["cdh"]));
    s.qty(&["retention"], c.retention.as_ref(), "Time");
    s.fraction(&["storage_margin"], c.storage_margin);
    s.fraction(&["compression_ratio"], c.compression_ratio);
}

fn check_comm(ctx: &mut Ctx) {
    let design = ctx.design;
    let Some(c) = &design.analysis.comm else {
        return;
    };
    let mut s = scope(ctx, SettingPath::analysis(&["comm"]));
    for station in &c.stations {
        let exists = design.mission.ground.contains_key(station);
        s.reference(
            &["stations"],
            exists,
            "ref.station",
            format!("地上局 `{station}`（mission/ground/）"),
        );
    }
    s.qty(&["min_margin"], c.min_margin.as_ref(), "Dimensionless");
    for (name, link) in &c.links {
        check_link(&mut s, design, &c.stations, name, link);
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
        format!("部品 `{}`", link.part),
    );
    for station in stations {
        let Some(g) = design.mission.ground.get(station) else {
            continue;
        };
        if !g.links.contains_key(&link.band) {
            s.warning(
                &["link", name, "band"],
                "ref.band",
                format!(
                    "地上局 `{station}` に帯域 `{}` の性能がありません",
                    link.band
                ),
            );
        }
    }
    let losses = [
        ("ebn0_required", &link.ebn0_required),
        ("pointing_loss", &link.pointing_loss),
        ("polarization_loss", &link.polarization_loss),
        ("atmospheric_loss", &link.atmospheric_loss),
        ("implementation_loss", &link.implementation_loss),
        ("line_loss", &link.line_loss),
    ];
    for (key, q) in losses {
        s.qty(&["link", name, key], q.as_ref(), "Dimensionless");
    }
}

fn check_thermal(ctx: &mut Ctx) {
    let design = ctx.design;
    let Some(t) = &design.analysis.thermal else {
        return;
    };
    let mut s = scope(ctx, SettingPath::analysis(&["thermal"]));
    s.qtys(&[
        (
            &["contact_conductance"],
            t.contact_conductance.as_ref(),
            "ThermalConductance",
        ),
        (
            &["radiative_coupling"],
            t.radiative_coupling.as_ref(),
            "Area",
        ),
        (
            &["initial_temperature"],
            t.initial_temperature.as_ref(),
            "Temperature",
        ),
        (&["time_step"], t.time_step.as_ref(), "Time"),
    ]);
    for (name, case) in &t.cases {
        let n = name.as_str();
        if let Some(mode) = &case.mode {
            let exists = mode_exists(design, mode);
            s.reference(
                &["case", n, "mode"],
                exists,
                "ref.mode",
                format!("モード `{mode}`"),
            );
        }
        s.qtys(&[
            (
                &["case", n, "solar_flux"],
                case.solar_flux.as_ref(),
                "Irradiance",
            ),
            (
                &["case", n, "earth_ir"],
                case.earth_ir.as_ref(),
                "Irradiance",
            ),
            (&["case", n, "beta"], case.beta.as_ref(), "Angle"),
        ]);
        s.fraction(&["case", n, "albedo"], case.albedo);
    }
    check_nodes(&mut s, design, t);
}

/// 節点の部品の参照と、1 つの部品が 2 つの節点に入っていないか。
fn check_nodes(s: &mut Scope, design: &Design, t: &ThermalSettings) {
    let mut owner: std::collections::BTreeMap<&str, &str> = std::collections::BTreeMap::new();
    for (node, parts) in &t.nodes {
        for part in parts {
            let exists = design.parts.contains_key(part.as_str());
            s.reference(
                &["nodes", node],
                exists,
                "ref.part",
                format!("部品 `{part}`"),
            );
            if let Some(other) = owner.insert(part.as_str(), node.as_str()) {
                s.error(
                    &["nodes", node],
                    "thermal.node_overlap",
                    format!("部品 `{part}` が節点 `{other}` と `{node}` の両方にあります"),
                );
            }
        }
    }
}

fn check_structure(ctx: &mut Ctx) {
    let design = ctx.design;
    let Some(st) = &design.analysis.structure else {
        return;
    };
    let mut s = scope(ctx, SettingPath::analysis(&["structure"]));
    for (name, m) in &st.material {
        check_material(&mut s, name, m);
    }
    s.qty(
        &["fastener_allowable"],
        st.fastener_allowable.as_ref(),
        "Force",
    );
}

fn check_material(s: &mut Scope, name: &str, m: &MaterialSettings) {
    s.qtys(&[
        (
            &["material", name, "density"],
            m.density.as_ref(),
            "MassDensity",
        ),
        (
            &["material", name, "youngs_modulus"],
            m.youngs_modulus.as_ref(),
            "Pressure",
        ),
        (
            &["material", name, "yield_strength"],
            m.yield_strength.as_ref(),
            "Pressure",
        ),
        (
            &["material", name, "ultimate_strength"],
            m.ultimate_strength.as_ref(),
            "Pressure",
        ),
        (
            &["material", name, "thermal_conductivity"],
            m.thermal_conductivity.as_ref(),
            "W/(m*K)",
        ),
    ]);
}
