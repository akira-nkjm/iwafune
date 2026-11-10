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
    use sat_analysis::attitude::disturbance::DisturbanceKind;
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
