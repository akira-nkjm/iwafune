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
            "Link",
            Verdict::Violation,
            "Does not meet the required link margin even at zenith",
        ));
    } else {
        // 閉じる局が 1 つでもあれば全体は続くが、閉じない局は数えていないことを知らせる。
        for c in station_closures {
            if c.closure == DataThroughputLinkClosure::NeverCloses {
                out.push(Check::new(
                    &format!("link_closes:{}", c.station),
                    &format!("Link ({})", c.station),
                    Verdict::Warning,
                    format!(
                        "{} does not meet the required link margin even at zenith, so it is not counted in the downlink volume",
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
        "Downlink volume and generated data",
        generated,
        capacity_per_day,
    )?);
    if let Some(cap) = input.storage_capacity {
        out.extend(at_most(
            "storage",
            "Backlog and storage",
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
    let counted = input.downlink.as_deref().unwrap_or("first downlink");
    Some(Check::new(
        "downlinks_not_counted",
        "Downlinks not counted",
        Verdict::Ok,
        format!(
            "Counted downlink: {counted}. Downlinks not counted: {}. The design data cannot yet say which link carries mission data, so the first downlink in name order is used (links are not summed)",
            input.other_downlinks.join(", ")
        ),
    ))
}
