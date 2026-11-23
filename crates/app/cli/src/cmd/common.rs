//! CLI コマンドの共有型・表示・引数の補助。
use crate::args::{EditOpts, PointArgs};
use crate::output::{C, MARK_OK, MARK_VIOLATION, MARK_WARNING, Out};
use fune_service::dto::{Op, Origin, Pose, ValueView, Verdict};
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

/// 判定の印（✔ / ⚠ / ✖）。`Verdict` から印への対応はここだけで決める。
pub(crate) fn verdict_mark(v: Verdict) -> &'static str {
    match v {
        Verdict::Ok => MARK_OK,
        Verdict::Warning => MARK_WARNING,
        Verdict::Violation => MARK_VIOLATION,
    }
}

/// 判定の語（ok / warning / violation）。
fn verdict_word(v: Verdict) -> &'static str {
    match v {
        Verdict::Ok => "ok",
        Verdict::Warning => "warning",
        Verdict::Violation => "violation",
    }
}

/// 印だけのセル（KPI・バジェット）。違反は赤、警告は色なし、OK は緑。
pub(super) fn verdict_cell(v: Verdict) -> C {
    verdict_styled(v, verdict_mark(v).to_owned())
}

/// 印と語のセル（チェックの表）。
pub(super) fn verdict_word_cell(v: Verdict) -> C {
    verdict_styled(v, format!("{} {}", verdict_mark(v), verdict_word(v)))
}

/// 値に判定の印を添えた文字列（モードの表）。
pub(super) fn value_with_mark(display: &str, v: Verdict) -> String {
    format!("{display} {}", verdict_mark(v))
}

/// 判定に合わせた色のセル（警告は色を付けない）。
fn verdict_styled(v: Verdict, text: String) -> C {
    match v {
        Verdict::Ok => C::Verdict(text, true),
        Verdict::Warning => C::Text(text),
        Verdict::Violation => C::Verdict(text, false),
    }
}

/// 判定に合わせて 1 行を出す（印と色は `Out` の `ok` / `warn` / `fail`。判定が無ければ OK の色）。
pub(super) fn verdict_line(v: Option<Verdict>, line: String, out: &Out) {
    match v.unwrap_or(Verdict::Ok) {
        Verdict::Ok => out.ok(line),
        Verdict::Warning => out.warn(line),
        Verdict::Violation => out.fail(line),
    }
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
