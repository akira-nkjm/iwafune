//! 取付の付け替えで形態の上書きが古い取付点に対するものになるときの警告（`configuration.mount_changed`）。
//! 形態の `[pose.<部品>]` は部品の取付点に対する姿勢なので、基準の取付先・取付点を `Place` で変えると、上書きの意味が変わる。
//! 検査（`check`）は適用後の設計だけを見て付け替えを知らないので、計画が Op の前後を見て警告を足す。

use crate::op::Op;
use fune_model::design::EntityKey;
use fune_model::target::Target;
use fune_model::{Design, Diagnostic};

/// 付け替えられた取付と、その部品に上書きを持つ形態。
pub(super) struct Remount {
    part: String,
    from: String,
    to: String,
    configurations: Vec<String>,
}

/// 取付の表示（`BUS` か `BUS.PY_SLOT`。取付が無ければ `none`）。
fn show(on: Option<&str>, point: Option<&str>) -> String {
    match (on, point) {
        (Some(on), Some(point)) => format!("{on}.{point}"),
        (Some(on), None) => on.to_owned(),
        _ => "none".to_owned(),
    }
}

/// `op` を適用する前の設計 `design` で、`op` が取付を付け替え、その部品に上書きを持つ形態があれば返す。
pub(super) fn remount_of(design: &Design, op: &Op) -> Option<Remount> {
    let Op::Place {
        part,
        on,
        point,
        configuration: None,
        ..
    } = op
    else {
        return None;
    };
    let now = design.parts.get(part.as_str())?.mount.as_ref();
    let was = (
        now.map(|m| m.on.as_str()),
        now.and_then(|m| m.point.as_deref()),
    );
    let next = (on.as_deref(), on.as_ref().and(point.as_deref()));
    if was == next {
        return None;
    }
    let configurations: Vec<String> = design
        .configurations
        .values()
        .filter(|c| c.pose.contains_key(part.as_str()))
        .map(|c| c.id.to_string())
        .collect();
    (!configurations.is_empty()).then(|| Remount {
        part: part.clone(),
        from: show(was.0, was.1),
        to: show(next.0, next.1),
        configurations,
    })
}

/// 警告の診断（適用後の設計 `design` で位置を付ける）。形態ごとに 1 件。
pub(super) fn diagnostics(design: &Design, remounts: &[Remount]) -> Vec<Diagnostic> {
    let mut out = Vec::new();
    for r in remounts {
        for configuration in &r.configurations {
            let field = format!("pose.{}", r.part);
            let loc = design.source_map.locate(
                &EntityKey::Configuration(configuration.clone()),
                &["pose", r.part.as_str()],
            );
            let d = Diagnostic::warning(
                "configuration.mount_changed",
                format!(
                    "The mount of part `{}` changed from `{}` to `{}`, but configuration `{configuration}` still overrides its pose, which is now relative to the new mount. Check or remove the override",
                    r.part, r.from, r.to
                ),
            )
            .with_target(Target::configuration(configuration.as_str(), Some(&field)))
            .at(loc);
            out.push(d);
        }
    }
    out
}
