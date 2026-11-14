//! `comm.data_throughput` の判定（回線が閉じるか・数えない下り回線・送信可能量・ストレージ）。

use super::{DataThroughputInput, DataThroughputLinkClosure, DataThroughputStationClosure};
use crate::budget::{BudgetItem, limit_check};
use crate::spec::{AnalysisError, Check, Verdict};
use sat_units::typed::Information;

pub(super) fn checks(
    input: &DataThroughputInput,
    closure: DataThroughputLinkClosure,
    station_closures: &[DataThroughputStationClosure],
    capacity_per_day: f64,
    max_backlog: f64,
) -> Result<Vec<Check>, AnalysisError> {
    let mut out = Vec::new();
    if closure == DataThroughputLinkClosure::NeverCloses {
        out.push(Check::new(
            "link_closes",
            "回線",
            Verdict::Violation,
            "天頂でも要求の回線余裕を満たしません",
        ));
    } else {
        // 閉じる局が 1 つでもあれば全体は続くが、閉じない局は数えていないことを知らせる。
        for c in station_closures {
            if c.closure == DataThroughputLinkClosure::NeverCloses {
                out.push(Check::new(
                    &format!("link_closes:{}", c.station),
                    &format!("回線（{}）", c.station),
                    Verdict::Warning,
                    format!(
                        "{} では天頂でも要求の回線余裕を満たさないので、送信量に数えていません",
                        c.station
                    ),
                ));
            }
        }
    }
    out.extend(other_downlinks_check(input));
    // `value ≤ limit` の判定はバジェットの判定（`budget::limit_check`）を 1 行で使う。
    let at_most = |key: &str, label: &str, value: f64, limit: f64| {
        let row = [BudgetItem::counted(key, value, 1)];
        limit_check::<Information>(&row, limit, key, label)
    };
    let generated = input.generated_per_day.value();
    out.extend(at_most(
        "capacity",
        "送信可能量と発生量",
        generated,
        capacity_per_day,
    )?);
    if let Some(cap) = input.storage_capacity {
        out.extend(at_most(
            "storage",
            "未送信データとストレージ",
            max_backlog,
            cap.value(),
        )?);
    }
    Ok(out)
}

/// 数えていない下り回線があれば事実として知らせる判定（Ok。管制用とミッションデータ用の下りが別にあるのは普通の設計。
/// どの回線がミッションデータを運ぶかを設計データがまだ言えないので、名前順の先頭の 1 本だけを数える）。
fn other_downlinks_check(input: &DataThroughputInput) -> Option<Check> {
    if input.other_downlinks.is_empty() {
        return None;
    }
    let counted = input.downlink.as_deref().unwrap_or("先頭の下り回線");
    Some(Check::new(
        "downlinks_not_counted",
        "送信量に数えない下り回線",
        Verdict::Ok,
        format!(
            "送信量に数えた下り回線: {counted}。数えていない下り回線: {}。どの回線がミッションデータを運ぶかを設計データがまだ言えないので、名前順の先頭の下り回線を使っています（足し合わせていません）",
            input.other_downlinks.join("、")
        ),
    ))
}
