//! プロジェクトを開く。読み取りのたびにディレクトリから読み直したスナップショット（Design・Resolved・診断）を作る。
//! 状態はディスクだけに置くので、CLI と GUI が同時に書いても食い違わない（数十ファイルで数 ms）。
//! fune-store・fune-ops のエラーをサービスのエラーに読み替える関数（`store_error`・`op_error`）もここに置く（edit・preview が共有）。

use crate::dto::{DiagnosticView, SourceExcerpt};
use crate::error::{Result, ServiceError};
use fune_model::{Design, Diagnostic, Resolved, Target};
use fune_store::FileSet;
use std::path::{Path, PathBuf};

/// 開いているプロジェクト。
#[derive(Debug, Clone)]
pub struct Workspace {
    root: PathBuf,
}

/// ある時点の設計（読み直したもの）。
#[derive(Debug, Clone)]
pub struct Snapshot {
    pub design: Design,
    pub resolved: Resolved,
    /// 読み込み時の問題と検証の結果（該当行の本文付き）
    pub diagnostics: Vec<DiagnosticView>,
    /// 読んだファイル（相対パス → 本文）
    pub files: FileSet,
}

impl Snapshot {
    /// 設計と、その元のファイルから作る（解決と、読み込み時の問題・検証の結果を付ける）。
    /// ディスクから読んだ設計にも、保存していない編集を当てた設計（[`crate::preview`]）にも使う。
    pub fn of(design: Design, files: FileSet) -> Self {
        let resolved = fune_model::resolve(&design);
        // `check` は読み込み時の問題も含めて返す（ここで足すと 2 回ずつ出る）
        // 暦を要る検査（日付つきの遷移の日付）は暦を持つ fune-projection の側（model は sat-astro に依存しない）
        let diagnostics = fune_model::check(&design)
            .into_iter()
            .chain(fune_projection::maneuver_dates::check(&design))
            .chain(fune_projection::outline::check(&design, &resolved))
            .chain(fune_projection::thermal::check(&design, &resolved))
            .chain(fune_projection::propulsion::tank_refs::check(
                &design, &resolved,
            ))
            .chain(crate::stale_acknowledgements::stale(&design, &resolved))
            .map(|d| with_excerpt(d, &files))
            .collect();
        Self {
            design,
            resolved,
            diagnostics,
            files,
        }
    }
}

impl Workspace {
    /// `dir` かその親から fune.toml を探して開く。外部データの初期化は data::install_cached_datasets が担当する。
    pub fn open(dir: impl AsRef<Path>) -> Result<Self> {
        let dir = dir.as_ref();
        let root = fune_store::project::find_root(dir)
            .map_err(|_| ServiceError::ProjectNotFound(dir.display().to_string()))?;
        Ok(Self { root })
    }

    /// プロジェクトのルート。
    pub fn root(&self) -> &Path {
        &self.root
    }

    /// ディスクから読み直す。
    pub fn snapshot(&self) -> Result<Snapshot> {
        let fune_store::Snapshot { design, files, .. } =
            fune_store::load(&self.root).map_err(store_error)?;
        Ok(Snapshot::of(design, files))
    }

    /// git のブランチ名（`fune_store::project::git_branch`。git でなければ None）。
    pub fn git_branch(&self) -> Option<String> {
        fune_store::project::git_branch(&self.root)
    }
}

/// 店（store）のエラー → サービスのエラー。ロックと「もうある」（`fune init` の既存のプロジェクトなど）は競合に分ける。
pub(crate) fn store_error(e: fune_store::StoreError) -> ServiceError {
    use fune_store::StoreError;
    match e {
        StoreError::NotAProject { start } => ServiceError::ProjectNotFound(start),
        StoreError::Locked { lock } => ServiceError::Locked(lock),
        StoreError::AlreadyExists { path } => ServiceError::AlreadyExists { what: path },
        // 打たれたパスの誤り（入力の誤り。サーバーの故障ではない）
        e @ StoreError::SettingPath { .. } => ServiceError::InvalidInput(e.to_string()),
        other => ServiceError::Store(other.to_string()),
    }
}

/// fune-ops のエラー → サービスのエラー（保存・保存しない編集のプレビュー・打った文字列の読み取りで共通）。
pub(crate) fn op_error(ws: &Workspace, e: fune_ops::OpError) -> ServiceError {
    use fune_ops::OpError;
    match e {
        OpError::Conflict { expected, actual } => ServiceError::Conflict {
            expected: expected.0,
            actual: actual.0,
        },
        OpError::NothingToStep { redo } => ServiceError::NothingToStep {
            action: if redo { "redo" } else { "undo" },
        },
        OpError::HistoryMismatch {
            redo,
            expected,
            actual,
        } => ServiceError::HistoryConflict {
            action: if redo { "redo" } else { "undo" },
            expected: expected.0,
            actual: actual.0,
        },
        OpError::NotFound { what } => ServiceError::NotFound {
            what: "target",
            id: what,
        },
        OpError::AlreadyExists { what } => ServiceError::AlreadyExists { what },
        OpError::Invalid { message } => ServiceError::InvalidInput(message),
        OpError::Rejected { diagnostics } => {
            // 書き込まなかったので、該当行は今のファイルから取る（変えていないファイルの行は同じ）。
            let files = fune_store::read::scan(ws.root()).unwrap_or_default();
            ServiceError::Validation {
                diagnostics: diagnostics
                    .into_iter()
                    .map(|d| with_excerpt(d, &files))
                    .collect(),
            }
        }
        OpError::Store(e) => store_error(e),
    }
}

/// 診断に、対象の部品・表示用のパス（どちらも対象の構造から）と、該当行の前後 2 行の本文を添える。
pub(crate) fn with_excerpt(d: Diagnostic, files: &FileSet) -> DiagnosticView {
    let excerpt = match (d.file.as_deref(), d.line) {
        (Some(f), Some(line)) => files.get(f).and_then(|text| excerpt(text, line)),
        _ => None,
    };
    DiagnosticView {
        part: d.target.as_ref().and_then(Target::part).map(str::to_owned),
        path: d.path(),
        diagnostic: d,
        excerpt,
    }
}

fn excerpt(text: &str, line: u32) -> Option<SourceExcerpt> {
    let first = line.saturating_sub(2).max(1);
    let lines: Vec<&str> = text
        .lines()
        .skip(usize::try_from(first - 1).ok()?)
        .take(5)
        .collect();
    (!lines.is_empty()).then(|| SourceExcerpt {
        first_line: first,
        text: lines.join("\n") + "\n",
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    use fune_model::path::{AttrPath, SettingPath};

    /// 部品 ID は対象の構造から取る（設定・カタログ品・モードの行・ケースには付けない）。
    #[test]
    fn diagnostic_part_comes_from_the_target() {
        let not_parts: [Target; 4] = [
            SettingPath::analysis(&["power", "dod_max"]).into(),
            Target::Catalog {
                id: "local/rw".into(),
                attr: AttrPath::parse("power.on").ok(),
            },
            Target::mode("NOMINAL", Some("TRX")),
            Target::case("HOT", Some("beta")),
        ];
        for target in not_parts {
            let d = Diagnostic::warning("test", "test").with_target(target);
            assert_eq!(with_excerpt(d, &FileSet::new()).part, None);
        }
        let d = Diagnostic::warning("test", "test").with_target(Target::attr("RW_X", "power"));
        let view = with_excerpt(d, &FileSet::new());
        assert_eq!(view.part.as_deref(), Some("RW_X"));
        assert_eq!(view.path.as_deref(), Some("RW_X.power"));
        // 対象の無い診断は、ファイルの場所から部品を推さない
        let mut d = Diagnostic::warning("test", "test");
        d.file = Some("parts/aocs/RW_X.toml".into());
        assert_eq!(with_excerpt(d, &FileSet::new()).part, None);
    }

    /// 読み込み時の問題は 1 回だけ出る（`check` が含めて返す）。
    #[test]
    fn load_diagnostics_are_listed_once() {
        let mut design = Design::default();
        let d =
            Diagnostic::error("toml.syntax", "test").with_target(Target::Part { id: "X".into() });
        design.load_diagnostics.push(d);
        let snapshot = Snapshot::of(design, FileSet::new());
        let found = snapshot
            .diagnostics
            .iter()
            .filter(|d| d.diagnostic.code == "toml.syntax");
        assert_eq!(found.count(), 1);
    }
}
