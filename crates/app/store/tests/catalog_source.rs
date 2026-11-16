//! fune.toml の `[[catalog]]` を足す・外す書き込み（`write::project::{add_catalog_source, remove_catalog_source}`）。
//! 書いた本文は正規形（`fune fmt` で変わらない）で、ほかの行とコメントは残る。検査（名前の重複など）は fune-ops。

use fune_model::design::CatalogSource;
use fune_store::format::format_text;
use fune_store::write::project::{add_catalog_source, remove_catalog_source};
use fune_store::{Edits, FileSet};

fn files(project: &str) -> FileSet {
    let mut files = FileSet::new();
    files.insert("fune.toml", project);
    files
}

fn shared(name: &str, rev: Option<&str>) -> CatalogSource {
    CatalogSource {
        name: name.into(),
        path: None,
        git: Some("https://example.com/catalog.git".into()),
        rev: rev.map(str::to_owned),
    }
}

const BARE: &str = "# 注記\n[project]\nname   = \"t\"\nschema = 2\n";

#[test]
fn add_and_remove_without_an_existing_catalog_table() {
    let mut edits = Edits::new(&files(BARE));
    add_catalog_source(&mut edits, &shared("lab", Some("v1"))).unwrap();
    let after = edits.files().get("fune.toml").unwrap().to_owned();
    assert_eq!(
        after,
        "# 注記\n[project]\nname   = \"t\"\nschema = 2\n\n[[catalog]]\nname = \"lab\"\ngit  = \"https://example.com/catalog.git\"\nrev  = \"v1\"\n"
    );
    assert_eq!(format_text("fune.toml", &after).unwrap(), after);
    assert!(remove_catalog_source(&mut edits, "lab").unwrap());
    assert_eq!(edits.files().get("fune.toml"), Some(BARE));
    assert!(!remove_catalog_source(&mut edits, "lab").unwrap());
}

#[test]
fn remove_keeps_the_other_catalogs_and_comments() {
    let text = "[project]\nname = \"t\"\n\n# 共有カタログ\n[[catalog]]\nname = \"a\"\ngit  = \"https://example.com/a.git\"\n\n[[catalog]]\nname = \"b\"\npath = \"catalog\" # 手元\n\n[modes]\norder = [\"SAFE\"]\n";
    let mut edits = Edits::new(&files(text));
    add_catalog_source(&mut edits, &shared("c", None)).unwrap();
    let added = edits.files().get("fune.toml").unwrap().to_owned();
    assert!(
        added.contains("path = \"catalog\" # 手元\n\n[[catalog]]\nname = \"c\"\ngit  = \"https://example.com/catalog.git\"\n\n[modes]"),
        "{added}"
    );
    assert!(remove_catalog_source(&mut edits, "c").unwrap());
    assert_eq!(edits.files().get("fune.toml"), Some(text));
}
