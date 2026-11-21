//! 通信の設定の参照の検査: `data_downlink`（定義した下り回線）と回線の `antenna`（部品）。
use fune_model::design::{CommLinkSettings, CommSettings, LinkDirection};
use fune_model::{Design, Severity};

fn link(direction: LinkDirection) -> CommLinkSettings {
    serde_json::from_value(serde_json::json!({
        "direction": direction.as_str(), "part": "TRX", "band": "uhf"
    }))
    .unwrap()
}

fn errors(design: &Design, key: &str) -> Vec<String> {
    fune_model::check(design)
        .into_iter()
        .filter(|d| d.severity == Severity::Error && d.message.contains(key))
        .map(|d| d.code.to_string())
        .collect()
}

fn design(data_downlink: Option<&str>, antenna: Option<&str>) -> Design {
    let mut d = Design::default();
    let mut up = link(LinkDirection::Up);
    up.antenna = antenna.map(str::to_owned);
    let mut c = CommSettings::default();
    c.links.insert("down".into(), link(LinkDirection::Down));
    c.links.insert("up".into(), up);
    c.data_downlink = data_downlink.map(str::to_owned);
    d.analysis.comm = Some(c);
    d
}

#[test]
fn data_downlink_must_be_a_defined_downlink() {
    assert!(errors(&design(Some("down"), None), "`down`").is_empty());
    assert!(errors(&design(None, None), "downlink").is_empty());
    assert_eq!(errors(&design(Some("nope"), None), "`nope`"), ["ref.link"]);
    assert_eq!(
        errors(&design(Some("up"), None), "`up`"),
        ["setting.link_direction"]
    );
}

#[test]
fn link_antenna_must_be_a_defined_part() {
    assert_eq!(
        errors(&design(None, Some("GHOST")), "`GHOST`"),
        ["ref.part"]
    );
}

#[test]
fn ground_station_ids_follow_the_entity_id_rule() {
    let mut d = Design::default();
    for id in ["TOKYO_GS", "a/b", "Gs.1"] {
        d.conditions
            .ground
            .insert(id.to_owned(), Default::default());
    }
    let bad: Vec<String> = fune_model::check(&d)
        .into_iter()
        .filter(|x| x.code == "id.invalid")
        .map(|x| x.message)
        .collect();
    assert_eq!(bad.len(), 2, "{bad:?}");
    assert!(bad.iter().any(|m| m.contains("a/b")), "{bad:?}");
    let ok = Design::default();
    assert!(
        fune_model::check(&ok)
            .iter()
            .all(|x| x.code != "id.invalid")
    );
}
