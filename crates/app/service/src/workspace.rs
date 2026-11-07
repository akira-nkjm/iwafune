//! プロジェクトを開く。読み取りのたびにディレクトリから読み直したスナップショット（Design・Resolved・診断）を作る。
//! 状態はディスクだけに置くので、CLI と GUI が同時に書いても食い違わない（数十ファイルで数 ms）。

use crate::dto::{DiagnosticView, SourceExcerpt};
use crate::error::{Result, ServiceError};
use fune_model::{Design, Diagnostic, Resolved};
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
        let mut found: Vec<Diagnostic> = design.load_diagnostics.clone();
        found.extend(fune_model::check(&design));
        let diagnostics = found.into_iter().map(|d| with_excerpt(d, &files)).collect();
        Self {
            design,
            resolved,
            diagnostics,
            files,
        }
    }
}

impl Workspace {
    /// `dir` かその親から fune.toml を探して開く。外部データの初期化は data::install_cached_ephemeris が担当する。
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

/// 店（store）のエラー → サービスのエラー。
pub(crate) fn store_error(e: fune_store::StoreError) -> ServiceError {
    match e {
        fune_store::StoreError::NotAProject { start } => ServiceError::ProjectNotFound(start),
        other => ServiceError::Store(other.to_string()),
    }
}

/// 診断に該当行の前後 2 行の本文を添える。
pub(crate) fn with_excerpt(d: Diagnostic, files: &FileSet) -> DiagnosticView {
    let excerpt = match (d.file.as_deref(), d.line) {
        (Some(f), Some(line)) => files.get(f).and_then(|text| excerpt(text, line)),
        _ => None,
    };
    let part = d
        .path
        .as_deref()
        .and_then(|path| match fune_model::path::parse(path).ok()? {
            fune_model::path::ParamPath::Attr { part, .. }
            | fune_model::path::ParamPath::Field { part, .. }
                if !part.contains('/')
                    && d.file.as_deref().is_none_or(|f| f.starts_with("parts/")) =>
            {
                Some(part)
            }
            _ => None,
        })
        .or_else(|| {
            d.file
                .as_deref()
                .filter(|f| f.starts_with("parts/") && f.ends_with(".toml"))
                .and_then(|file| {
                    std::path::Path::new(file)
                        .file_stem()?
                        .to_str()
                        .map(str::to_owned)
                })
        });
    DiagnosticView {
        part,
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

    #[test]
    fn diagnostic_links_exclude_settings_and_catalog_references() {
        for path in [
            "analysis.power.dod_max",
            "local/rw.power.on",
            "mission.orbit.altitude",
        ] {
            let d = Diagnostic::warning("test", "test").with_path(path);
            assert_eq!(with_excerpt(d, &FileSet::new()).part, None);
        }
        let d = Diagnostic::warning("test", "test").with_path("RW_X.power.on");
        assert_eq!(
            with_excerpt(d, &FileSet::new()).part.as_deref(),
            Some("RW_X")
        );
        let mut d = Diagnostic::warning("test", "test");
        d.file = Some("parts/AOCS/RW_X.toml".into());
        assert_eq!(
            with_excerpt(d, &FileSet::new()).part.as_deref(),
            Some("RW_X")
        );
    }
}
