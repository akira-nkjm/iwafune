use super::*;

#[test]
fn builders_and_readers_agree_for_every_family() {
    let cases = [
        (
            link_margin("s_down", "TOKYO_GS"),
            vec![(Link, "s_down"), (Station, "TOKYO_GS")],
        ),
        (
            link_pfd("s_down", "TOKYO_GS"),
            vec![(Link, "s_down"), (Station, "TOKYO_GS")],
        ),
        (link_closes("TOKYO_GS"), vec![(Station, "TOKYO_GS")]),
        (sensor_intrusion("STT"), vec![(Part, "STT")]),
        (mount_interface("BUS_FRAME"), vec![(Part, "BUS_FRAME")]),
        (heater("HTR_BAT"), vec![(Part, "HTR_BAT")]),
        (
            modal_frequency(family::PANEL, "PANEL_PX"),
            vec![(Part, "PANEL_PX")],
        ),
        (
            modal_frequency(family::LATERAL, "PANEL_PX"),
            vec![(Part, "PANEL_PX")],
        ),
        (
            modal_frequency(family::AXIAL, "PANEL_PX"),
            vec![(Part, "PANEL_PX")],
        ),
        (
            part_overlap("EPT", "RW_Y"),
            vec![(Part, "EPT"), (Part, "RW_Y")],
        ),
        (
            thermal_range(family::RANGE_LOW, "BUS", "operating"),
            vec![(ThermalNode, "BUS")],
        ),
        (
            thermal_range(family::RANGE_HIGH, "BUS", "operating"),
            vec![(ThermalNode, "BUS")],
        ),
    ];
    for (key, named) in cases {
        for kind in [Part, Link, Station, ThermalNode] {
            let want: Vec<&str> = named
                .iter()
                .filter(|(k, _)| *k == kind)
                .map(|(_, n)| *n)
                .collect();
            assert_eq!(names(&key, kind), want, "{key} {kind:?}");
        }
        for (kind, name) in &named {
            let renamed_key = renamed(&key, *kind, name, "NEW").unwrap();
            assert_eq!(renamed_key, key.replacen(name, "NEW", 1), "{key}");
            // 書き換えたキーは新しい名前を名指す
            assert!(names(&renamed_key, *kind).contains(&"NEW"));
        }
    }
}

#[test]
fn prefix_keys_are_read_as_far_as_they_go() {
    assert_eq!(names("margin", Link), Vec::<&str>::new());
    assert_eq!(names("margin:s_down", Link), ["s_down"]);
    assert_eq!(names("margin:s_down", Station), Vec::<&str>::new());
    assert_eq!(names("pfd:s_down@T", Station), ["T"]);
    assert_eq!(names("range_low:BUS", ThermalNode), ["BUS"]);
    assert_eq!(
        renamed("range_high:BUS", ThermalNode, "BUS", "BODY").as_deref(),
        Some("range_high:BODY")
    );
    assert_eq!(
        renamed("margin:s_down", Link, "s_down", "x").as_deref(),
        Some("margin:x")
    );
}

#[test]
fn only_whole_names_match() {
    assert!(renamed("margin:s_down@tokyo2", Station, "tokyo", "x").is_none());
    assert!(renamed("margin:s_down@xtokyo", Station, "tokyo", "x").is_none());
    assert!(renamed("margin:s_down@tokyo", Link, "tokyo", "x").is_none());
    assert!(renamed("margin:s_down@tokyo", Link, "s", "x").is_none());
    assert_eq!(
        renamed("margin:s_down@tokyo", Station, "tokyo", "t2").as_deref(),
        Some("margin:s_down@t2")
    );
    // 族が違えば名指さない
    assert!(names("dod", Part).is_empty());
    assert!(names("frequency:lateral", Part).is_empty());
    assert!(renamed("energy_balance", Part, "a", "b").is_none());
}

#[test]
fn a_name_may_repeat_in_two_slots() {
    // 回線と局が同じ名前でも、種類で見分けて、その種類の欄だけを替える
    assert_eq!(
        renamed("margin:a@a", Link, "a", "b").as_deref(),
        Some("margin:b@a")
    );
    assert_eq!(
        renamed("margin:a@a", Station, "a", "b").as_deref(),
        Some("margin:a@b")
    );
}
