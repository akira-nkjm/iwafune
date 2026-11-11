//! `layout` のテスト。

use super::*;

#[test]
fn classifies_files() {
    let cat = vec![("local".to_string(), "catalog".to_string())];
    assert_eq!(
        classify("parts/POWER/SAP_PY/CELLS.toml", &cat),
        FileRole::Part {
            dirs: vec!["POWER".into(), "SAP_PY".into()],
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
        classify("mission/ground/TOKYO_GS.toml", &cat),
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
        setting_location(&p("mission.ground.TOKYO_GS.min_elevation")).ok(),
        Some((
            "mission/ground/TOKYO_GS.toml".into(),
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
}
