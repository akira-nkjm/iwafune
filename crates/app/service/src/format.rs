//! 設計ファイルを store の TOML プロファイルで整形する共通操作。
use crate::dto::{FormatDesignRequest, FormatDesignResult};
use crate::error::Result;
use crate::workspace::{Workspace, store_error};
use std::time::Duration;

/// check 時は変更予定のみ返す。書き込みはロック内で読み直してまとめて反映する。
pub fn format_design(ws: &Workspace, req: &FormatDesignRequest) -> Result<FormatDesignResult> {
    let _lock = if req.check {
        None
    } else {
        Some(
            fune_store::lock::ProjectLock::acquire(ws.root(), Duration::ZERO)
                .map_err(store_error)?,
        )
    };
    let files = fune_store::read::scan(ws.root()).map_err(store_error)?;
    // Parse every TOML first: format_files must never silently skip an invalid file here.
    for (path, text) in files.iter().filter(|(p, _)| p.ends_with(".toml")) {
        fune_store::profile::parse(path, text).map_err(store_error)?;
    }
    let changes = fune_store::format::format_files(&files);
    if !req.check {
        fune_store::write::commit(ws.root(), &changes).map_err(store_error)?;
    }
    Ok(FormatDesignResult {
        changed: changes.into_iter().map(|c| c.path).collect(),
        check: req.check,
    })
}
