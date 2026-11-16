//! ディレクトリの下に置けるエンティティ（カタログ品・出典）の Op に共通の計画
//! （新しいファイルのディレクトリを選ぶ・別のディレクトリへ移す）。ディレクトリ名の読み方とパスの組み立ては
//! `fune_store::layout`（`parse_subdir`・`file_in_dir`）。

use crate::error::OpError;
use fune_store::Edits;
use fune_store::layout::{dirs_text, file_in_dir, parse_subdir, split_below};

/// 人が打ったディレクトリ（省けば基準の直下）→ ディレクトリ名の列。
pub(super) fn parse_dir(dir: Option<&str>) -> Result<Vec<String>, OpError> {
    match dir {
        Some(text) => parse_subdir(text).map_err(OpError::invalid),
        None => Ok(Vec::new()),
    }
}

/// 新しいファイルのパス（`<base>/<dir…>/<id>.toml`）。`dir` を省けば基準の直下。
pub(super) fn new_file(base: &str, dir: Option<&str>, id: &str) -> Result<String, OpError> {
    Ok(file_in_dir(base, &parse_dir(dir)?, id))
}

/// 書き込むファイルのパス。既にある（`current`）ならそこ、無ければ [`new_file`]。
/// 既にあるものに `dir` が今の場所と違うディレクトリを指定したら拒む（移すのは `mv_hint` のコマンド）。
pub(super) fn placed_file(
    base: &str,
    current: Option<&str>,
    dir: Option<&str>,
    id: &str,
    (label, mv_hint): (&str, &str),
) -> Result<String, OpError> {
    let Some(current) = current else {
        return new_file(base, dir, id);
    };
    let wanted = new_file(base, dir, id)?;
    if dir.is_some() && wanted != current {
        let here = current
            .strip_suffix(".toml")
            .and_then(|stem| split_below(stem, base))
            .map_or_else(|| dirs_text(&[]), |(dirs, _)| dirs_text(&dirs));
        return Err(OpError::invalid(format!(
            "{label} already exists in `{here}`, so it is not placed in `{}` (move it with `{mv_hint}`)",
            dirs_text(&parse_dir(dir)?)
        )));
    }
    Ok(current.to_string())
}

/// 今のファイル（`file`）を別のディレクトリ（人が打った `dir`）へ移す。今いるディレクトリなら拒む。
pub(super) fn move_file(
    edits: &mut Edits,
    label: &str,
    file: &str,
    base: &str,
    id: &str,
    dir: &str,
) -> Result<(), OpError> {
    let target = file_in_dir(base, &parse_dir(Some(dir))?, id);
    if target == file {
        return Err(OpError::invalid(format!("{label} is already in `{dir}`")));
    }
    Ok(edits.rename(file, &target)?)
}
