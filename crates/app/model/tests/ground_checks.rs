//! 地上局の緯度・経度・最低仰角の範囲の検査（緯度は ±90°、経度は -180〜360°、最低仰角は 0〜90°）。

use fune_model::design::GroundStationSettings;
use fune_model::value::Quantity;
use fune_model::{Design, Diagnostic, check};

fn deg(v: f64) -> Option<Quantity> {
    Some(Quantity::new(v, "deg"))
}

fn errors(g: GroundStationSettings) -> Vec<Diagnostic> {
    let mut d = Design::default();
    d.conditions.ground.insert("GS".into(), g);
    check(&d)
        .into_iter()
        .filter(|x| x.is_error() && x.code == "setting.range")
        .collect()
}

fn station(lat: f64, lon: f64, el: f64) -> GroundStationSettings {
    GroundStationSettings {
        id: "GS".into(),
        latitude: deg(lat),
        longitude: deg(lon),
        min_elevation: deg(el),
        ..GroundStationSettings::default()
    }
}

#[test]
fn valid_positions_pass_including_the_edges() {
    for (lat, lon, el) in [
        (35.7, 139.8, 10.0),
        (-90.0, -180.0, 0.0),
        (90.0, 360.0, 90.0),
        (0.0, 200.0, 5.0),
    ] {
        assert!(errors(station(lat, lon, el)).is_empty(), "{lat} {lon} {el}");
    }
}

#[test]
fn out_of_range_values_are_errors_naming_the_key() {
    let e = errors(station(95.0, 139.0, 10.0));
    assert_eq!(e.len(), 1);
    assert!(
        e[0].message.contains("Latitude must be from -90 to 90 deg"),
        "{e:?}"
    );
    assert!(!errors(station(0.0, 400.0, 10.0)).is_empty());
    assert!(!errors(station(0.0, -181.0, 10.0)).is_empty());
    assert!(!errors(station(0.0, 10.0, 95.0)).is_empty());
    assert!(!errors(station(0.0, 10.0, -1.0)).is_empty());
}

#[test]
fn radians_are_converted_before_the_range_is_checked() {
    let g = GroundStationSettings {
        latitude: Some(Quantity::new(1.7, "rad")),
        ..GroundStationSettings::default()
    };
    assert_eq!(errors(g).len(), 1);
}
