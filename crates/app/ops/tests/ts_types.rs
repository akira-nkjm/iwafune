//! API の型が ts-rs で書き出せること（名前の重複が無いこと）を確かめる。書き出し先は一時ディレクトリ。

use ts_rs::{Config, TS};

#[test]
fn api_types_export_to_typescript() {
    let dir = tempfile::tempdir().unwrap();
    let cfg = Config::new().with_out_dir(dir.path());
    fune_ops::Op::export_all(&cfg).unwrap();
    fune_ops::ApplyOutcome::export_all(&cfg).unwrap();
    fune_ops::ApplyOptions::export_all(&cfg).unwrap();
    fune_model::Design::export_all(&cfg).unwrap();
    fune_model::Resolved::export_all(&cfg).unwrap();
    fune_model::Diagnostic::export_all(&cfg).unwrap();
    fune_model::path::ParamPath::export_all(&cfg).unwrap();
    fune_model::builtin::Labels::export_all(&cfg).unwrap();
    let mut names: Vec<String> = std::fs::read_dir(dir.path())
        .unwrap()
        .filter_map(Result::ok)
        .map(|e| e.file_name().to_string_lossy().into_owned())
        .collect();
    names.sort();
    assert!(names.contains(&"AttrValue.ts".to_string()), "{names:?}");
    assert!(names.contains(&"Op.ts".to_string()));
    let value = std::fs::read_to_string(dir.path().join("AttrValue.ts")).unwrap();
    assert!(value.contains("\"type\": \"by_state\""), "{value}");
}

#[test]
#[ignore]
fn dump_ts() {
    let dir = std::path::PathBuf::from(std::env::var("TS_DUMP").unwrap());
    let cfg = Config::new().with_out_dir(&dir);
    fune_ops::ApplyOutcome::export_all(&cfg).unwrap();
    fune_model::Design::export_all(&cfg).unwrap();
    fune_model::Resolved::export_all(&cfg).unwrap();
    fune_ops::Op::export_all(&cfg).unwrap();
}
