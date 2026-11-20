//! 電源のモードの並び: 従来の `[[sequence]]` を読める（後方互換）・名前の一覧と使う並びの選び方・検査・モードの時刻の指定。

use fune_model::design::{
    Design, Mode, ModeWhen, PowerSettings, StartFrom, setting_dims::setting_dimension,
};
use fune_model::path::SettingPath;
use fune_model::{Severity, check};

fn power(text: &str) -> PowerSettings {
    toml::from_str(text).unwrap()
}

const LEGACY: &str = r#"
[[sequence]]
mode     = "NOMINAL"
duration = { value = 76.0, unit = "min" }

[[sequence]]
mode     = "DOWNLINK"
duration = { value = 9.6, unit = "min" }
"#;

const NAMED: &str = r#"
active_sequence = "burst"

[sequences.imaging_day]
anchor = { from = "eclipse_exit", offset = { value = 5.0, unit = "min" } }
segments = [{ mode = "NOMINAL", duration = { value = 60.0, unit = "min" } }]

[sequences.burst]
segments = [{ mode = "DOWNLINK", duration = { value = 95.0, unit = "min" } }]
"#;

fn modes(ids: &[&str]) -> Design {
    let mut d = Design::default();
    for id in ids {
        let m = Mode {
            id: (*id).into(),
            ..Default::default()
        };
        d.modes.insert(m.id.clone(), m);
    }
    d
}

fn error_codes(d: &Design) -> Vec<String> {
    check(d)
        .into_iter()
        .filter(|x| x.severity == Severity::Error)
        .map(|x| x.code)
        .collect()
}

#[test]
fn the_legacy_form_is_the_default_sequence() {
    let p = power(LEGACY);
    assert_eq!(p.sequence_names(), ["default"]);
    let v = p.active_sequence_view().unwrap();
    assert!(v.legacy && v.anchor.is_none());
    assert_eq!(v.name, "default");
    assert_eq!(v.segments.len(), 2);
    assert_eq!(v.segments[1].mode, "DOWNLINK");
}

#[test]
fn the_active_sequence_is_the_setting_then_default_then_the_first_name() {
    let p = power(NAMED);
    assert_eq!(p.sequence_names(), ["burst", "imaging_day"]);
    assert_eq!(p.active_sequence_name(), Some("burst"));
    // 設定が無ければ名前の順で最初
    let mut p2 = p.clone();
    p2.active_sequence = None;
    assert_eq!(p2.active_sequence_name(), Some("burst"));
    // default（従来の形）があればそれが先
    let mixed = power(&format!("{LEGACY}{NAMED}").replace("active_sequence = \"burst\"", ""));
    assert_eq!(mixed.sequence_names(), ["default", "burst", "imaging_day"]);
    assert_eq!(mixed.active_sequence_name(), Some("default"));
    // 存在しない名前は黙って別の並びにせず None
    let mut p3 = p;
    p3.active_sequence = Some("nope".into());
    assert_eq!(p3.active_sequence_name(), None);
    assert!(PowerSettings::default().active_sequence_name().is_none());
}

#[test]
fn the_anchor_is_read_with_its_offset() {
    let p = power(NAMED);
    let a = p.sequence("imaging_day").unwrap().anchor.unwrap();
    assert_eq!(a.from, StartFrom::EclipseExit);
    assert_eq!(a.offset.as_ref().unwrap().value, 5.0);
}

#[test]
fn sequence_checks_reference_modes_and_names() {
    let mut d = modes(&["NOMINAL", "DOWNLINK"]);
    d.analysis.power = Some(power(NAMED));
    assert!(error_codes(&d).is_empty(), "{:?}", error_codes(&d));
    // 知らないモード・存在しない使う並び・`default` の二重定義・空の並び
    let mut bad = d.clone();
    bad.analysis.power = Some(power(
        r#"active_sequence = "nope"
[sequences.a]
segments = [{ mode = "GONE", duration = { value = 1.0, unit = "min" } }]
[sequences.b]
segments = []
"#,
    ));
    let codes = error_codes(&bad);
    for expected in ["ref.mode", "ref.sequence", "setting.sequence_empty"] {
        assert!(
            codes.iter().any(|c| c == expected),
            "{expected} in {codes:?}"
        );
    }
    let mut dup = d;
    dup.analysis.power = Some(power(&format!(
        "{LEGACY}\n[sequences.default]\nsegments = [{{ mode = \"NOMINAL\", duration = {{ value = 1.0, unit = \"min\" }} }}]\n"
    )));
    assert!(error_codes(&dup).contains(&"setting.sequence_conflict".to_owned()));
}

#[test]
fn sequence_durations_and_offsets_have_a_time_dimension() {
    let seg = SettingPath::analysis(&[
        "power",
        "sequences",
        "imaging_day",
        "segments",
        "0",
        "duration",
    ]);
    let anchor = SettingPath::analysis(&["power", "sequences", "imaging_day", "anchor", "offset"]);
    let legacy = SettingPath::analysis(&["power", "sequence", "0", "duration"]);
    for p in [seg, anchor, legacy] {
        assert_eq!(setting_dimension(&p), Some("Time"), "{p}");
    }
}

#[test]
fn mode_timing_is_read_and_a_wrong_dimension_is_an_error() {
    let mode: Mode = toml::from_str(
        r#"
max_duration = { value = 10.0, unit = "min" }
when = "eclipse"
start = { from = "eclipse_entry", offset = { value = -2.0, unit = "min" } }
"#,
    )
    .unwrap();
    assert_eq!(mode.when, Some(ModeWhen::Eclipse));
    assert_eq!(mode.start.as_ref().unwrap().from, StartFrom::EclipseEntry);
    let mut d = Design::default();
    d.modes.insert(
        "M".into(),
        Mode {
            id: "M".into(),
            ..mode.clone()
        },
    );
    assert!(error_codes(&d).is_empty(), "{:?}", error_codes(&d));
    // 時刻の指定だけで最長継続時間が無ければ、効かないので警告
    let mut loose = mode.clone();
    loose.max_duration = None;
    d.modes.insert(
        "M".into(),
        Mode {
            id: "M".into(),
            ..loose
        },
    );
    assert!(
        check(&d)
            .iter()
            .any(|x| x.code == "mode.timing_unused" && x.severity == Severity::Warning)
    );
    // 次元の違うオフセットは誤り
    let mut wrong = mode;
    wrong.start.as_mut().unwrap().offset.as_mut().unwrap().unit = "kg".into();
    d.modes.insert(
        "M".into(),
        Mode {
            id: "M".into(),
            ..wrong
        },
    );
    assert!(!error_codes(&d).is_empty());
    // 綴りの違う `when` は読めない
    assert!(toml::from_str::<Mode>(r#"when = "night""#).is_err());
}
