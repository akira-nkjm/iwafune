//! データの表が状態と取得を決めること（取得は偽の取得関数で、ネットワークは使わない）。

use super::release::{Dataset, ReleaseSpec};
use super::*;
use sha2::{Digest, Sha256};

const BYTES: &[u8] = b"test dataset bytes";
const FILE: &str = "test.bin";

fn hex(bytes: &[u8]) -> String {
    Sha256::digest(bytes)
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect()
}

/// 試験用の行（本物の表には入れない）。`sha256` は `BYTES` のもの。
fn test_dataset(installer: release::Installer) -> Dataset {
    let sha: &'static str = Box::leak(hex(BYTES).into_boxed_str());
    Dataset::new(ReleaseSpec {
        id: "test_data",
        file: FILE,
        repo: "owner/repo",
        tag: "data-test-v1",
        sha256: sha,
        size: BYTES.len() as u64,
        used_by: "tests",
        license: "test licence",
        installer,
    })
}

/// 成果物として `bytes` を置く偽の取得関数。
fn serving(bytes: &'static [u8]) -> impl Fn(&ReleaseSpec, &Path) -> Result<()> {
    move |spec, tmp| std::fs::write(tmp.join(spec.file), bytes).map_err(|e| io_error(tmp, e))
}

fn accept(bytes: Vec<u8>) -> std::result::Result<(), String> {
    assert_eq!(bytes, BYTES);
    Ok(())
}

fn reject(_: Vec<u8>) -> std::result::Result<(), String> {
    Err("cannot read".into())
}

#[test]
fn real_table_has_unique_ids_and_starts_with_de440() {
    let ids: Vec<_> = RELEASE_DATASETS.iter().map(|d| d.spec.id).collect();
    assert_eq!(ids[0], "de440");
    for (i, id) in ids.iter().enumerate() {
        assert!(!ids[..i].contains(id), "duplicate id {id}");
        assert_ne!(*id, space_weather::ID);
    }
    let de440 = &de440::DE440.spec;
    assert_eq!((de440.file, de440.tag), ("de440.bin", "data-de440-v1"));
}

#[test]
fn table_drives_status_and_fetch() {
    let dir = tempfile::tempdir().unwrap();
    let ds = test_dataset(accept);
    let table = [&ds];
    let before = table[0].item(dir.path()).unwrap();
    assert!(!before.present && !before.installed && before.verified.is_none());
    assert_eq!(
        before.source,
        "gh release download data-test-v1 --repo owner/repo"
    );
    assert_eq!(before.license.as_deref(), Some("test licence"));

    fetch_in(dir.path(), "test_data", &table, &serving(BYTES)).unwrap();
    assert_eq!(std::fs::read(dir.path().join(FILE)).unwrap(), BYTES);
    let after = table[0].item(dir.path()).unwrap();
    assert!(after.present && after.installed && after.verified == Some(true));
    assert!(after.install_error.is_none());
    // 一時ディレクトリは残らない
    let leftovers = std::fs::read_dir(dir.path()).unwrap().count();
    assert_eq!(leftovers, 1);
}

#[test]
fn install_failure_is_reported_in_the_status() {
    let dir = tempfile::tempdir().unwrap();
    let ds = test_dataset(reject);
    fetch_in(dir.path(), "test_data", &[&ds], &serving(BYTES)).unwrap();
    let item = ds.item(dir.path()).unwrap();
    assert!(item.present && !item.installed);
    assert_eq!(item.install_error.as_deref(), Some("cannot read"));
}

#[test]
fn sha_mismatch_leaves_the_existing_file() {
    let dir = tempfile::tempdir().unwrap();
    let existing = dir.path().join(FILE);
    std::fs::write(&existing, b"old").unwrap();
    let ds = test_dataset(accept);
    // 大きさは同じで中身が違う成果物
    let err = fetch_in(
        dir.path(),
        "test_data",
        &[&ds],
        &serving(b"test dataset byteX"),
    )
    .unwrap_err()
    .to_string();
    assert!(err.contains("SHA-256 mismatch for test_data"), "{err}");
    assert_eq!(std::fs::read(&existing).unwrap(), b"old");
    assert!(!ds.item(dir.path()).unwrap().installed);
    // 大きさが違うときは、ハッシュの前にそう知らせる
    let err = fetch_in(dir.path(), "test_data", &[&ds], &serving(b"short"))
        .unwrap_err()
        .to_string();
    assert!(err.contains("Size mismatch for test_data"), "{err}");
    assert_eq!(std::fs::read(&existing).unwrap(), b"old");
}

#[test]
fn download_failure_leaves_the_existing_file() {
    let dir = tempfile::tempdir().unwrap();
    let existing = dir.path().join(FILE);
    std::fs::write(&existing, b"old").unwrap();
    let ds = test_dataset(accept);
    let failing = |_: &ReleaseSpec, _: &Path| Err(ServiceError::Data("no network".into()));
    let err = fetch_in(dir.path(), "test_data", &[&ds], &failing).unwrap_err();
    assert!(err.to_string().contains("no network"));
    assert_eq!(std::fs::read(&existing).unwrap(), b"old");
}

#[test]
fn unknown_id_is_not_found() {
    let dir = tempfile::tempdir().unwrap();
    let ds = test_dataset(accept);
    let err = fetch_in(dir.path(), "nope", &[&ds], &serving(BYTES)).unwrap_err();
    assert!(matches!(err, ServiceError::NotFound { what: "data", .. }));
    assert_eq!(err.to_string(), "data not found: nope");
}
