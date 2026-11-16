//! 共有カタログ（`[[catalog]] git = …`）の読み込み: lock の commit のキャッシュの品と出典を出所付きで読み、
//! lock・キャッシュの欠けと食い違いを診断にする（docs/model.md §12）。取得（git）は fune-service のテストが確かめる。

use fune_model::design::{CatalogLock, EntityKey, LockedCatalog, SharedCatalogState};
use fune_model::resolve::{Evidence, Origin};
use fune_model::{Design, Diagnostic, check, resolve};
use fune_store::FileSet;
use fune_store::read::{design_of, parse_lock};
use fune_store::write::project::catalog_lock_text;

const COMMIT: &str = "0123456789abcdef0123456789abcdef01234567";
const GIT: &str = "https://example.com/catalog.git";

fn project(rev: &str) -> String {
    format!(
        "[project]\nname = \"t\"\nschema = 2\n\n[[catalog]]\nname = \"lab\"\ngit = \"{GIT}\"\nrev = \"{rev}\"\n\n[[catalog]]\nname = \"local\"\npath = \"catalog\"\n"
    )
}

fn lock(rev: &str) -> CatalogLock {
    CatalogLock {
        catalogs: vec![LockedCatalog {
            name: "lab".into(),
            git: GIT.into(),
            rev: Some(rev.into()),
            commit: COMMIT.into(),
        }],
    }
}

const RW: &str = "kind = \"reaction_wheel\"\nlabel = \"RW\"\nsource = \"ds_rw\"\ndatasheet = \"ds_rw\"\n\n[attr.mass]\nvalue       = 0.15\nunit        = \"kg\"\nfrom.source = \"ds_rw\"\nfrom.at     = \"p.2\"\n";
const OLD_DATASHEET: &str =
    "kind = \"reaction_wheel\"\ndatasheet = \"https://example.com/old.pdf\"\n";
const DS: &str = "kind = \"datasheet\"\nref = \"https://example.com/rw.pdf\"\n";

/// fune.toml・lock・キャッシュ（`cached` が false なら無い）・部品から設計を作る。
fn design(rev: &str, with_lock: Option<&CatalogLock>, cached: bool, ds: &str) -> Design {
    let mut set = FileSet::new();
    set.insert("fune.toml", project(rev));
    set.insert("sources/ds_rw.toml", DS);
    set.insert(
        "parts/RW_X.toml",
        "catalog = \"lab/rw\"\n\n[attr.power]\non = { value = 2.0, unit = \"W\", from = { source = \"lab/ds_rw\" } }\n",
    );
    if let Some(l) = with_lock {
        set.insert("catalog.lock", catalog_lock_text(l).unwrap());
    }
    if cached {
        let mut shared = FileSet::new();
        shared.insert("catalog/rw.toml", RW);
        shared.insert("catalog/old.toml", OLD_DATASHEET);
        shared.insert("sources/ds_rw.toml", ds);
        set.insert_shared("lab", shared);
    }
    design_of(&set)
}

fn errors<'a>(d: &'a [Diagnostic], prefix: &str) -> Vec<&'a Diagnostic> {
    d.iter()
        .filter(|x| x.is_error() && x.code.starts_with(prefix))
        .collect()
}

/// 品と出典は `lab/<ID>` で入り、品の中の出典の参照にも出所が付く（プロジェクトの `ds_rw` とは別の出典）。
#[test]
fn cached_items_and_sources_are_qualified() {
    let d = design("v1", Some(&lock("v1")), true, DS);
    assert!(d.load_diagnostics.is_empty(), "{:?}", d.load_diagnostics);
    let item = &d.catalog["lab/rw"];
    assert_eq!(item.source.as_deref(), Some("lab/ds_rw"));
    assert_eq!(item.datasheet.as_deref(), Some("lab/ds_rw"));
    let from = item.attr["mass"].from.as_ref().unwrap();
    assert_eq!(from.source.as_deref(), Some("lab/ds_rw"));
    assert_eq!(from.at.as_deref(), Some("p.2"));
    assert!(d.sources.contains_key("lab/ds_rw") && d.sources.contains_key("ds_rw"));
    // 古い datasheet（URL を直接書く）はそのまま残り、warning になる
    assert_eq!(
        d.catalog["lab/old"].datasheet.as_deref(),
        Some("https://example.com/old.pdf")
    );
    let dir = format!(".fune/catalogs/lab/{COMMIT}");
    let loc = d
        .source_map
        .locate(&EntityKey::Catalog("lab/rw".into()), &["attr", "mass"])
        .unwrap();
    assert_eq!(loc.file, format!("{dir}/catalog/rw.toml"));
    assert!(loc.line.is_some());
    let diagnostics = check(&d);
    let codes: Vec<&str> = diagnostics
        .iter()
        .filter(|x| x.is_error())
        .map(|x| x.code.as_str())
        .collect();
    assert!(codes.is_empty(), "{diagnostics:?}");
    assert!(
        diagnostics
            .iter()
            .any(|x| x.code == "source.datasheet_location")
    );
    assert_eq!(
        d.shared_catalog_statuses()[0].state,
        SharedCatalogState::Ready
    );
}

/// 共有の品を使う部品は、品の値と共有カタログの出典の根拠で解決する。
#[test]
fn parts_resolve_values_of_shared_items() {
    let d = design("v1", Some(&lock("v1")), true, DS);
    let resolved = resolve(&d);
    let part = &resolved.parts["RW_X"];
    let mass = &part.attr["mass"];
    assert_eq!(mass.origin, Origin::Catalog);
    assert_eq!(mass.evidence, Evidence::Datasheet);
    assert_eq!(
        mass.from.as_ref().and_then(|f| f.source.as_deref()),
        Some("lab/ds_rw")
    );
    assert_eq!(part.attr["power"].origin, Origin::Override);
}

/// lock が無い・lock の rev が fune.toml と違う・キャッシュが無いときは、次にするコマンドを書いた error。
#[test]
fn missing_lock_mismatch_and_missing_cache_are_errors() {
    let cases: [(&str, Option<CatalogLock>, bool, &str, &str); 3] = [
        (
            "v1",
            None,
            false,
            "catalog.not_locked",
            "fune catalog fetch",
        ),
        (
            "v2",
            Some(lock("v1")),
            true,
            "catalog.lock_mismatch",
            "fune catalog update lab",
        ),
        (
            "v1",
            Some(lock("v1")),
            false,
            "catalog.not_fetched",
            "fune catalog fetch",
        ),
    ];
    for (rev, l, cached, code, hint) in cases {
        let d = design(rev, l.as_ref(), cached, DS);
        let found = check(&d);
        let catalog = errors(&found, "catalog.");
        assert_eq!(catalog.len(), 1, "{code}: {found:?}");
        assert_eq!(catalog[0].code, code);
        assert!(catalog[0].message.contains(hint), "{}", catalog[0].message);
        assert_eq!(catalog[0].file.as_deref(), Some("fune.toml"));
        assert!(catalog[0].line.is_some());
    }
    // lock だけにある記録は warning
    let mut extra = lock("v1");
    extra.catalogs.push(LockedCatalog {
        name: "gone".into(),
        ..lock("v1").catalogs[0].clone()
    });
    let found = check(&design("v1", Some(&extra), true, DS));
    assert!(found.iter().any(|x| x.code == "catalog.lock_unused"));
    assert!(errors(&found, "catalog.").is_empty());
}

/// 公開できない出典を使う共有の品は error（#32 の検査が出所付きの出典でも働く）。
#[test]
fn shared_items_with_restricted_sources_are_errors() {
    let restricted = format!("{DS}distribution = \"restricted\"\n");
    let found = check(&design("v1", Some(&lock("v1")), true, &restricted));
    let restricted = errors(&found, "source.restricted");
    assert_eq!(restricted.len(), 3, "{restricted:?}");
    assert!(restricted[0].message.contains("lab/ds_rw"));
}

/// lock の本文は正規形で、読み直すと同じ。記録が無ければ注記だけ。
#[test]
fn lock_text_round_trips() {
    let text = catalog_lock_text(&lock("v1")).unwrap();
    assert!(text.starts_with("# "), "{text}");
    assert!(text.contains("[[catalog]]\nname   = \"lab\"\n"), "{text}");
    assert_eq!(parse_lock(&text).unwrap(), lock("v1"));
    assert_eq!(
        fune_store::format::format_text("catalog.lock", &text).unwrap(),
        text
    );
    let empty = catalog_lock_text(&CatalogLock::default()).unwrap();
    assert_eq!(parse_lock(&empty).unwrap(), CatalogLock::default());
    assert!(!empty.contains("catalog ="), "{empty}");
    assert!(parse_lock("[[catalog]]\nname = \"x\"\n").is_err());
}

/// ディスクの走査は lock の commit のキャッシュだけを読み、キャッシュを読むと revision が変わる。
#[test]
fn scan_reads_the_cache_of_the_locked_commit() {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path();
    let write = |rel: &str, text: &str| {
        let path = root.join(rel);
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(path, text).unwrap();
    };
    write("fune.toml", &project("v1"));
    write("catalog.lock", &catalog_lock_text(&lock("v1")).unwrap());
    let before = fune_store::load(root).unwrap().design;
    assert!(before.catalog.is_empty());
    let cache = format!(".fune/catalogs/lab/{COMMIT}");
    write(&format!("{cache}/catalog/rw.toml"), RW);
    write(&format!("{cache}/sources/ds_rw.toml"), DS);
    write(&format!("{cache}/README.toml"), "x = 1\n");
    write(".fune/catalogs/lab/ffffffffffff/catalog/other.toml", RW);
    let after = fune_store::load(root).unwrap().design;
    let items: Vec<&String> = after.catalog.keys().collect();
    assert_eq!(items, ["lab/rw"]);
    assert!(after.sources.contains_key("lab/ds_rw"));
    assert_ne!(before.revision, after.revision);
    assert!(
        after.load_diagnostics.is_empty(),
        "{:?}",
        after.load_diagnostics
    );
}
