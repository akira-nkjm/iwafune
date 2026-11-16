//! 共有カタログの取得・読み込み・更新・診断（docs/model.md §12）。ネットワークは使わず、一時ディレクトリに作った git の
//! リポジトリ（`file://`）を共有カタログにする。デモの RW の品を共有カタログに移し、部品の値と解析が変わらないことも確かめる。

use fune_model::design::SharedCatalogState;
use fune_service::dto::{
    BudgetKind, BudgetRequest, CatalogFetchRequest, CatalogItemChangeKind, CatalogUpdateRequest,
    RunAnalysisRequest,
};
use fune_service::{ErrorKind, Workspace, analysis, budget, query, shared_catalog};
use std::path::{Path, PathBuf};
use std::process::Command;

fn fixture() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../../fixtures/demo-6u")
}

fn copy_dir(from: &Path, to: &Path) {
    std::fs::create_dir_all(to).unwrap();
    for entry in std::fs::read_dir(from).unwrap() {
        let entry = entry.unwrap();
        let target = to.join(entry.file_name());
        if entry.file_type().unwrap().is_dir() {
            copy_dir(&entry.path(), &target);
        } else {
            std::fs::copy(entry.path(), target).unwrap();
        }
    }
}

fn git(dir: &Path, args: &[&str]) -> String {
    let out = Command::new("git")
        .arg("-C")
        .arg(dir)
        .args(["-c", "user.name=test", "-c", "user.email=test@example.com"])
        .args([
            "-c",
            "init.defaultBranch=main",
            "-c",
            "commit.gpgsign=false",
        ])
        .args(args)
        .output()
        .unwrap();
    assert!(
        out.status.success(),
        "git {args:?}: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    String::from_utf8_lossy(&out.stdout).trim().to_owned()
}

/// デモの RW の品と、その出典を持つ共有カタログのリポジトリ（タグ `v1`）。
fn catalog_repo(dir: &Path) {
    std::fs::create_dir_all(dir.join("catalog")).unwrap();
    std::fs::create_dir_all(dir.join("sources")).unwrap();
    let demo = fixture();
    std::fs::copy(
        demo.join("catalog/rw_10mNms.toml"),
        dir.join("catalog/rw_10mNms.toml"),
    )
    .unwrap();
    for s in ["ds_rw10", "ds_vendor"] {
        let file = format!("sources/{s}.toml");
        std::fs::copy(demo.join(&file), dir.join(&file)).unwrap();
    }
    std::fs::write(dir.join("README.md"), "catalog\n").unwrap();
    git(dir, &["init", "-q"]);
    git(dir, &["add", "."]);
    git(dir, &["commit", "-q", "-m", "first"]);
    git(dir, &["tag", "v1"]);
}

fn url(repo: &Path) -> String {
    format!("file://{}", repo.display())
}

/// デモの写しの RW を共有カタログ `lab` の品にしたプロジェクト。
fn project(tmp: &Path, repo: &Path, rev: &str) -> Workspace {
    let root = tmp.join("sat");
    copy_dir(&fixture(), &root);
    let toml = root.join("fune.toml");
    let mut text = std::fs::read_to_string(&toml).unwrap();
    text.push_str(&format!(
        "\n[[catalog]]\nname = \"lab\"\ngit  = \"{}\"\nrev  = \"{rev}\"\n",
        url(repo)
    ));
    std::fs::write(&toml, text).unwrap();
    for rw in ["RW_X", "RW_Y", "RW_Z"] {
        let file = root.join(format!("parts/aocs/{rw}.toml"));
        let text = std::fs::read_to_string(&file).unwrap();
        std::fs::write(&file, text.replace("local/rw_10mNms", "lab/rw_10mNms")).unwrap();
    }
    Workspace::open(&root).unwrap()
}

fn codes(ws: &Workspace, prefix: &str) -> Vec<String> {
    ws.snapshot()
        .unwrap()
        .diagnostics
        .iter()
        .filter(|d| d.diagnostic.code.starts_with(prefix))
        .map(|d| d.diagnostic.code.clone())
        .collect()
}

fn state(ws: &Workspace) -> Option<SharedCatalogState> {
    let info = query::project_info(ws).unwrap();
    info.catalogs
        .iter()
        .find(|c| c.name == "lab")
        .and_then(|c| c.shared.as_ref())
        .map(|s| s.state)
}

/// 取得で lock とキャッシュができ、共有の品を使う部品の値・バジェット・解析は、同じ品をこのプロジェクトのカタログに
/// 置いたデモと同じになる。共有の品は読み取り専用。
#[test]
fn fetch_loads_shared_items_and_analyses_run() {
    let tmp = tempfile::tempdir().unwrap();
    let repo = tmp.path().join("catalog-repo");
    catalog_repo(&repo);
    let ws = project(tmp.path(), &repo, "main");
    assert_eq!(state(&ws), Some(SharedCatalogState::NotLocked));
    assert!(codes(&ws, "catalog.").contains(&"catalog.not_locked".to_owned()));
    let result = shared_catalog::fetch(&ws, &CatalogFetchRequest::default()).unwrap();
    assert!(result.lock_changed);
    let head = git(&repo, &["rev-parse", "HEAD"]);
    assert_eq!(result.catalogs[0].commit, head);
    assert_eq!(result.catalogs[0].previous, None);
    assert!(
        result
            .items
            .iter()
            .any(|i| i.id == "lab/rw_10mNms" && i.change == CatalogItemChangeKind::Added)
    );
    let lock = std::fs::read_to_string(ws.root().join("catalog.lock")).unwrap();
    assert!(lock.contains(&format!("commit = \"{head}\"")), "{lock}");
    assert!(
        ws.root()
            .join(format!(".fune/catalogs/lab/{head}/catalog/rw_10mNms.toml"))
            .is_file()
    );
    assert!(
        !ws.root()
            .join(format!(".fune/catalogs/lab/{head}/README.md"))
            .exists()
    );
    assert_eq!(state(&ws), Some(SharedCatalogState::Ready));
    // 共有カタログに関する誤りは無い（デモと同じ診断の数）
    let demo = Workspace::open(fixture()).unwrap();
    let errors = |w: &Workspace| query::check(w).unwrap().counts.errors;
    assert_eq!(errors(&ws), errors(&demo));
    let view = query::design_view(&ws).unwrap();
    let item = view
        .catalog
        .iter()
        .find(|c| c.id == "lab/rw_10mNms")
        .unwrap();
    assert!(!item.editable);
    assert_eq!(item.datasheet.as_deref(), Some("lab/ds_rw10"));
    assert_eq!(item.datasheet_link, demo_item_link(&demo));
    let shared_source = view
        .sources
        .iter()
        .find(|s| s.source.id.as_str() == "lab/ds_rw10")
        .unwrap();
    assert!(!shared_source.editable);
    let part = query::part_view(&ws, "RW_X", &Default::default()).unwrap();
    let demo_part = query::part_view(&demo, "RW_X", &Default::default()).unwrap();
    assert_eq!(
        part.part.values["mass"].display,
        demo_part.part.values["mass"].display
    );
    let mass = |w: &Workspace| {
        budget::budget(w, BudgetKind::Mass, &BudgetRequest::default())
            .unwrap()
            .total
    };
    assert_eq!(mass(&ws).map(|q| q.display), mass(&demo).map(|q| q.display));
    let run = |w: &Workspace| {
        analysis::run_analysis(w, "aocs.rw_unloading", &RunAnalysisRequest::default())
            .unwrap()
            .output
    };
    let output = run(&ws);
    assert!(output.is_some());
    assert_eq!(output, run(&demo));
    // 2 回目の取得は何も変えない
    let again = shared_catalog::fetch(&ws, &CatalogFetchRequest { locked: true }).unwrap();
    assert!(!again.lock_changed && again.items.is_empty());
    assert_eq!(again.catalogs[0].previous.as_deref(), Some(head.as_str()));
}

fn demo_item_link(demo: &Workspace) -> Option<String> {
    let view = query::design_view(demo).unwrap();
    view.catalog
        .into_iter()
        .find(|c| c.id == "local/rw_10mNms")
        .and_then(|c| c.datasheet_link)
}

/// 更新は lock を新しい commit に上げ、変わった値を返す。取得は lock の commit を動かさない。
#[test]
fn update_moves_the_lock_and_reports_changed_values() {
    let tmp = tempfile::tempdir().unwrap();
    let repo = tmp.path().join("catalog-repo");
    catalog_repo(&repo);
    let ws = project(tmp.path(), &repo, "main");
    let first = shared_catalog::fetch(&ws, &CatalogFetchRequest::default()).unwrap();
    let file = repo.join("catalog/rw_10mNms.toml");
    let text = std::fs::read_to_string(&file).unwrap();
    std::fs::write(
        &file,
        text.replace("value = 150.0, unit = \"g\"", "value = 160.0, unit = \"g\""),
    )
    .unwrap();
    std::fs::write(
        repo.join("catalog/mtq_new.toml"),
        "kind = \"magnetorquer\"\n",
    )
    .unwrap();
    git(&repo, &["add", "."]);
    git(&repo, &["commit", "-q", "-m", "heavier"]);
    // fetch は lock を守る
    let kept = shared_catalog::fetch(&ws, &CatalogFetchRequest::default()).unwrap();
    assert_eq!(kept.catalogs[0].commit, first.catalogs[0].commit);
    assert!(!kept.lock_changed);
    let updated =
        shared_catalog::update(&ws, &CatalogUpdateRequest { name: "lab".into() }).unwrap();
    assert!(updated.lock_changed);
    assert_eq!(
        updated.catalogs[0].commit,
        git(&repo, &["rev-parse", "HEAD"])
    );
    assert_eq!(
        updated.catalogs[0].previous,
        Some(first.catalogs[0].commit.clone())
    );
    let rw = updated
        .items
        .iter()
        .find(|i| i.id == "lab/rw_10mNms")
        .unwrap();
    assert_eq!(rw.change, CatalogItemChangeKind::Changed);
    let mass = rw.values.iter().find(|v| v.key == "attr.mass").unwrap();
    assert_eq!(mass.before.as_deref(), Some("150 g"));
    assert_eq!(mass.after.as_deref(), Some("160 g"));
    assert_eq!(rw.values.len(), 1, "{:?}", rw.values);
    assert!(
        updated
            .items
            .iter()
            .any(|i| i.id == "lab/mtq_new" && i.change == CatalogItemChangeKind::Added)
    );
    // RW_X は質量を部品で上書きしている（EM 実測）。上書きの無い RW_Y は品の新しい値になる
    let part = query::part_view(&ws, "RW_Y", &Default::default()).unwrap();
    assert_eq!(part.part.values["mass"].display.as_deref(), Some("160 g"));
    let missing = shared_catalog::update(
        &ws,
        &CatalogUpdateRequest {
            name: "local".into(),
        },
    );
    assert_eq!(missing.unwrap_err().kind(), ErrorKind::NotFound);
}

/// lock と fune.toml が違う・キャッシュが無いときの診断と、`--locked` の取得の拒否。タグと commit でも固定できる。
#[test]
fn mismatch_and_missing_cache_are_diagnosed() {
    let tmp = tempfile::tempdir().unwrap();
    let repo = tmp.path().join("catalog-repo");
    catalog_repo(&repo);
    let ws = project(tmp.path(), &repo, "main");
    let locked = shared_catalog::fetch(&ws, &CatalogFetchRequest { locked: true });
    assert_eq!(locked.unwrap_err().kind(), ErrorKind::InvalidInput);
    assert!(!ws.root().join("catalog.lock").exists());
    shared_catalog::fetch(&ws, &CatalogFetchRequest::default()).unwrap();
    // fune.toml の rev をタグに変えると lock と食い違う
    let toml = ws.root().join("fune.toml");
    let text = std::fs::read_to_string(&toml).unwrap();
    std::fs::write(&toml, text.replace("rev  = \"main\"", "rev  = \"v1\"")).unwrap();
    assert_eq!(state(&ws), Some(SharedCatalogState::LockMismatch));
    assert!(codes(&ws, "catalog.").contains(&"catalog.lock_mismatch".to_owned()));
    let err = shared_catalog::fetch(&ws, &CatalogFetchRequest { locked: true }).unwrap_err();
    assert_eq!(err.kind(), ErrorKind::InvalidInput);
    assert!(err.to_string().contains("fune catalog fetch"), "{err}");
    let fixed = shared_catalog::fetch(&ws, &CatalogFetchRequest::default()).unwrap();
    assert!(fixed.lock_changed);
    assert_eq!(fixed.catalogs[0].rev.as_deref(), Some("v1"));
    assert_eq!(state(&ws), Some(SharedCatalogState::Ready));
    // キャッシュを消すと「未取得」。取得し直せる
    std::fs::remove_dir_all(ws.root().join(".fune/catalogs")).unwrap();
    assert_eq!(state(&ws), Some(SharedCatalogState::NotFetched));
    let diagnostics = query::check(&ws).unwrap();
    let not_fetched = diagnostics
        .diagnostics
        .iter()
        .find(|d| d.diagnostic.code == "catalog.not_fetched")
        .unwrap();
    assert!(
        not_fetched
            .diagnostic
            .message
            .contains("fune catalog fetch")
    );
    let again = shared_catalog::fetch(&ws, &CatalogFetchRequest { locked: true }).unwrap();
    assert!(!again.lock_changed);
    assert_eq!(state(&ws), Some(SharedCatalogState::Ready));
    // commit を直に rev にしても固定できる
    let head = git(&repo, &["rev-parse", "HEAD"]);
    let text = std::fs::read_to_string(&toml).unwrap();
    std::fs::write(
        &toml,
        text.replace("rev  = \"v1\"", &format!("rev  = \"{}\"", &head[..12])),
    )
    .unwrap();
    let by_commit = shared_catalog::fetch(&ws, &CatalogFetchRequest::default()).unwrap();
    assert_eq!(by_commit.catalogs[0].commit, head);
}

/// 取得元に繋がらないときは、共有カタログの名前の付いた誤り。lock の commit のキャッシュがあれば繋がらなくても読める。
#[test]
fn unreachable_remote_is_an_error_but_cached_commits_still_load() {
    let tmp = tempfile::tempdir().unwrap();
    let repo = tmp.path().join("catalog-repo");
    catalog_repo(&repo);
    let ws = project(tmp.path(), &repo, "main");
    shared_catalog::fetch(&ws, &CatalogFetchRequest::default()).unwrap();
    std::fs::remove_dir_all(&repo).unwrap();
    let kept = shared_catalog::fetch(&ws, &CatalogFetchRequest { locked: true }).unwrap();
    assert!(!kept.lock_changed);
    let err =
        shared_catalog::update(&ws, &CatalogUpdateRequest { name: "lab".into() }).unwrap_err();
    assert!(err.to_string().contains("Shared catalog `lab`"), "{err}");
    assert_eq!(state(&ws), Some(SharedCatalogState::Ready));
}

/// 共有カタログの品がディレクトリの下にあっても取得して読める（ID はファイル名。API の `dir` はその置き場所）。
#[test]
fn fetch_reads_items_in_directories() {
    let tmp = tempfile::tempdir().unwrap();
    let repo = tmp.path().join("catalog-repo");
    catalog_repo(&repo);
    let nested = repo.join("catalog/vendor_a/wheels");
    std::fs::create_dir_all(&nested).unwrap();
    std::fs::rename(
        repo.join("catalog/rw_10mNms.toml"),
        nested.join("rw_10mNms.toml"),
    )
    .unwrap();
    git(&repo, &["add", "-A"]);
    git(&repo, &["commit", "-q", "-m", "move into directories"]);
    let ws = project(tmp.path(), &repo, "main");
    shared_catalog::fetch(&ws, &CatalogFetchRequest::default()).unwrap();
    let head = git(&repo, &["rev-parse", "HEAD"]);
    assert!(
        ws.root()
            .join(format!(
                ".fune/catalogs/lab/{head}/catalog/vendor_a/wheels/rw_10mNms.toml"
            ))
            .is_file()
    );
    assert_eq!(state(&ws), Some(SharedCatalogState::Ready));
    let demo = Workspace::open(fixture()).unwrap();
    let errors = |w: &Workspace| query::check(w).unwrap().counts.errors;
    assert_eq!(errors(&ws), errors(&demo));
    let view = query::design_view(&ws).unwrap();
    let item = view
        .catalog
        .iter()
        .find(|c| c.id == "lab/rw_10mNms")
        .unwrap();
    assert_eq!(item.dir, ["vendor_a", "wheels"]);
    assert!(!item.editable);
}

/// 共有カタログの出典もディレクトリの下にあっても取得して読める（ID はファイル名。`SourceView.dir` が置き場所）。
#[test]
fn fetch_reads_sources_in_directories() {
    let tmp = tempfile::tempdir().unwrap();
    let repo = tmp.path().join("catalog-repo");
    catalog_repo(&repo);
    std::fs::create_dir_all(repo.join("sources/docs/vendor")).unwrap();
    let to = "sources/docs/vendor/ds_vendor.toml";
    std::fs::rename(repo.join("sources/ds_vendor.toml"), repo.join(to)).unwrap();
    git(&repo, &["add", "-A"]);
    git(&repo, &["commit", "-q", "-m", "move sources"]);
    let ws = project(tmp.path(), &repo, "main");
    shared_catalog::fetch(&ws, &CatalogFetchRequest::default()).unwrap();
    let head = git(&repo, &["rev-parse", "HEAD"]);
    let cached = ws.root().join(format!(".fune/catalogs/lab/{head}/{to}"));
    assert!(cached.is_file());
    assert_eq!(state(&ws), Some(SharedCatalogState::Ready));
    let view = query::design_view(&ws).unwrap();
    let id = "lab/ds_vendor";
    let source = view.sources.iter().find(|s| s.source.id == id.into());
    assert_eq!(source.unwrap().dir, ["docs", "vendor"]);
}
