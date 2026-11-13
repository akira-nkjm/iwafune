//! 不足入力と設定次元の検証。物理の比較は sat-analysis のテストに置く。
use fune_model::design::{EclipseSeasonSettings, OrbitSettings};
use fune_model::value::Quantity;
use fune_model::{Design, Severity};
use fune_projection::{Context, orbit::eclipse_season};

fn design() -> Design {
    let mut d = Design::default();
    d.conditions.orbit = Some(OrbitSettings {
        epoch: Some("2027-04-01T00:00:00Z".into()),
        altitude: Some(Quantity::new(550.0, "km")),
        sun_synchronous: Some(true),
        ltdn: Some("10:30".into()),
        ..Default::default()
    });
    d
}

#[test]
fn missing_duration_and_optional_step() {
    let mut d = design();
    let r = fune_model::resolve(&d);
    let ctx = Context::new(&d, &r, None).unwrap();
    assert!(
        eclipse_season(&ctx)
            .unwrap_err()
            .into_vec()
            .iter()
            .any(|m| m.path == "conditions.orbit.duration")
    );
    d.conditions.orbit.as_mut().unwrap().duration = Some(Quantity::new(1.0, "yr"));
    let r = fune_model::resolve(&d);
    let input = eclipse_season(&Context::new(&d, &r, None).unwrap()).unwrap();
    assert!(input.step.is_none());
    assert!((input.duration.value() - 365.25 * 86400.0).abs() < 1.0);
}

#[test]
fn wrong_dimensions_are_design_errors() {
    for field in ["duration", "step"] {
        let mut d = design();
        let o = d.conditions.orbit.as_mut().unwrap();
        o.duration = Some(Quantity::new(
            1.0,
            if field == "duration" { "W" } else { "yr" },
        ));
        o.eclipse_season = Some(EclipseSeasonSettings {
            step: Some(Quantity::new(
                1.0,
                if field == "step" { "km" } else { "day" },
            )),
        });
        assert!(
            fune_model::check(&d)
                .iter()
                .any(|e| e.code == "unit.dimension" && e.severity == Severity::Error)
        );
        let r = fune_model::resolve(&d);
        assert!(eclipse_season(&Context::new(&d, &r, None).unwrap()).is_err());
    }
}
