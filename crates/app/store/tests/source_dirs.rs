//! 出典をディレクトリの下に置ける（docs/model.md §11.1）。ディレクトリは置き場所だけで、ID はファイル名。

use fune_model::Design;
use fune_model::design::{CatalogLock, EntityKey, LockedCatalog};
use fune_store::FileSet;
use fune_store::layout::source_dir;
use fune_store::read::design_of;
use fune_store::write::project::catalog_lock_text;

const PROJECT: &str =
    "[project]\nname = \"t\"\nschema = 2\n\n[[catalog]]\nname = \"local\"\npath = \"catalog\"\n";
const DS: &str = "kind = \"datasheet\"\nref = \"https://example.com/rw.pdf\"\n";
const TR: &str = "kind = \"analysis\"\nlabel = \"Report\"\n";

fn design(sources: &[(&str, &str)]) -> Design {
    let mut set = FileSet::new();
    set.insert("fune.toml", PROJECT);
    for (path, text) in sources {
        set.insert(*path, *text);
    }
    design_of(&set)
}

/// 入れ子のディレクトリに分けても、平らに置いたときと同じ台帳になる（ID は変わらない）。
#[test]
fn nested_directories_read_to_the_same_sources_as_flat() {
    let flat = design(&[("sources/ds_rw.toml", DS), ("sources/tr_1.toml", TR)]);
    let nested = design(&[
        ("sources/datasheets/rw/ds_rw.toml", DS),
        ("sources/reports/tr_1.toml", TR),
    ]);
    assert!(
        nested.load_diagnostics.is_empty(),
        "{:?}",
        nested.load_diagnostics
    );
    assert_eq!(flat.sources, nested.sources);
    assert_eq!(
        nested
            .source_map
            .file_of(&EntityKey::Source("ds_rw".into())),
        Some("sources/datasheets/rw/ds_rw.toml")
    );
    assert_eq!(source_dir(&nested, "ds_rw"), ["datasheets", "rw"]);
    assert!(source_dir(&flat, "ds_rw").is_empty());
}

/// 同じ ID が 2 つのファイルにあれば、2 つ目を `id.duplicate` にする（最初のファイルが位置）。
#[test]
fn the_same_id_in_two_directories_is_an_error() {
    let d = design(&[("sources/a/ds_rw.toml", DS), ("sources/b/ds_rw.toml", DS)]);
    let dup: Vec<_> = d
        .load_diagnostics
        .iter()
        .filter(|x| x.code == "id.duplicate")
        .collect();
    assert_eq!(dup.len(), 1, "{:?}", d.load_diagnostics);
    assert_eq!(dup[0].file.as_deref(), Some("sources/b/ds_rw.toml"));
    assert_eq!(
        d.source_map.file_of(&EntityKey::Source("ds_rw".into())),
        Some("sources/a/ds_rw.toml")
    );
}

/// ディレクトリ名は ID の規則に従う。
#[test]
fn directory_names_follow_the_id_rules() {
    let d = design(&[("sources/1bad/ds_rw.toml", DS)]);
    let bad: Vec<_> = d
        .load_diagnostics
        .iter()
        .filter(|x| x.code == "id.invalid")
        .collect();
    assert_eq!(bad.len(), 1, "{:?}", d.load_diagnostics);
    assert_eq!(bad[0].file.as_deref(), Some("sources/1bad/ds_rw.toml"));
}

/// 共有カタログのキャッシュの出典もディレクトリの下を読む（ID は `<カタログ名>/<ID>`）。
#[test]
fn shared_catalog_sources_read_directories() {
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
    shared.insert("sources/x/y/ds.toml", DS);
    shared.insert("sources/tr.toml", TR);
    set.insert_shared("lab", shared);
    let d = design_of(&set);
    assert!(d.load_diagnostics.is_empty(), "{:?}", d.load_diagnostics);
    assert!(d.sources.contains_key("lab/ds") && d.sources.contains_key("lab/tr"));
    assert_eq!(source_dir(&d, "lab/ds"), ["x", "y"]);
    assert!(source_dir(&d, "lab/tr").is_empty());
}
