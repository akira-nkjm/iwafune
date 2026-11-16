//! CLI コマンドの共有型・表示・引数の補助。
use crate::args::{EditOpts, PointArgs};
use fune_service::dto::{Op, Origin, Pose, ValueView};
use fune_service::{edit, query};
/// 読み取りコマンドを処理したときの終了コード。
pub(crate) type Handled = anyhow::Result<Option<u8>>;
/// 引数から作った Op と編集のオプション（読み取りのコマンドなら None）。
pub type Built<'a> = anyhow::Result<Option<(Op, &'a EditOpts)>>;

/// 任意の旗のカンマ区切りの一覧（読み方は `fune_service::edit::list_from_text`）。
pub(crate) fn list(text: &Option<String>) -> Vec<String> {
    text.as_deref()
        .map(edit::list_from_text)
        .unwrap_or_default()
}

/// 未設定の値の文言（service は `ValueData::Unset` と `display: null` の構造で返す）。
pub(crate) const UNSET: &str = "not set";

/// 値の表示文字列（未設定なら [`UNSET`]）。
pub(crate) fn value_text(display: Option<&str>) -> String {
    display.unwrap_or(UNSET).to_owned()
}

/// 値の出所・出典・根拠（表示名は組み込みの語彙の表。GUI と同じ）。
pub(crate) fn provenance(v: &ValueView) -> String {
    let mut parts = vec![query::enum_label("origin", &v.origin)];
    parts.extend(v.source.clone());
    if v.origin != Origin::Unset {
        parts.push(query::enum_label("evidence", &v.evidence));
    }
    parts.join(", ")
}

/// `fune part point PART POINT [--position …] [--orientation …] | --rm` → `SetMountPoint`
pub(crate) fn point_op(p: &PointArgs) -> anyhow::Result<Op> {
    let pose = if p.rm {
        None
    } else {
        Some(Pose {
            position: p
                .position
                .as_deref()
                .map(edit::vector_from_text)
                .transpose()?,
            orientation: p
                .orientation
                .as_deref()
                .map(edit::quaternion_from_text)
                .transpose()?,
        })
    };
    Ok(Op::SetMountPoint {
        part: p.part.clone(),
        point: p.point.clone(),
        pose,
    })
}
