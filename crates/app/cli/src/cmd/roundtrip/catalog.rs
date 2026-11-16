//! 往復の検査の見本（カタログ品の Op）。

use super::samples::{s, scalar};
use fune_service::dto::{CatalogField, Op, Provenance};
use serde_json::json;

pub(super) fn catalog_ops() -> Vec<Op> {
    let item = s("local/rw_10mNms");
    let field = |f: CatalogField| Op::SetCatalogField {
        item: item.clone(),
        field: f,
    };
    vec![
        Op::CreateCatalogItem {
            id: s("local/rw_30mNms"),
            kind: s("reaction_wheel"),
            label: Some(s("30 mNms RW")),
            label_en: Some(s("30 mNms wheel")),
            maturity: serde_json::from_value(json!("off_the_shelf")).unwrap(),
            copy_from_part: Some(s("RW_X")),
        },
        Op::SetCatalogValue {
            item: item.clone(),
            attr: s("mass"),
            state: None,
            value: scalar(0.9, "kg"),
            from: Some(Provenance {
                source: Some(s("ds_vendor")),
                at: Some(s("p.2 Table 1")),
                note: None,
            }),
        },
        Op::SetCatalogValue {
            item: item.clone(),
            attr: s("power"),
            state: Some(s("peak")),
            value: scalar(3.5, "W"),
            from: None,
        },
        Op::UnsetCatalogValue {
            item: item.clone(),
            attr: s("power"),
            state: Some(s("idle")),
        },
        Op::UnsetCatalogValue {
            item: item.clone(),
            attr: s("mass"),
            state: None,
        },
        field(CatalogField::Manufacturer(Some(s("Acme")))),
        field(CatalogField::LabelEn(None)),
        field(CatalogField::Tags(vec![s("flight_proven"), s("domestic")])),
        field(CatalogField::Tags(vec![])),
        field(CatalogField::Kind(s("reaction_wheel"))),
        field(serde_json::from_value(json!({ "field": "maturity", "value": "modified" })).unwrap()),
        Op::RemoveCatalogItem {
            id: s("local/rw_30mNms"),
        },
        Op::RenameCatalogItem {
            from: s("local/rw_10mNms"),
            to: s("local/rw_12mNms"),
        },
        Op::AddCatalogSource {
            name: s("lab"),
            git: s("https://example.com/catalog.git"),
            rev: Some(s("v1.2")),
        },
        Op::AddCatalogSource {
            name: s("team"),
            git: s("file:///srv/git/catalog.git"),
            rev: None,
        },
        Op::RemoveCatalogSource { name: s("lab") },
    ]
}
