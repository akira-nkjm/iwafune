//! `fune skill`: AI エージェント向けの skill（`SKILL.md`）の本文と、その置き場への書き込み。**CLI だけの操作**
//! （呼び出した人のエージェント用ディレクトリに書くので HTTP には出さない。設計ファイルへの書き込みではないので Op も通さない）。
//!
//! 本文は `skill/SKILL.md`（**本文の置き場はここだけ**）を `include_str!` で埋め込む。`{{FUNE_VERSION}}` は
//! 描画のときに実行中の `fune` の版に置き換わり、先頭の `metadata.fune-version` が「どの版の本文か」の印になる。
//! 書き込みの扱い: 無ければ作る / 同じなら何もしない / 別の版の印があれば置き換える / それ以外の違い（手で直した）は
//! `force` でなければ断る。

use crate::error::{Result, ServiceError};
use serde::Serialize;
use std::path::{Path, PathBuf};

/// 埋め込んだ本文（版を埋める前）。
const TEMPLATE: &str = include_str!("../skill/SKILL.md");
/// 版を埋める場所。
const VERSION_SLOT: &str = "{{FUNE_VERSION}}";
/// 版の印の行（フロントマターの `metadata:` の下）の頭。
const VERSION_KEY: &str = "  fune-version:";
/// skill のディレクトリ名（`<skills>/fune/SKILL.md`）。
const SKILL_NAME: &str = "fune";
/// プロジェクトの中の既定の skill 置き場（Agent Skills の配置）。
const PROJECT_SKILLS_DIR: &str = ".claude/skills";

/// 実行中の `fune` の版（本文に埋める）。
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

/// 本文（版を埋めたもの）。`fune skill show` が出し、`install` が書く。
pub fn render() -> String {
    TEMPLATE.replace(VERSION_SLOT, VERSION)
}

/// 入れた結果の種類。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum InstallStatus {
    /// 無かったので作った
    Created,
    /// すでに同じ本文があった（何も書いていない）
    UpToDate,
    /// 別の版の本文だったので置き換えた
    Updated,
    /// 手で直したファイルを `force` で上書きした
    Overwritten,
}

/// `fune skill install` の結果。
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct InstallOutcome {
    pub status: InstallStatus,
    /// 書いた（または確かめた）`SKILL.md`
    pub path: String,
    /// 本文の版（実行中の `fune` の版）
    pub version: String,
    /// 置き換える前の版（`Updated` のとき。印が読めなかったら `None`）
    pub previous_version: Option<String>,
}

/// 既定の skill 置き場。プロジェクトの中なら根の `.claude/skills`、プロジェクトの外なら `start` の `.claude/skills`。
pub fn default_skills_dir(start: &Path) -> PathBuf {
    let base = fune_store::project::find_root(start).unwrap_or_else(|_| start.to_path_buf());
    base.join(PROJECT_SKILLS_DIR)
}

/// 本文の `metadata.fune-version` の値。フロントマターの外は見ない。
fn version_marker(text: &str) -> Option<String> {
    let body = text.strip_prefix("---\n")?;
    let (front, _) = body.split_once("\n---")?;
    front
        .lines()
        .find_map(|l| l.strip_prefix(VERSION_KEY))
        .map(|v| v.trim().trim_matches('"').to_owned())
        .filter(|v| !v.is_empty())
}

/// `skills_dir/fune/SKILL.md` に本文を入れる。`force` のときは手で直したファイルも上書きする。
pub fn install(skills_dir: &Path, force: bool) -> Result<InstallOutcome> {
    install_text(skills_dir, &render(), VERSION, force)
}

/// 本文と版を指定して入れる（テストは別の版を渡して、版が違う場合を確かめる）。
fn install_text(
    skills_dir: &Path,
    text: &str,
    version: &str,
    force: bool,
) -> Result<InstallOutcome> {
    let dir = skills_dir.join(SKILL_NAME);
    let path = dir.join("SKILL.md");
    let shown = path.display().to_string();
    let fail = |what: &str, e: std::io::Error| ServiceError::Skill(format!("{what} {shown}: {e}"));
    let outcome = |status, previous_version| InstallOutcome {
        status,
        path: shown.clone(),
        version: version.to_owned(),
        previous_version,
    };
    let existing = match std::fs::read_to_string(&path) {
        Ok(t) => Some(t),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => None,
        Err(e) => return Err(fail("Cannot read", e)),
    };
    let (status, previous) = match existing {
        None => (InstallStatus::Created, None),
        Some(t) if t == text => return Ok(outcome(InstallStatus::UpToDate, None)),
        Some(t) => match version_marker(&t) {
            Some(v) if v != version => (InstallStatus::Updated, Some(v)),
            _ if force => (InstallStatus::Overwritten, version_marker(&t)),
            _ => {
                return Err(ServiceError::SkillModified {
                    path: shown,
                    version: version.to_owned(),
                });
            }
        },
    };
    std::fs::create_dir_all(&dir).map_err(|e| fail("Cannot create the directory of", e))?;
    std::fs::write(&path, text).map_err(|e| fail("Cannot write", e))?;
    Ok(outcome(status, previous))
}

#[cfg(test)]
mod tests;
