//! `layout` のテスト。

use super::*;

#[test]
fn classifies_files() {
    let cat = vec![("local".to_string(), "catalog".to_string())];
    assert_eq!(
        classify("parts/power/SAP_PY/CELLS.toml", &cat),
        FileRole::Part {
            dirs: vec!["power".into(), "SAP_PY".into()],
            id: "CELLS".into()
        }
    );
    assert_eq!(
        classify("modes/SAFE.toml", &cat),
        FileRole::Mode("SAFE".into())
    );
    assert_eq!(
        classify("catalog/rw.toml", &cat),
        FileRole::Catalog {
            source: "local".into(),
            id: "rw".into()
        }
    );
    assert_eq!(
        classify("conditions/ground/TOKYO_GS.toml", &cat),
        FileRole::Ground("TOKYO_GS".into())
    );
    assert_eq!(
        classify("analysis/power.toml", &cat),
        FileRole::Analysis(AnalysisSystem::Power)
    );
    for s in AnalysisSystem::ALL {
        assert_eq!(classify(&analysis_file(*s), &cat), FileRole::Analysis(*s));
        let (file, _) = setting_location(&s.setting_path().child(&["x"])).unwrap();
        assert_eq!(file, analysis_file(*s));
    }
    assert_eq!(classify("analysis/other.toml", &cat), FileRole::Other);
    assert_eq!(classify("analysis/attitude.toml", &cat), FileRole::Other);
    assert_eq!(classify("README.md", &cat), FileRole::Other);
}

#[test]
fn maps_setting_paths_to_files() {
    let p = |s: &str| match fune_model::path::parse(s) {
        Ok(fune_model::path::ParamPath::Setting(p)) => p,
        other => panic!("{other:?}"),
    };
    assert_eq!(
        setting_location(&p("analysis.power.dod_max")).ok(),
        Some(("analysis/power.toml".into(), vec!["dod_max".into()]))
    );
    assert_eq!(
        setting_location(&p("conditions.ground.TOKYO_GS.min_elevation")).ok(),
        Some((
            "conditions/ground/TOKYO_GS.toml".into(),
            vec!["min_elevation".into()]
        ))
    );
    assert_eq!(
        setting_location(&p("budget.mass.limit")).ok(),
        Some((
            "budget/allocation.toml".into(),
            vec!["mass".into(), "limit".into()]
        ))
    );
    assert!(setting_location(&p("analysis.nope.x")).is_err());
    assert!(setting_location(&p("analysis.attitude.law")).is_err());
}

/// 区切りが空の設定パスは、どの範囲でも panic せずエラーにする（HTTP の入力から届きうる）。
#[test]
fn empty_setting_paths_are_errors() {
    for scope in [
        SettingScope::Conditions,
        SettingScope::Analysis,
        SettingScope::Budget,
    ] {
        let path = SettingPath {
            scope,
            segments: Vec::new(),
        };
        assert!(
            matches!(setting_location(&path), Err(StoreError::SettingPath { .. })),
            "{scope:?}"
        );
        assert!(locate_setting(&Design::default(), &path).is_none());
    }
}

/// 旧配置は予約ディレクトリとして無視し、話題の解析ファイルは認めない。
#[test]
fn conditions_replace_the_reserved_mission_directory() {
    let cat = [];
    for path in [
        "mission/orbit.toml",
        "mission/launch.toml",
        "mission/ground/GS.toml",
    ] {
        assert_eq!(classify(path, &cat), FileRole::Reserved);
    }
    assert_eq!(
        classify("conditions/orbit.toml", &cat),
        FileRole::Conditions("orbit".into())
    );
    assert_eq!(
        classify("conditions/launch.toml", &cat),
        FileRole::Conditions("launch".into())
    );
    assert_eq!(classify("analysis/orbit.toml", &cat), FileRole::Other);
    // 古い区分 `mission` は設定のパスとして読まない（部品のパスなどとして読まれる）
    assert!(!matches!(
        fune_model::path::parse("mission.orbit.altitude"),
        Ok(fune_model::path::ParamPath::Setting(_))
    ));
}
