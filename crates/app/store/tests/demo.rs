//! fixtures/demo-6u を読み、検証と整形の往復を確かめる。

use fune_model::{Severity, check, resolve};
use fune_store::{format, load, read};
use std::path::PathBuf;

fn demo() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../../fixtures/demo-6u")
}

#[test]
fn demo_loads_without_errors_or_warnings() {
    let snap = load(&demo()).unwrap();
    let diags = check(&snap.design);
    for d in &diags {
        eprintln!(
            "{:?} {} {:?} {:?}:{:?} {}",
            d.severity,
            d.code,
            d.path(),
            d.file,
            d.line,
            d.message
        );
    }
    assert!(
        diags.iter().all(|d| d.severity != Severity::Error),
        "errors in demo"
    );
    assert!(
        diags.iter().all(|d| d.severity != Severity::Warning),
        "warnings in demo"
    );
    let design = &snap.design;
    assert!(design.parts.len() >= 30);
    assert_eq!(design.modes.len(), 4);
    assert!(design.groups.contains_key("aocs"));
    assert_eq!(
        design
            .parts
            .get("CELLS")
            .and_then(|p| p.parent.as_ref())
            .map(|n| n.id().to_string()),
        Some("SAP_PY".to_string())
    );
    let resolved = resolve(&snap.design);
    let rw = &resolved.parts["RW_X"];
    assert!((rw.scalar("mass").unwrap() - 0.152).abs() < 1e-12);
    assert!((rw.scalar("max_momentum").unwrap() - 0.010).abs() < 1e-12);
    assert!((rw.at_state("power", "peak").unwrap().value - 3.2).abs() < 1e-12);
    assert_eq!(resolved.mount_root("RW_X"), Some("BUS_FRAME"));
    assert_eq!(resolved.pose_chain("RW_X").len(), 4);
}

#[test]
fn format_round_trip_is_stable() {
    let snap = load(&demo()).unwrap();
    let mut files = snap.files.clone();
    for change in format::format_files(&snap.files) {
        if let Some(after) = change.after {
            files.insert(change.path, after);
        }
    }
    let formatted = read::design_of(&files);
    let mut a = snap.design.clone();
    let mut b = formatted.clone();
    a.revision = Default::default();
    b.revision = Default::default();
    a.source_map = Default::default();
    b.source_map = Default::default();
    assert_eq!(a, b);
    assert!(
        format::format_files(&files).is_empty(),
        "format is not idempotent"
    );
}

/// `FUNE_FMT_FIXTURE=1 cargo test -p fune-store --test demo fmt_fixture -- --ignored` でデモを正規形に書き直す。
#[test]
#[ignore]
fn fmt_fixture() {
    if std::env::var("FUNE_FMT_FIXTURE").is_err() {
        return;
    }
    let snap = load(&demo()).unwrap();
    let changes = format::format_files(&snap.files);
    for c in &changes {
        eprintln!("format: {}", c.path);
    }
    fune_store::commit(&demo(), &changes).unwrap();
}

/// 機体座標での方向（テストだけの計算。本番の合成は projection が sat-astro で行う）。
fn body_direction(resolved: &fune_model::Resolved, part: &str, attr: &str) -> [f64; 3] {
    fn rotate(q: [f64; 4], v: [f64; 3]) -> [f64; 3] {
        // v' = v + 2w (u×v) + 2 u×(u×v)
        let u = [q[1], q[2], q[3]];
        let cross = |a: [f64; 3], b: [f64; 3]| {
            [
                a[1] * b[2] - a[2] * b[1],
                a[2] * b[0] - a[0] * b[2],
                a[0] * b[1] - a[1] * b[0],
            ]
        };
        let t = cross(u, v).map(|c| 2.0 * c);
        let ut = cross(u, t);
        [0, 1, 2].map(|i| v[i] + q[0] * t[i] + ut[i])
    }
    let mut v = resolved.parts[part].vec3(attr).unwrap();
    for step in resolved.pose_chain(part).iter().rev() {
        v = rotate(step.orientation, v);
    }
    v.map(|c| (c * 1e9).round() / 1e9)
}

#[test]
fn demo_directions_in_the_body_frame() {
    let resolved = resolve(&load(&demo()).unwrap().design);
    let cases = [
        ("SP_PX", "normal", [1.0, 0.0, 0.0]),
        ("SP_MX", "normal", [-1.0, 0.0, 0.0]),
        ("SP_PY", "normal", [0.0, 1.0, 0.0]),
        ("SP_MY", "normal", [0.0, -1.0, 0.0]),
        ("CELLS", "normal", [1.0, 0.0, 0.0]),
        ("RW_X", "spin_axis", [1.0, 0.0, 0.0]),
        ("RW_Y", "spin_axis", [0.0, 1.0, 0.0]),
        ("RW_Z", "spin_axis", [0.0, 0.0, 1.0]),
        ("STT", "boresight", [0.0, -1.0, 0.0]),
        ("SS_PX", "boresight", [1.0, 0.0, 0.0]),
        ("SS_MY", "boresight", [0.0, -1.0, 0.0]),
        ("CAM", "boresight", [0.0, 0.0, 1.0]),
        ("MTQ_Y", "axis", [0.0, 1.0, 0.0]),
    ];
    for (part, attr, want) in cases {
        let got = body_direction(&resolved, part, attr);
        // ファイルの姿勢は 6 桁で書く（0.707107）ので、90° の回転では 1e-7 ほどずれる
        let ok = got.iter().zip(want).all(|(g, w)| (g - w).abs() < 1e-6);
        assert!(ok, "{part}.{attr}: {got:?} != {want:?}");
    }
}

#[test]
fn every_setting_maps_to_its_file_and_line() {
    let snap = load(&demo()).unwrap();
    let entries = snap.design.setting_entries();
    assert!(entries.len() > 100, "{}", entries.len());
    for e in entries.iter().filter(|e| e.value.is_some()) {
        let (file, _) = fune_store::layout::setting_location(&e.path)
            .unwrap_or_else(|err| panic!("{}: {err}", e.path));
        assert!(snap.files.contains(&file), "{} -> {file}", e.path);
        let loc = fune_store::layout::locate_setting(&snap.design, &e.path).unwrap();
        assert!(loc.line.is_some(), "{} has no line in {file}", e.path);
        assert_eq!(snap.design.setting(&e.path), e.value, "{}", e.path);
    }
    let p = |s: &str| match fune_model::path::parse(s) {
        Ok(fune_model::path::ParamPath::Setting(p)) => p,
        other => panic!("{other:?}"),
    };
    use fune_model::value::SettingValue;
    assert_eq!(
        snap.design.setting(&p("analysis.power.dod_max")),
        Some(SettingValue::Number(0.3))
    );
    assert_eq!(
        snap.design.setting(&p("analysis.power.sequence.3.mode")),
        Some(SettingValue::Text("DOWNLINK".into()))
    );
    assert!(
        matches!(snap.design.setting(&p("budget.mass.limit")), Some(SettingValue::Quantity(q)) if q.value == 12.0)
    );
    assert!(matches!(
        snap.design
            .setting(&p("mission.ground.TOKYO_GS.link.uhf.antenna_gain")),
        Some(SettingValue::Quantity(_))
    ));
    assert_eq!(
        snap.design.setting(&p("budget.policy.growth.new")),
        Some(SettingValue::Number(0.2))
    );
    assert_eq!(
        snap.design.setting(&p("analysis.comm.stations")),
        Some(SettingValue::List(vec!["TOKYO_GS".into()]))
    );
}
