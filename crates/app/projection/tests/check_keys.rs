//! 解析が作る判定のキーと、キーの書式の正本 `sat_checkkey`（要求の `verify.check` が名指す名前の読み書き）の一致。
//! 解析のキーの組み立ても fune-model の読み書きも `sat_checkkey` を使うので、ここは「デモのどの解析のキーも、書式の表にある族か、
//! 名前を持たない族のどちらかで、読んだ名前が設計に実在する」ことを両方が見えるクレートで確かめる。
//! 名前を持つ新しい族を解析に足して書式の表に載せ忘れると、ここが落ちる。

use fune_model::design::CommonSettings;
use fune_projection::{Context, entries, project_json};
use sat_checkkey::EntityKind;
use std::collections::BTreeSet;
use std::path::Path;

/// 名前を持たない族（`frequency:<方向>`・`envelope:<軸>`）。
const NAMELESS: &[&str] = &["frequency", "envelope"];

fn demo() -> (fune_model::Design, fune_model::Resolved) {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../../fixtures/demo-6u");
    let mut design = fune_store::load(&root)
        .expect("the demo should load")
        .design;
    design.analysis.common = Some(CommonSettings {
        default_case: Some("hot_imaging".to_owned()),
    });
    let resolved = fune_model::resolve(&design);
    (design, resolved)
}

/// デモのすべての解析の判定のキー。
fn demo_keys() -> BTreeSet<String> {
    let (design, resolved) = demo();
    let ctx = Context::new(&design, &resolved, Some("hot_imaging")).unwrap();
    let mut keys = BTreeSet::new();
    for e in entries() {
        let Ok(input) = project_json(e.id, &ctx) else {
            continue;
        };
        let out = sat_analysis::catalog::run_json(e.id, input).unwrap();
        for c in out["checks"].as_array().into_iter().flatten() {
            keys.insert(c["key"].as_str().unwrap().to_owned());
        }
    }
    keys
}

#[test]
fn every_check_key_is_registered_or_nameless() {
    for key in demo_keys() {
        let Some((family, _)) = key.split_once(':') else {
            continue;
        };
        assert!(
            sat_checkkey::is_family(family) || NAMELESS.contains(&family),
            "check key `{key}` has a name-like part but `{family}` is not in sat-checkkey's table"
        );
    }
}

#[test]
fn names_read_from_demo_keys_exist_in_the_design() {
    let (design, _) = demo();
    let nodes: BTreeSet<&str> = design
        .analysis
        .thermal
        .iter()
        .flat_map(|t| t.nodes.keys().map(String::as_str))
        .collect();
    let links: BTreeSet<&str> = design
        .analysis
        .comm
        .iter()
        .flat_map(|c| c.links.keys().map(String::as_str))
        .collect();
    let exists = |kind: EntityKind, n: &str| match kind {
        // `LV` は取付先のロケット（部品ではない）
        EntityKind::Part => n == "LV" || design.parts.contains_key(n),
        EntityKind::Link => links.contains(n),
        EntityKind::Station => design.conditions.ground.contains_key(n),
        EntityKind::ThermalNode => nodes.contains(n),
    };
    let mut seen = BTreeSet::new();
    for key in demo_keys() {
        for kind in [
            EntityKind::Part,
            EntityKind::Link,
            EntityKind::Station,
            EntityKind::ThermalNode,
        ] {
            for name in sat_checkkey::names(&key, kind) {
                assert!(
                    exists(kind, name),
                    "`{key}` names {kind:?} `{name}`, not in the design"
                );
                seen.insert(key.split(':').next().unwrap().to_owned());
            }
        }
    }
    // デモが出す、名前を持つ族はすべて読めている
    for family in [
        "margin",
        "pfd",
        "intrusion",
        "interface",
        "heater",
        "range_low",
        "range_high",
    ] {
        assert!(
            seen.contains(family),
            "no demo key of `{family}` was read: {seen:?}"
        );
    }
}
