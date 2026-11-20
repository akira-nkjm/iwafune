//! `analysis::dims` のテスト。

use super::dims_of;
use sat_units::typed::{Power, Qty, Time};
use serde::Serialize;

#[derive(Serialize)]
struct Inner {
    p: Qty<Power>,
    ratio: f64,
}

#[derive(Serialize)]
struct Out {
    items: Vec<Inner>,
    t: Option<Qty<Time>>,
    map: std::collections::BTreeMap<String, Qty<Power>>,
}

#[test]
fn records_pointer_and_dimension_of_every_qty() {
    let out = Out {
        items: vec![Inner {
            p: Qty::from_raw(1.0),
            ratio: 0.5,
        }],
        t: Some(Qty::from_raw(3.0)),
        map: [("a/b".to_owned(), Qty::from_raw(2.0))].into(),
    };
    let d = dims_of(&out);
    assert_eq!(d.get("/items/0/p").map(String::as_str), Some("Power"));
    assert_eq!(d.get("/t").map(String::as_str), Some("Time"));
    assert_eq!(d.get("/map/a~1b").map(String::as_str), Some("Power"));
    assert!(!d.contains_key("/items/0/ratio"));
}

/// 列挙の値は型名と綴りで集める（外部タグの単位の値と、内部タグの列挙のタグ）。中の量の次元も集める。
#[test]
fn records_enum_values_with_their_type() {
    use sat_analysis::aocs::disturbance::DisturbanceKind;
    use sat_analysis::comm::data_throughput::DataThroughputLinkClosure;
    #[derive(Serialize)]
    struct WithEnums {
        kinds: Vec<DisturbanceKind>,
        link: DataThroughputLinkClosure,
        name: String,
    }
    let m = super::meta_of(&WithEnums {
        kinds: vec![DisturbanceKind::Aero],
        link: DataThroughputLinkClosure::ClosesAbove {
            min_elevation: Qty::from_raw(0.2),
        },
        name: "aero".to_owned(),
    });
    let v = |p: &str| m.variants.get(p).map(|(t, s)| (*t, s.as_str()));
    assert_eq!(v("/kinds/0"), Some(("DisturbanceKind", "aero")));
    assert_eq!(
        v("/link/kind"),
        Some(("DataThroughputLinkClosure", "closes_above"))
    );
    assert_eq!(v("/name"), None);
    assert_eq!(
        m.dims.get("/link/min_elevation").map(String::as_str),
        Some("Angle")
    );
}

/// 実際の解析出力を通し、表が使う Pointer・種類・表示と SI 数値の維持を確かめる。
fn assert_torque_fields<T: Serialize>(id: &str, output: &T, pointers: &[&str]) {
    let dims = dims_of(output);
    let json = serde_json::to_value(output).unwrap();
    let (api_dims, _) = super::super::output_meta(id, &json);
    assert_eq!(api_dims, dims);
    for &pointer in pointers {
        assert_eq!(dims[pointer], "Torque", "{pointer}");
        let si = json.pointer(pointer).unwrap().as_f64().unwrap();
        let formatted = crate::units::quantity_auto(si, &dims[pointer]);
        assert!(
            formatted.display.ends_with("N·m"),
            "{pointer}: {}",
            formatted.display
        );
        assert_eq!(formatted.si, si);
    }
}

#[test]
fn rw_unloading_and_disturbance_fields_record_torque_kind() {
    use sat_analysis::aocs::rw_unloading::RwUnloadingOutput;
    let out: RwUnloadingOutput = serde_json::from_value(serde_json::json!({
        "period": 5400.0, "complete_orbits": 1, "span_fallback": null,
        "usable_momentum_fraction": 0.8,
        "disturbances": [{"key": "srp", "peak": 0.012, "mean": 0.006}],
        "peak_disturbance": 0.012,
        "secular_per_orbit_eci": {"x": 0.0, "y": 0.01, "z": 0.0},
        "secular_per_orbit": 0.01, "cyclic_amplitude": 0.001,
        "peak_momentum": 0.015, "capacity": 0.02, "unload_interval": 5400.0,
        "unloads_per_day": 16.0, "first_saturation": null, "min_torque_margin": 0.5,
        "wheels": [{"id": "RW", "peak_momentum": 0.015, "momentum_utilization": 0.75,
            "peak_torque": 0.012, "torque_utilization": 0.5}],
        "series": {"t": [], "channels": []}, "checks": []
    }))
    .unwrap();
    assert_torque_fields(
        "aocs.rw_unloading",
        &out,
        &[
            "/peak_disturbance",
            "/disturbances/0/peak",
            "/disturbances/0/mean",
            "/wheels/0/peak_torque",
        ],
    );
    assert_eq!(dims_of(&out)["/peak_momentum"], "AngularMomentum");
    assert_eq!(
        crate::units::quantity_auto(out.peak_disturbance.value(), "Torque").display,
        "12.0 mN·m"
    );
}

#[test]
fn unload_time_mtq_and_rcs_fields_record_torque_kind() {
    use sat_analysis::aocs::unload_time::UnloadTimeOutput;
    let direction = serde_json::json!({
        "label": "given", "momentum_direction": {"x": 0.0, "y": 0.0, "z": 1.0},
        "torque": 2.0, "off_axis_torque": 0.001, "net_force": 1.0,
        "time": 0.01, "propellant": 0.001, "thrusters": ["T1"]
    });
    let out: UnloadTimeOutput = serde_json::from_value(serde_json::json!({
        "momentum": 0.02, "usable_momentum_fraction": null,
        "mtq": {"mean_field": 0.00003, "min_field": 0.00003, "max_field": 0.00003,
            "effective_dipole": 0.2, "efficiency": 0.5, "torque": 0.000003,
            "time": 6666.0, "orbits": null, "assumption": "mtq_mean_field"},
        "rcs": {"directions": [direction.clone()], "worst": direction,
            "assumption": "rcs_simultaneous_firing"},
        "checks": []
    }))
    .unwrap();
    assert_torque_fields(
        "aocs.unload_time",
        &out,
        &[
            "/mtq/torque",
            "/rcs/directions/0/torque",
            "/rcs/directions/0/off_axis_torque",
            "/rcs/worst/torque",
            "/rcs/worst/off_axis_torque",
        ],
    );
    assert_eq!(dims_of(&out)["/mtq/time"], "Time");
    assert_eq!(
        crate::units::quantity_auto(out.rcs.unwrap().worst.torque.value(), "Torque").display,
        "2.00 N·m"
    );
}

#[test]
fn energy_output_fields_keep_energy_kind_and_wh_display() {
    use sat_analysis::power::energy_balance::EnergyBalanceOrbit;
    let out = EnergyBalanceOrbit {
        index: 0,
        start: Qty::from_raw(0.0),
        generation: Qty::from_raw(72000.0),
        load: Qty::from_raw(36000.0),
        margin: Qty::from_raw(36000.0),
        min_soc: sat_units::typed::Fraction::new(0.8),
        end_soc: sat_units::typed::Fraction::new(0.9),
    };
    let dims = dims_of(&out);
    let json = serde_json::to_value(&out).unwrap();
    for (pointer, display) in [
        ("/generation", "20.0 Wh"),
        ("/load", "10.0 Wh"),
        ("/margin", "10.0 Wh"),
    ] {
        assert_eq!(dims[pointer], "Energy");
        let si = json.pointer(pointer).unwrap().as_f64().unwrap();
        assert_eq!(
            crate::units::quantity_auto(si, &dims[pointer]).display,
            display
        );
    }
}

#[test]
fn static_load_moment_records_torque_kind() {
    use sat_analysis::structure::static_load::StaticLoadOutput;
    let out: StaticLoadOutput = serde_json::from_value(serde_json::json!({
        "items": [{"id": "A", "mount": "STR", "axial": 10.0, "lateral": 5.0, "resultant": 11.0}],
        "mounts": [{"mount": "STR", "carried_mass": 1.0, "axial": 10.0, "lateral": 5.0,
            "moment": 0.5, "margin_axial": null, "margin_lateral": null, "margin_moment": null,
            "min_margin": null, "verdict": "ok"}],
        "safety_factor": 1.25, "checks": []
    }))
    .unwrap();
    assert_torque_fields("structure.static_load", &out, &["/mounts/0/moment"]);
    assert_eq!(dims_of(&out)["/mounts/0/axial"], "Force");
}

/// 前に書き出した量の記録が残っていても、次の集計の欄に付かない。
#[test]
fn leftover_record_does_not_leak_into_next_output() {
    #[derive(Serialize)]
    struct Named {
        id: String,
        p: Qty<Power>,
    }
    let _ = serde_json::to_value(Qty::<Time>::from_raw(1.0)).unwrap();
    let d = dims_of(&Named {
        id: "A".into(),
        p: Qty::from_raw(1.0),
    });
    assert!(!d.contains_key("/id"));
    assert_eq!(d["/p"], "Power");
}

/// 温度差の欄（解析マージンの不確かさ・警告の幅、範囲への余裕）は温度差の種類を記録し、K で表示する
/// （温度の欄は従来どおり Temperature で °C）。5 K の警告の幅が -268 °C にならない。
#[test]
fn thermal_margin_fields_record_temperature_difference() {
    use sat_analysis::thermal::ThermalMarginPolicy;
    let used = ThermalMarginPolicy::default().used();
    let d = dims_of(&used);
    assert_eq!(d["/uncertainty"], "TemperatureDifference");
    assert_eq!(d["/warning_band"], "TemperatureDifference");
    let band = crate::units::quantity_auto(used.warning_band.value(), &d["/warning_band"]);
    assert_eq!(band.display, "5.00 K");
    // 省略できる欄（方針）は、書かれていれば種類を記録する
    let policy = ThermalMarginPolicy {
        uncertainty: Some(Qty::from_raw(2.0)),
        warning_band: None,
        heater_duty_warning: None,
    };
    let d = dims_of(&policy);
    assert_eq!(d["/uncertainty"], "TemperatureDifference");
    assert!(!d.contains_key("/warning_band"));
    // 温度そのものは Temperature のまま
    let t = dims_of(&Qty::<sat_units::typed::Temperature>::from_raw(293.15));
    assert_eq!(t[""], "Temperature");
}
