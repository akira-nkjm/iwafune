//! カタログ品のタグの検査（読み直せない文字列と重複）。
use fune_model::design::CatalogItem;
use fune_model::{Design, Severity};

fn codes(tags: &[&str]) -> Vec<String> {
    let item: CatalogItem = serde_json::from_value(serde_json::json!({
        "id": "local/rw", "kind": "reaction_wheel", "label": null, "manufacturer": null,
        "part_number": null, "maturity": null, "tags": tags, "datasheet": null, "source": null,
        "attr": {}, "ports": {}
    }))
    .unwrap();
    let mut design = Design::default();
    design.catalog.insert(item.id.clone(), item);
    fune_model::check(&design)
        .into_iter()
        .filter(|d| d.severity == Severity::Error && d.code.starts_with("catalog.tag"))
        .map(|d| d.code.to_string())
        .collect()
}

#[test]
fn plain_distinct_tags_pass() {
    assert!(codes(&["flight_proven", "国産"]).is_empty());
}

#[test]
fn tags_that_the_list_text_cannot_read_back_are_errors() {
    for bad in ["", " ", " a", "a,b"] {
        assert_eq!(codes(&[bad]), ["catalog.tag_text"], "{bad:?}");
    }
}

#[test]
fn a_repeated_tag_is_reported_once_per_repeat() {
    assert_eq!(codes(&["a", "b", "a"]), ["catalog.tag_duplicate"]);
}
