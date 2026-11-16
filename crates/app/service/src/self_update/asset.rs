//! リリースの成果物の名前と取得先、実行環境から選ぶ成果物、`.sha256` の読み方。
//! **成果物の名前の規則はここだけ**（cargo-binstall の宣言との一致は `crates/app/cli/Cargo.toml` を読むテストで確かめる）。

use super::version::Version;
use crate::error::{Result, ServiceError};

/// リリースの成果物がある環境。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Platform {
    /// x86_64 の Linux（glibc）
    LinuxX86_64,
    /// macOS（Apple Silicon と Intel で共通のバイナリ）
    MacosUniversal,
}

impl Platform {
    /// このバイナリを作った環境。成果物が無い環境は誤り。
    pub fn current() -> Result<Self> {
        if cfg!(all(target_os = "linux", target_arch = "x86_64")) {
            Ok(Self::LinuxX86_64)
        } else if cfg!(target_os = "macos") {
            Ok(Self::MacosUniversal)
        } else {
            Err(ServiceError::SelfUpdate(format!(
                "No release binary exists for this platform ({} {}). Build from source instead: cargo install --git {} fune-cli",
                std::env::consts::OS,
                std::env::consts::ARCH,
                repository()
            )))
        }
    }

    /// 成果物の名前に入る環境の部分。
    pub fn target(self) -> &'static str {
        match self {
            Self::LinuxX86_64 => "x86_64-linux-gnu",
            Self::MacosUniversal => "universal-apple-darwin",
        }
    }
}

/// このプロジェクトのリポジトリの URL（パッケージの情報から。末尾の `/` なし）。
pub fn repository() -> &'static str {
    env!("CARGO_PKG_REPOSITORY").trim_end_matches('/')
}

/// 成果物（アーカイブ）の名前。
pub fn archive_name(version: Version, platform: Platform) -> String {
    format!("fune-{version}-{}.tar.gz", platform.target())
}

/// アーカイブの中のディレクトリの名前（中に `fune` がある）。
pub fn dir_name(version: Version, platform: Platform) -> String {
    format!("fune-{version}-{}", platform.target())
}

/// 成果物の取得先。`repo` はリポジトリの URL。
pub fn download_url(repo: &str, version: Version, file: &str) -> String {
    format!("{repo}/releases/download/v{version}/{file}")
}

/// 最新のリリースを返す API の URL（`repo` は `https://github.com/<owner>/<name>`）。
pub fn latest_api_url(repo: &str) -> Result<String> {
    let path = repo
        .strip_prefix("https://github.com/")
        .filter(|p| p.split('/').count() == 2 && !p.is_empty())
        .ok_or_else(|| {
            ServiceError::SelfUpdate(format!("The repository URL is not a GitHub URL: {repo}"))
        })?;
    Ok(format!(
        "https://api.github.com/repos/{path}/releases/latest"
    ))
}

/// `.sha256`（`sha256sum` の出力 `<64 桁の 16 進>  <ファイル名>`）から SHA-256 を取り出す。
/// ファイル名があれば `file` と一致することも確かめる。
pub fn parse_sha256_file(text: &str, file: &str) -> Result<String> {
    let bad = || ServiceError::SelfUpdate(format!("The checksum file for {file} is malformed"));
    let mut words = text.split_whitespace();
    let hash = words.next().ok_or_else(bad)?;
    if hash.len() != 64 || !hash.bytes().all(|b| b.is_ascii_hexdigit()) {
        return Err(bad());
    }
    if let Some(name) = words.next()
        && name.trim_start_matches('*') != file
    {
        return Err(bad());
    }
    Ok(hash.to_ascii_lowercase())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn names_follow_the_release_layout() {
        let v = Version::parse("0.5.2").unwrap();
        assert_eq!(
            archive_name(v, Platform::LinuxX86_64),
            "fune-0.5.2-x86_64-linux-gnu.tar.gz"
        );
        assert_eq!(
            dir_name(v, Platform::MacosUniversal),
            "fune-0.5.2-universal-apple-darwin"
        );
        assert_eq!(
            download_url("https://github.com/o/r", v, "a.tar.gz"),
            "https://github.com/o/r/releases/download/v0.5.2/a.tar.gz"
        );
    }

    #[test]
    fn api_url_comes_from_the_repository() {
        assert_eq!(
            latest_api_url("https://github.com/o/r").unwrap(),
            "https://api.github.com/repos/o/r/releases/latest"
        );
        assert!(latest_api_url("https://example.com/o/r").is_err());
        assert!(latest_api_url("https://github.com/o").is_err());
    }

    #[test]
    fn sha256_file_is_parsed() {
        let h = "a".repeat(64);
        assert_eq!(
            parse_sha256_file(&format!("{h}  f.tar.gz\n"), "f.tar.gz").unwrap(),
            h
        );
        assert_eq!(
            parse_sha256_file(&format!("{h} *f.tar.gz"), "f.tar.gz").unwrap(),
            h
        );
        assert!(parse_sha256_file(&format!("{h}  other"), "f.tar.gz").is_err());
        assert!(parse_sha256_file("xyz  f.tar.gz", "f.tar.gz").is_err());
        assert!(parse_sha256_file("", "f.tar.gz").is_err());
    }

    /// cargo-binstall の宣言（`crates/app/cli/Cargo.toml`）と、ここで作る名前・取得先が一致する。
    #[test]
    fn matches_the_binstall_declaration() {
        let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../cli/Cargo.toml");
        let text = std::fs::read_to_string(path).unwrap();
        let v = Version::parse("1.2.3").unwrap();
        let mut checked = 0;
        let mut platform = None;
        for line in text.lines() {
            if line.starts_with('[') {
                platform = if line.contains("linux") {
                    Some(Platform::LinuxX86_64)
                } else if line.contains("apple-darwin") {
                    Some(Platform::MacosUniversal)
                } else {
                    None
                };
            }
            let Some(p) = platform else { continue };
            let Some((key, value)) = line.split_once(" = ") else {
                continue;
            };
            let value = value.trim().trim_matches('"');
            let fill = |s: &str| {
                s.replace("{ repo }", "REPO")
                    .replace("{ version }", &v.to_string())
                    .replace("{ bin }", "fune")
                    .replace("{ binary-ext }", "")
            };
            match key {
                "pkg-url" => {
                    let expected = download_url("REPO", v, &archive_name(v, p));
                    assert_eq!(fill(value), expected);
                    checked += 1;
                }
                "bin-dir" => {
                    assert_eq!(fill(value), format!("{}/fune", dir_name(v, p)));
                    checked += 1;
                }
                _ => {}
            }
        }
        assert_eq!(checked, 6, "expected pkg-url and bin-dir for 3 targets");
    }
}
