//! 部品・カタログ品・ネットのフィールド名の一覧（`PartFieldName` など。正本）と、型付きのフィールド（`PartField` など）の
//! 名前・serde の綴りが一致する。store の読み書きの一致は fune-store の `tests/fields.rs` と `read::tree` のテスト。

use fune_model::design::{
    CatalogField, CatalogFieldName, CatalogItem, Net, NetField, NetFieldName, NetKind, Part,
    PartField, PartFieldName,
};
use serde::Serialize;

/// serde の `field` の綴り（`{ "field": "label", "value": … }`）。
fn tag<T: Serialize>(field: &T) -> String {
    serde_json::to_value(field).unwrap()["field"]
        .as_str()
        .unwrap()
        .to_owned()
}

#[test]
fn part_field_names_match() {
    let part = Part::new("RW_X");
    for &key in PartFieldName::ALL {
        let field = PartField::of(&part, key);
        assert_eq!(field.key(), key);
        assert_eq!(tag(&field), key.as_str());
        assert_eq!(key.as_str().parse::<PartFieldName>(), Ok(key));
    }
    // パス記法の `RW_X.<フィールド>` はフィールドとして読む
    for name in PartFieldName::NAMES {
        let path = fune_model::path::parse(&format!("RW_X.{name}")).unwrap();
        assert!(
            matches!(path, fune_model::path::ParamPath::Field { .. }),
            "{name}"
        );
    }
    assert_eq!(
        PartFieldName::NAMES,
        [
            "label",
            "description",
            "catalog",
            "kind",
            "candidates",
            "count",
            "maturity"
        ]
    );
}

#[test]
fn catalog_field_names_match() {
    let item = CatalogItem {
        id: "local/rw".into(),
        kind: "reaction_wheel".into(),
        label: None,
        label_en: None,
        manufacturer: None,
        part_number: None,
        maturity: None,
        datasheet: None,
        source: None,
        attr: Default::default(),
        ports: Default::default(),
    };
    for &key in CatalogFieldName::ALL {
        let field = CatalogField::of(&item, key);
        assert_eq!(field.key(), key);
        assert_eq!(tag(&field), key.as_str());
    }
}

#[test]
fn net_field_names_match() {
    let net = Net::new("P5V", NetKind::Power);
    for &key in NetFieldName::ALL {
        let field = NetField::of(&net, key);
        assert_eq!(field.key(), key);
        assert_eq!(tag(&field), key.as_str());
    }
    // ネットのファイルのキー（`Net` の serde の名前）はフィールド名と同じ
    let json = serde_json::to_value(Net {
        label: Some("5 V".into()),
        source: Some(fune_model::path::PortRef::new("PCDU", "OUT1")),
        rail: Some(fune_model::value::Quantity::new(5.0, "V")),
        limit: Some(fune_model::value::Quantity::new(1.0, "A")),
        protocol: Some("CAN".into()),
        bitrate: Some(fune_model::value::Quantity::new(1.0, "Mbit/s")),
        master: Some(fune_model::path::PortRef::new("OBC", "CAN1")),
        ..net
    })
    .unwrap();
    let mut keys: Vec<&str> = json
        .as_object()
        .unwrap()
        .keys()
        .map(String::as_str)
        .filter(|k| *k != "id")
        .collect();
    keys.sort_unstable();
    let mut names = NetFieldName::NAMES.to_vec();
    names.sort_unstable();
    assert_eq!(keys, names);
}
