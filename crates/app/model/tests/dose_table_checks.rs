//! 環境の線量率の表（`dose_rate_by_shielding`。曲線の設定の値）の検査と、文字列の読み書き。引数と値の単位の次元は設定の次元の表
//! （`setting_dims::CURVES`）、形（点が増える順・値が正）は `curve.shape`。設計の軌道とフェーズの `[orbit]` で同じ検査。
use fune_model::design::{OrbitSettings, Phase};
use fune_model::value::{Curve, SettingValue, setting_from_text, setting_to_text};
use fune_model::{Design, Severity, check};

fn table(at_unit: &str, unit: &str, values: &[f64]) -> OrbitSettings {
    OrbitSettings {
        dose_rate_by_shielding: Some(Curve {
            at: (1..=values.len()).map(|i| i as f64).collect(),
            at_unit: at_unit.into(),
            values: values.to_vec(),
            unit: unit.into(),
        }),
        ..OrbitSettings::default()
    }
}

fn codes(o: OrbitSettings, in_phase: bool) -> Vec<String> {
    let mut d = Design::default();
    if in_phase {
        d.phases.insert(
            "P".into(),
            Phase {
                id: "P".into(),
                orbit: Some(o),
                ..Phase::default()
            },
        );
    } else {
        d.conditions.orbit = Some(o);
    }
    check(&d)
        .into_iter()
        .filter(|x| x.severity == Severity::Error)
        .map(|x| x.code.to_string())
        .collect()
}

#[test]
fn a_good_table_has_no_error_and_wrong_units_or_shapes_are_errors() {
    for in_phase in [false, true] {
        assert!(codes(table("mm", "Gy/yr", &[100.0, 10.0]), in_phase).is_empty());
        assert!(
            codes(table("kg", "Gy/yr", &[100.0, 10.0]), in_phase)
                .contains(&"unit.dimension".into())
        );
        assert!(
            codes(table("mm", "Gy", &[100.0, 10.0]), in_phase).contains(&"unit.dimension".into())
        );
        assert!(
            codes(table("mm", "Gy/yr", &[100.0, 0.0]), in_phase).contains(&"curve.shape".into())
        );
    }
}

#[test]
fn the_table_text_round_trips_as_a_setting() {
    let like = OrbitSettings::field_shape("dose_rate_by_shielding");
    let v = setting_from_text("0.5:400 1:120 (mm -> Gy/yr)", like.as_ref()).unwrap();
    let SettingValue::Curve(c) = &v else {
        panic!("{v:?}")
    };
    assert_eq!(
        (c.at.as_slice(), c.values.as_slice()),
        (&[0.5, 1.0][..], &[400.0, 120.0][..])
    );
    assert_eq!((c.at_unit.as_str(), c.unit.as_str()), ("mm", "Gy/yr"));
    assert_eq!(
        setting_from_text(&setting_to_text(&v), like.as_ref()).unwrap(),
        v
    );
}

#[test]
fn the_radiation_settings_are_checked() {
    use fune_model::design::{CommonSettings, RadiationSettings};
    use fune_model::value::Quantity;
    let run = |r: RadiationSettings| {
        let mut d = Design::default();
        d.analysis.common = Some(CommonSettings {
            radiation: Some(r),
            ..Default::default()
        });
        check(&d)
            .into_iter()
            .filter(|x| x.severity == Severity::Error)
            .map(|x| x.code.to_string())
            .collect::<Vec<_>>()
    };
    assert!(run(RadiationSettings::default()).is_empty());
    let good = RadiationSettings {
        default_shielding: Some(Quantity::new(3.0, "mm")),
        design_margin: Some(2.0),
    };
    assert!(run(good.clone()).is_empty());
    let bad_unit = RadiationSettings {
        default_shielding: Some(Quantity::new(3.0, "kg")),
        ..good.clone()
    };
    assert!(run(bad_unit).contains(&"unit.dimension".into()));
    let bad_margin = RadiationSettings {
        design_margin: Some(0.0),
        ..good
    };
    assert!(run(bad_margin).contains(&"setting.range".into()));
}
