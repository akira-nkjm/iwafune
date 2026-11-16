//! ネットワーク無しの確かめ。偽のリリース（システムの `tar` で作った tar.gz と `.sha256`、版を答えるだけのシェルスクリプト）と、
//! 偽の「実行中のバイナリ」で、確認から置き換えまでを通す。

use super::*;
use crate::data::sha256_hex;
use std::collections::BTreeMap;
use std::process::Command;

const REPO: &str = "https://github.com/o/r";
const OLD: &[u8] = b"old binary";

/// 取得先ごとの中身を持つ偽のホスト。展開は本物の `tar`、版の確認は偽のバイナリの `--version`。
struct FakeHost {
    files: BTreeMap<String, Vec<u8>>,
}

impl Host for FakeHost {
    fn fetch_text(&self, url: &str) -> Result<String> {
        self.files
            .get(url)
            .map(|b| String::from_utf8_lossy(b).into_owned())
            .ok_or_else(|| ServiceError::Data(format!("Cannot fetch {url}: 404")))
    }

    fn download(&self, url: &str, to: &Path) -> Result<()> {
        let bytes = self
            .files
            .get(url)
            .ok_or_else(|| ServiceError::Data(format!("Cannot fetch {url}: 404")))?;
        std::fs::write(to, bytes).map_err(|e| ServiceError::Data(e.to_string()))
    }

    fn extract(&self, archive: &Path, into: &Path) -> Result<()> {
        system::SystemHost.extract(archive, into)
    }

    fn run_version(&self, binary: &Path) -> Result<String> {
        system::SystemHost.run_version(binary)
    }
}

/// 偽のリリースの 1 つの版。`reported` は偽のバイナリが名乗る版の文字列。
fn release(platform: Platform, version: &str, reported: &str) -> BTreeMap<String, Vec<u8>> {
    let v = Version::parse(version).unwrap();
    let work = tempfile::tempdir().unwrap();
    let dir = work.path().join(dir_name(v, platform));
    std::fs::create_dir(&dir).unwrap();
    std::fs::write(
        dir.join("fune"),
        format!("#!/bin/sh\necho 'fune {reported}'\n"),
    )
    .unwrap();
    std::fs::write(dir.join("README.md"), "readme").unwrap();
    let file = archive_name(v, platform);
    let status = Command::new("tar")
        .arg("-czf")
        .arg(work.path().join(&file))
        .arg("-C")
        .arg(work.path())
        .arg(dir_name(v, platform))
        .status()
        .unwrap();
    assert!(status.success());
    let archive = std::fs::read(work.path().join(&file)).unwrap();
    let sum = format!("{}  {file}\n", sha256_hex(&archive));
    BTreeMap::from([
        (download_url(REPO, v, &file), archive),
        (
            download_url(REPO, v, &format!("{file}.sha256")),
            sum.into_bytes(),
        ),
    ])
}

fn latest_json(tag: &str) -> (String, Vec<u8>) {
    (
        "https://api.github.com/repos/o/r/releases/latest".to_owned(),
        format!(r#"{{"tag_name":"{tag}"}}"#).into_bytes(),
    )
}

struct Fixture {
    /// 「実行中のバイナリ」のあるディレクトリ
    dir: tempfile::TempDir,
    ctx: Context,
}

fn fixture(current: &str) -> Fixture {
    let dir = tempfile::tempdir().unwrap();
    let exe = dir.path().join("fune");
    std::fs::write(&exe, OLD).unwrap();
    let ctx = Context {
        repo: REPO.to_owned(),
        current: Version::parse(current).unwrap(),
        platform: Platform::LinuxX86_64,
        exe,
    };
    Fixture { dir, ctx }
}

fn host(latest: &str, reported: &str) -> FakeHost {
    let mut files = release(Platform::LinuxX86_64, latest, reported);
    files.extend([latest_json(&format!("v{latest}"))]);
    FakeHost { files }
}

/// 実行中のバイナリが変わっておらず、一時ディレクトリが残っていない。
fn assert_untouched(f: &Fixture) {
    assert_eq!(std::fs::read(&f.ctx.exe).unwrap(), OLD);
    let names: Vec<_> = std::fs::read_dir(f.dir.path()).unwrap().collect();
    assert_eq!(names.len(), 1, "temporary files were left behind");
}

#[test]
fn check_reports_a_newer_release() {
    let f = fixture("0.5.1");
    let c = check_with(&f.ctx, &host("0.5.2", "0.5.2"), None).unwrap();
    assert_eq!((c.current.as_str(), c.latest.as_str()), ("0.5.1", "0.5.2"));
    assert!(c.available && !c.ahead && !c.explicit);
    assert_eq!(c.asset, "fune-0.5.2-x86_64-linux-gnu.tar.gz");
    assert_eq!(
        c.url,
        "https://github.com/o/r/releases/download/v0.5.2/fune-0.5.2-x86_64-linux-gnu.tar.gz"
    );
    assert_untouched(&f);
}

#[test]
fn update_replaces_the_executable() {
    let f = fixture("0.5.1");
    let out = update_with(&f.ctx, &host("0.5.2", "0.5.2"), None).unwrap();
    assert_eq!(out.replaced.as_deref(), Some(f.ctx.exe.to_str().unwrap()));
    let version = system::SystemHost.run_version(&f.ctx.exe).unwrap();
    assert_eq!(version.trim(), "fune 0.5.2");
    let names: Vec<_> = std::fs::read_dir(f.dir.path()).unwrap().collect();
    assert_eq!(names.len(), 1, "temporary files were left behind");
}

#[test]
fn already_latest_does_nothing() {
    let f = fixture("0.5.2");
    let out = update_with(&f.ctx, &host("0.5.2", "0.5.2"), None).unwrap();
    assert!(!out.check.available && !out.check.ahead);
    assert!(out.replaced.is_none());
    assert_untouched(&f);
}

#[test]
fn a_development_build_is_left_alone_unless_a_version_is_given() {
    let f = fixture("0.5.1");
    let h = host("0.5.0", "0.5.0");
    let out = update_with(&f.ctx, &h, None).unwrap();
    assert!(out.check.ahead && !out.check.available && out.replaced.is_none());
    assert_untouched(&f);
    let out = update_with(&f.ctx, &h, Some("0.5.0")).unwrap();
    assert!(out.check.explicit && out.replaced.is_some());
    let v = system::SystemHost.run_version(&f.ctx.exe).unwrap();
    assert_eq!(v.trim(), "fune 0.5.0");
}

#[test]
fn checksum_mismatch_replaces_nothing() {
    let f = fixture("0.5.1");
    let mut h = host("0.5.2", "0.5.2");
    let v = Version::parse("0.5.2").unwrap();
    let sum = download_url(REPO, v, "fune-0.5.2-x86_64-linux-gnu.tar.gz.sha256");
    h.files.insert(
        sum,
        format!("{}  fune-0.5.2-x86_64-linux-gnu.tar.gz\n", "0".repeat(64)).into_bytes(),
    );
    let e = update_with(&f.ctx, &h, None).unwrap_err();
    assert!(e.to_string().contains("does not match"), "{e}");
    assert_untouched(&f);
}

#[test]
fn wrong_reported_version_replaces_nothing() {
    let f = fixture("0.5.1");
    let e = update_with(&f.ctx, &host("0.5.2", "0.5.3"), None).unwrap_err();
    assert!(e.to_string().contains("instead of 'fune 0.5.2'"), "{e}");
    assert_untouched(&f);
}

#[test]
fn bad_versions_and_answers_are_errors() {
    let f = fixture("0.5.1");
    let h = host("0.5.2", "0.5.2");
    assert!(check_with(&f.ctx, &h, Some("0.5")).is_err());
    assert!(check_with(&f.ctx, &h, Some("0.5.2-rc1")).is_err());
    let mut bad = host("0.5.2", "0.5.2");
    bad.files.extend([latest_json("nightly")]);
    assert!(check_with(&f.ctx, &bad, None).is_err());
    let none = FakeHost {
        files: BTreeMap::new(),
    };
    let e = check_with(&f.ctx, &none, None).unwrap_err();
    assert!(e.to_string().contains("--version"), "{e}");
}

#[test]
#[cfg(unix)]
fn an_unwritable_directory_is_explained() {
    use std::os::unix::fs::PermissionsExt;
    let f = fixture("0.5.1");
    std::fs::set_permissions(f.dir.path(), std::fs::Permissions::from_mode(0o555)).unwrap();
    let denied = std::fs::create_dir(f.dir.path().join("probe")).is_err();
    let r = update_with(&f.ctx, &host("0.5.2", "0.5.2"), None);
    std::fs::set_permissions(f.dir.path(), std::fs::Permissions::from_mode(0o755)).unwrap();
    if denied {
        let e = r.unwrap_err();
        assert!(e.to_string().contains("cannot write"), "{e}");
    }
}
