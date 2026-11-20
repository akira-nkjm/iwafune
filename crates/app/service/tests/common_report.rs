//! 書き出しのテストの共通部品（デモの設計・複製・ファイルの書き出し）。
#![allow(dead_code)]

use fune_service::Workspace;
use fune_service::dto::{ReportExport, ReportRequest};
use std::path::{Path, PathBuf};

pub fn demo_path() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../../fixtures/demo-6u")
}

pub fn demo() -> Workspace {
    Workspace::open(demo_path()).expect("demo")
}

/// ディレクトリを複製する。
pub fn copy_dir(from: &Path, to: &Path) {
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

/// デモの一時コピー（壊してよい）。
pub fn demo_copy() -> (tempfile::TempDir, Workspace) {
    let tmp = tempfile::tempdir().unwrap();
    let root = tmp.path().join("demo-6u");
    copy_dir(&demo_path(), &root);
    let ws = Workspace::open(&root).unwrap();
    (tmp, ws)
}

pub fn export(ws: &Workspace, req: &ReportRequest) -> ReportExport {
    fune_service::report::export(ws, req).expect("export")
}

/// 書き出したファイルの本文。
pub fn file<'a>(r: &'a ReportExport, path: &str) -> &'a str {
    &r.files
        .iter()
        .find(|f| f.path == path)
        .unwrap_or_else(|| panic!("no file {path}"))
        .content
}

/// `data.json` を読む。
pub fn data(r: &ReportExport) -> serde_json::Value {
    serde_json::from_str(file(r, "data.json")).unwrap()
}

/// ファイルをディレクトリに書く。
pub fn write_all(r: &ReportExport, dir: &Path) {
    for f in &r.files {
        let path = dir.join(&f.path);
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(path, &f.content).unwrap();
    }
}
