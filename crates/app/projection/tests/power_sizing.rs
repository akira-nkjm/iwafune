//! 逆算に必要な部品・期間と、任意の設定の写し替え。
use fune_model::Design;
use fune_model::design::{AnalysisSettings, PowerSettings, PowerSizingSettings};
use fune_model::value::Quantity;
use fune_projection::{Context, power::sizing};

#[test]
fn reports_missing_parts_orbit_and_modes() {
    let d = Design {
        analysis: AnalysisSettings {
            power: Some(PowerSettings::default()),
            ..Default::default()
        },
        ..Default::default()
    };
    let r = fune_model::resolve(&d);
    let missing = sizing(&Context::new(&d, &r, None).unwrap())
        .unwrap_err()
        .into_vec();
    assert!(
        missing
            .iter()
            .any(|m| m.path == "requirement[input/mission_duration]")
    );
    assert!(missing.iter().any(|m| m.reason.contains("solar panel")));
    assert!(missing.iter().any(|m| m.reason.contains("battery")));
    assert!(missing.iter().any(|m| m.path == "modes"));
}

#[test]
fn power_settings_round_trip_and_dimension_check() {
    let mut d = Design::default();
    d.analysis.power = Some(PowerSettings {
        sizing: Some(PowerSizingSettings {
            annual_degradation: Some(0.02),
            sunlit_efficiency: Some(0.8),
            bus_voltage: Some(Quantity::new(28.0, "V")),
            sun_distance: Some(Quantity::new(1.0, "AU")),
            ..Default::default()
        }),
        ..Default::default()
    });
    let serialized = serde_json::to_value(&d.analysis).unwrap();
    let restored: AnalysisSettings = serde_json::from_value(serialized).unwrap();
    assert_eq!(restored, d.analysis);
    assert!(
        !fune_model::check(&d)
            .iter()
            .any(|e| e.code == "unit.dimension")
    );
    d.analysis
        .power
        .as_mut()
        .unwrap()
        .sizing
        .as_mut()
        .unwrap()
        .bus_voltage = Some(Quantity::new(28.0, "W"));
    assert!(
        fune_model::check(&d)
            .iter()
            .any(|e| e.code == "unit.dimension")
    );
}

#[test]
fn projects_existing_parts_counts_and_options() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../../fixtures/demo-6u");
    let mut d = fune_store::load(&root).unwrap().design;
    d.conditions.orbit.as_mut().unwrap().duration = Some(Quantity::new(1.0, "yr"));
    let settings = d.analysis.power.get_or_insert_default();
    settings.sizing = Some(PowerSizingSettings {
        annual_degradation: Some(0.02),
        bus_voltage: Some(Quantity::new(28.0, "V")),
        ..Default::default()
    });
    let r = fune_model::resolve(&d);
    let projected = sizing(&Context::new(&d, &r, None).unwrap()).unwrap();
    assert!(!projected.array.panels.is_empty());
    assert!(!projected.batteries.is_empty());
    assert_eq!(projected.loads.modes.len(), d.modes.len());
    assert_eq!(projected.options.annual_degradation, Some(0.02));
    assert_eq!(projected.options.bus_voltage.unwrap().value(), 28.0);
    // SAP と姿勢則は電力収支と同じものを渡す（入射は姿勢の時系列から解析が求める）。
    let ctx = Context::new(&d, &r, None).unwrap();
    let balance = fune_projection::power::energy_balance(&ctx).unwrap();
    assert_eq!(
        serde_json::to_value(&projected.array).unwrap(),
        serde_json::to_value(&balance.array).unwrap()
    );
    assert_eq!(
        serde_json::to_value(&projected.timeline).unwrap(),
        serde_json::to_value(&balance.timeline).unwrap()
    );
    for case in &balance.cases {
        assert_eq!(
            serde_json::to_value(projected.mode_laws.get(&case.id)).unwrap(),
            serde_json::to_value(&case.attitude).unwrap()
        );
    }
    assert_eq!(
        projected.options.sunlit_efficiency,
        d.analysis.power.as_ref().unwrap().conversion_efficiency
    );
    assert!(projected.options.eclipse_efficiency.is_none());
    assert!(projected.options.battery_efficiency.is_none());
    for panel in &projected.array.panels {
        let source = r.parts.get(panel.id.as_str()).unwrap();
        assert_eq!(
            panel.area.value(),
            fune_projection::parts::extensive(source, source.scalar("area").unwrap())
        );
        assert_eq!(
            panel.cell_efficiency,
            source.scalar("cell_efficiency").unwrap()
        );
        assert_eq!(panel.eol_factor, source.scalar("eol_factor"));
    }
}
