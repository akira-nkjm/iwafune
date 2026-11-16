//! カタログ品をディレクトリの下に置ける（docs/model.md §7.8）。ディレクトリは置き場所だけで、ID はファイル名。

use fune_model::Design;
use fune_model::design::{CatalogLock, EntityKey, LockedCatalog};
use fune_store::FileSet;
use fune_store::layout::catalog_item_dir;
use fune_store::read::design_of;
use fune_store::write::project::catalog_lock_text;

const PROJECT: &str =
    "[project]\nname = \"t\"\nschema = 2\n\n[[catalog]]\nname = \"local\"\npath = \"catalog\"\n";
const RW: &str =
    "kind = \"reaction_wheel\"\nlabel = \"RW\"\n\n[attr.mass]\nvalue = 0.15\nunit = \"kg\"\n";
const SA: &str = "kind = \"sun_sensor\"\nlabel = \"SA\"\n";

fn design(items: &[(&str, &str)]) -> Design {
    let mut set = FileSet::new();
    set.insert("fune.toml", PROJECT);
    set.insert("parts/RW_X.toml", "catalog = \"local/rw_10\"\n");
    for (path, text) in items {
        set.insert(*path, *text);
    }
    design_of(&set)
}

/// 入れ子のディレクトリに分けても、平らに置いたときと同じカタログになる（ID は変わらず、参照も同じ）。
#[test]
fn nested_directories_read_to_the_same_catalog_as_flat() {
    let flat = design(&[("catalog/rw_10.toml", RW), ("catalog/ss_1.toml", SA)]);
    let nested = design(&[
        ("catalog/vendor_a/wheels/rw_10.toml", RW),
        ("catalog/vendor_b/ss_1.toml", SA),
    ]);
    assert!(
        nested.load_diagnostics.is_empty(),
        "{:?}",
        nested.load_diagnostics
    );
    assert_eq!(flat.catalog, nested.catalog);
    assert!(nested.catalog.contains_key("local/rw_10"));
    // 位置は実際のファイル
    assert_eq!(
        nested
            .source_map
            .file_of(&EntityKey::Catalog("local/rw_10".into())),
        Some("catalog/vendor_a/wheels/rw_10.toml")
    );
    assert_eq!(
        catalog_item_dir(&nested, "local/rw_10"),
        ["vendor_a", "wheels"]
    );
    assert!(catalog_item_dir(&flat, "local/rw_10").is_empty());
}

/// 同じ ID が 2 つのファイルにあれば、2 つ目を `id.duplicate` にする（最初のファイルが位置）。
#[test]
fn the_same_id_in_two_directories_is_an_error() {
    let d = design(&[("catalog/a/rw_10.toml", RW), ("catalog/b/rw_10.toml", RW)]);
    let dup: Vec<_> = d
        .load_diagnostics
        .iter()
        .filter(|x| x.code == "id.duplicate")
        .collect();
    assert_eq!(dup.len(), 1, "{:?}", d.load_diagnostics);
    assert_eq!(dup[0].file.as_deref(), Some("catalog/b/rw_10.toml"));
    assert_eq!(
        d.source_map
            .file_of(&EntityKey::Catalog("local/rw_10".into())),
        Some("catalog/a/rw_10.toml")
    );
}

/// ディレクトリ名は ID の規則に従う。
#[test]
fn directory_names_follow_the_id_rules() {
    let d = design(&[("catalog/1bad/rw_10.toml", RW)]);
    let bad: Vec<_> = d
        .load_diagnostics
        .iter()
        .filter(|x| x.code == "id.invalid")
        .collect();
    assert_eq!(bad.len(), 1, "{:?}", d.load_diagnostics);
    assert_eq!(bad[0].file.as_deref(), Some("catalog/1bad/rw_10.toml"));
}

/// 共有カタログのキャッシュもディレクトリの下を読む。出典（`sources/`）は平ら。
#[test]
fn shared_catalog_reads_directories() {
    const COMMIT: &str = "0123456789abcdef0123456789abcdef01234567";
    let mut set = FileSet::new();
    set.insert(
        "fune.toml",
        format!("{PROJECT}\n[[catalog]]\nname = \"lab\"\ngit = \"https://example.com/c.git\"\n"),
    );
    let lock = CatalogLock {
        catalogs: vec![LockedCatalog {
            name: "lab".into(),
            git: "https://example.com/c.git".into(),
            rev: None,
            commit: COMMIT.into(),
        }],
    };
    set.insert("catalog.lock", catalog_lock_text(&lock).unwrap());
    let mut shared = FileSet::new();
    shared.insert("catalog/wheels/rw.toml", RW);
    shared.insert("catalog/x/y/ss.toml", SA);
    set.insert_shared("lab", shared);
    let d = design_of(&set);
    assert!(d.load_diagnostics.is_empty(), "{:?}", d.load_diagnostics);
    assert!(d.catalog.contains_key("lab/rw") && d.catalog.contains_key("lab/ss"));
    assert_eq!(catalog_item_dir(&d, "lab/ss"), ["x", "y"]);
}
