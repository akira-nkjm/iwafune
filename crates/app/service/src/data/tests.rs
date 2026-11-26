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

#[test]
fn radiation_group_lists_exactly_the_radiation_rows() {
    let ids: Vec<_> = RELEASE_DATASETS.iter().map(|d| d.spec.id).collect();
    for id in radiation::GROUP_IDS {
        assert!(ids.contains(&id), "{id} is not in the table");
    }
    // 表は DE440 とこの群だけ
    assert_eq!(ids.len(), 1 + radiation::GROUP_IDS.len());
    assert_ne!(radiation::GROUP_ID, space_weather::ID);
    assert!(!ids.contains(&radiation::GROUP_ID));
    // 出所の表示は行ごとにある。NASA の地図には再配布の事情（無変更・ライセンスの記載なし）と文献、IGRF には CC BY 4.0
    for d in RELEASE_DATASETS.iter().skip(1) {
        let text = &d.spec.license;
        if d.spec.id == "igrf14" {
            assert!(text.contains("CC BY 4.0") && text.contains("10.5281/zenodo.14012302"));
        } else if d.spec.id == "shieldose2" {
            assert!(text.contains("NISTIR 5477") && text.contains("LGPL-3.0-or-later"));
            assert!(text.contains("2013 correction"));
            // 地図とは別のリリース（行ごとのタグ。群で取得しても特別な扱いは要らない）
            assert_eq!(d.spec.tag, "data-shielding-v1");
        } else {
            assert!(text.contains("PT-11B") && text.contains("redistributed unmodified"));
            assert!(
                text.contains("Sawyer & Vette 1976") && text.contains("Heynderickx & Beliaev 1995")
            );
        }
    }
}

/// 表の SHA-256・大きさが、手元にある配布ファイル（キャッシュ）と合っている（無ければ読み飛ばす）。
#[test]
fn radiation_rows_match_the_files_when_present() {
    let Ok(dir) = cache_dir() else { return };
    let mut checked = 0;
    for d in RELEASE_DATASETS.iter().skip(1) {
        let path = dir.join(d.spec.file);
        let Ok(bytes) = std::fs::read(&path) else {
            eprintln!("SKIPPED: {} is not in the data cache", path.display());
            continue;
        };
        assert_eq!(bytes.len() as u64, d.spec.size, "{}", d.spec.id);
        assert_eq!(sha256_hex(&bytes), d.spec.sha256, "{}", d.spec.id);
        checked += 1;
    }
    eprintln!("checked {checked} radiation files against the table");
}

/// ネットワークの代わりにディレクトリから取り込む（`fetch` の `from`）。大きさと SHA-256 の確かめ・置き換えは取得と同じ。
#[test]
fn install_from_a_directory_checks_and_places_the_file() {
    let source = tempfile::tempdir().unwrap();
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(source.path().join(FILE), BYTES).unwrap();
    let ds = test_dataset(accept);
    fetch_in(dir.path(), "test_data", &[&ds], &copy_from(source.path())).unwrap();
    assert_eq!(std::fs::read(dir.path().join(FILE)).unwrap(), BYTES);
    assert!(ds.item(dir.path()).unwrap().installed);
    // 中身が違えば置き換えない
    let bad = tempfile::tempdir().unwrap();
    std::fs::write(bad.path().join(FILE), b"test dataset byteX").unwrap();
    let err = fetch_in(dir.path(), "test_data", &[&ds], &copy_from(bad.path())).unwrap_err();
    assert!(err.to_string().contains("SHA-256 mismatch"), "{err}");
    assert_eq!(std::fs::read(dir.path().join(FILE)).unwrap(), BYTES);
    // ディレクトリにファイルが無ければ誤り
    let empty = tempfile::tempdir().unwrap();
    assert!(fetch_in(dir.path(), "test_data", &[&ds], &copy_from(empty.path())).is_err());
    // 宇宙天気はディレクトリから取り込めない
    let err = fetch("space_weather", Some(source.path())).unwrap_err();
    assert!(
        err.to_string()
            .contains("cannot be installed from a directory")
    );
}

/// 不足入力が示すデータをそろえる手順（projection が持つ文）は、ここの群の ID と一致する。
#[test]
fn the_missing_input_hint_names_the_radiation_group() {
    let hint = fune_projection::orbit::RADIATION_INSTALL_HINT;
    assert!(
        hint.starts_with(&format!("fune data fetch {}", radiation::GROUP_ID)),
        "{hint}"
    );
    assert!(hint.contains("--from <DIR>"));
}
