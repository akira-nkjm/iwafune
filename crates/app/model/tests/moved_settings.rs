//! 基準が要求へ移った旧い設定のキー（`design::moved_settings`）: 書かれていれば `fune check` が移し先を知らせ、読まれない。

use fune_model::design::{
    AnalysisSettings, AocsSettings, CommLinkSettings, CommSettings, CommonSettings, Design,
    LaunchSettings, LinkDirection, MOVED_SETTINGS, MovedSetting, OrbitDecaySettings, OrbitSettings,
    PowerSettings,
};
use fune_model::path::SettingPath;
use fune_model::value::Quantity;
use fune_model::{Diagnostic, Severity};

fn moved(design: &Design) -> Vec<Diagnostic> {
    fune_model::check(design)
        .into_iter()
        .filter(|d| d.code == "setting.moved_to_requirement")
        .collect()
}

/// 旧い基準の設定がすべて書かれた設計。
fn legacy() -> Design {
    let q = |v: f64, u: &str| Some(Quantity::new(v, u));
    let mut d = Design {
        analysis: AnalysisSettings {
            power: Some(PowerSettings {
                dod_max: Some(0.3),
                ..Default::default()
            }),
            aocs: Some(AocsSettings {
                max_slew_time: q(120.0, "s"),
                ..Default::default()
            }),
            comm: Some(CommSettings {
                min_margin: q(3.0, "dB"),
                links: [(
                    "s_down".to_owned(),
                    CommLinkSettings {
                        direction: LinkDirection::Down,
                        part: "STX".into(),
                        band: "s_band".into(),
                        antenna: None,
                        modulation: None,
                        ebn0_required: None,
                        pointing_loss: None,
                        polarization_loss: None,
                        atmospheric_loss: None,
                        implementation_loss: None,
                        line_loss: None,
                        pfd_limit: q(-135.0, "dB"),
                        pfd_reference_bandwidth: None,
                        occupied_bandwidth: None,
                    },
                )]
                .into(),
                ..Default::default()
            }),
            common: Some(CommonSettings::default()),
            ..Default::default()
        },
        ..Default::default()
    };
    d.conditions.orbit = Some(OrbitSettings {
        duration: q(1.0, "yr"),
        decay: Some(OrbitDecaySettings {
            max_lifetime: q(25.0, "yr"),
            ..Default::default()
        }),
        ..Default::default()
    });
    d.conditions.launch = Some(LaunchSettings {
        min_frequency_axial: q(100.0, "Hz"),
        min_frequency_lateral: q(50.0, "Hz"),
        ..Default::default()
    });
    d
}

/// 旧いキーは 1 つずつ error になり、診断の文が移し先の要求の書き方を言う。
#[test]
fn every_legacy_criterion_key_is_reported_with_where_it_went() {
    let found = moved(&legacy());
    let mut messages: Vec<String> = found.iter().map(|d| d.message.clone()).collect();
    messages.sort();
    for (path, hint) in [
        ("analysis.power.dod_max", "verify.check = \"dod\""),
        ("analysis.comm.min_margin", "verify.check = \"margin\""),
        (
            "analysis.comm.link.s_down.pfd_limit",
            "pfd:<link>@<station>",
        ),
        (
            "analysis.aocs.max_slew_time",
            "verify.check = \"slew_time\"",
        ),
        (
            "conditions.orbit.decay.max_lifetime",
            "verify.check = \"max_lifetime\"",
        ),
        ("conditions.orbit.duration", "input = \"mission_duration\""),
        (
            "conditions.launch.min_frequency_axial",
            "verify.check = \"axial\"",
        ),
        (
            "conditions.launch.min_frequency_lateral",
            "verify.check = \"lateral\"",
        ),
    ] {
        let hit = found
            .iter()
            .find(|d| d.message.contains(&format!("`{path}`")))
            .unwrap_or_else(|| panic!("{path} is not reported: {messages:?}"));
        assert_eq!(hit.severity, Severity::Error, "{path}");
        assert!(hit.message.contains(hint), "{path}: {}", hit.message);
        assert!(hit.message.contains(&format!("fune setting unset {path}")));
    }
    assert_eq!(found.len(), 8, "{messages:?}");
}

/// 値が書かれていなければ何も言わない。
#[test]
fn nothing_is_reported_without_a_legacy_value() {
    assert!(moved(&Design::default()).is_empty());
}

/// 表の各行は 1 回だけ当たり、`*` は 1 つの区切りに当たる。
#[test]
fn the_table_matches_whole_paths() {
    let of = |p: SettingPath| MovedSetting::of(&p).is_some();
    assert!(of(SettingPath::analysis(&[
        "comm",
        "link",
        "uhf_up",
        "pfd_limit"
    ])));
    assert!(!of(SettingPath::analysis(&["comm", "link", "pfd_limit"])));
    assert!(!of(SettingPath::analysis(&[
        "comm",
        "link",
        "a",
        "b",
        "pfd_limit"
    ])));
    assert!(!of(SettingPath::analysis(&["power", "initial_soc"])));
    assert!(of(SettingPath::budget(&["mass", "limit"])));
    assert!(!of(SettingPath::budget(&["mass", "system_margin"])));
    for (i, a) in MOVED_SETTINGS.iter().enumerate() {
        for b in &MOVED_SETTINGS[i + 1..] {
            assert!(a.scope != b.scope || a.path != b.path, "{:?} twice", a.path);
        }
    }
}
