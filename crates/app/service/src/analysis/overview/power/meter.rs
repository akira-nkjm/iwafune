//! 電源の概要の棒グラフのメーター: 限界・必要値に対する値の位置（0〜100）と補足の文。位置の計算はここだけ。
use crate::analysis::labels::label;
use crate::dto::{AnalysisLabel, QuantityView};
use sat_analysis::power::{energy_balance::EnergyBalanceCaseResult, sizing::SizingOutput};
use sat_analysis::spec::{Check, Verdict};
use sat_units::typed::{Area, Energy, Fraction, NamedKind, Qty};

/// 判定 1 つのメーター。位置はすべて 0〜100。
pub(super) struct Meter {
    pub label: AnalysisLabel,
    pub value: Option<QuantityView>,
    pub limit: QuantityView,
    pub verdict: Option<Verdict>,
    pub kind: LimitKind,
    /// 「上限の 143 %」「必要の 59 %」「8.70 Wh 不足」など
    pub note: AnalysisLabel,
    /// メーターの塗りの終わり（始まりは 0 で、限界が 0 の符号つきのメーターだけ 0 を中央に置く）
    pub fill_end: f64,
    /// 限界・必要値の線の位置
    pub limit_at: f64,
}

/// 限界の向き。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum LimitKind {
    /// 限界以下なら OK（DoD）
    AtMost,
    /// 必要値以上なら OK（面積・容量・収支）
    AtLeast,
}

fn check(checks: &[Check], key: &str) -> Option<Verdict> {
    checks.iter().find(|c| c.key == key).map(|c| c.verdict)
}

/// 電力収支のケース 1 つの最大 DoD と上限。値は判定と同じ運用の値（最長継続時間で評価したときはその値）。
pub(super) fn dod_bar(case: &EnergyBalanceCaseResult, limit: Fraction) -> Meter {
    verdict_view::<Fraction>(
        label("最大 DoD", "Maximum DoD"),
        Some(
            case.by_max_duration
                .as_ref()
                .map_or(case.max_dod.value(), |d| d.max_dod_from_start.value()),
        ),
        limit.value(),
        LimitKind::AtMost,
        check(&case.checks, "dod"),
    )
}

/// 太陽電池の面積の今と必要（`required`）。判定は `checks` の `sap_margin`。
pub(super) fn area_bar(
    out: &SizingOutput,
    required: Qty<Area>,
    checks: Option<&Vec<Check>>,
) -> Meter {
    verdict_view::<Area>(
        label("太陽電池の面積", "Solar Cells area"),
        Some(out.design_area.value()),
        required.value(),
        LimitKind::AtLeast,
        checks.and_then(|c| check(c, "sap_margin")),
    )
}

/// 電池容量の今と必要（`required`）。判定は `checks` の `battery_margin`。
pub(super) fn capacity_bar(
    out: &SizingOutput,
    required: Qty<Energy>,
    checks: Option<&Vec<Check>>,
) -> Meter {
    verdict_view::<Energy>(
        label("電池容量", "Battery capacity"),
        Some(out.design_capacity.value()),
        required.value(),
        LimitKind::AtLeast,
        checks.and_then(|c| check(c, "battery_margin")),
    )
}

/// メーターの右端（限界・必要値の何倍まで描くか）。限界の線はその 1/1.5 の位置になる。
const METER_SPAN: f64 = 1.5;

/// 判定 1 つ。限界が 0 でない量はメーターを「限界に対する割合」（0〜150 %）で描き、
/// 限界が 0 の量（1 周の収支など）は 0 を中央に置いた符号つきのメーターにする。
pub(super) fn verdict_view<K: NamedKind>(
    name: AnalysisLabel,
    value: Option<f64>,
    limit: f64,
    kind: LimitKind,
    verdict: Option<Verdict>,
) -> Meter {
    let fmt = |v: f64| crate::units::quantity_auto(v, K::name()).display;
    let pct = |r: f64| crate::units::quantity_auto(r, Fraction::name()).display;
    let (fill_end, limit_at, note) = match value {
        None => (0.0, 100.0 / METER_SPAN, label("—", "—")),
        Some(v) if limit.abs() > f64::EPSILON => {
            let r = v / limit;
            let end = (r / METER_SPAN).clamp(0.0, 1.0) * 100.0;
            let note = match kind {
                LimitKind::AtMost => {
                    label(format!("上限の {}", pct(r)), format!("{} of limit", pct(r)))
                }
                LimitKind::AtLeast => label(
                    format!("必要の {}", pct(r)),
                    format!("{} of required", pct(r)),
                ),
            };
            (end, 100.0 / METER_SPAN, note)
        }
        Some(v) => {
            // 限界が 0: 0 を中央に、|v| の 1.15 倍を端にする
            let span = (v.abs() * 1.15).max(f64::EPSILON);
            let pos = (50.0 + v / span * 50.0).clamp(0.0, 100.0);
            let note = if v < 0.0 {
                label(format!("{} 不足", fmt(-v)), format!("short by {}", fmt(-v)))
            } else {
                label(format!("{} の余裕", fmt(v)), format!("{} to spare", fmt(v)))
            };
            (pos.max(50.0), 50.0, note)
        }
    };
    Meter {
        label: name,
        value: value.map(|v| crate::units::quantity_auto(v, K::name())),
        limit: crate::units::quantity_auto(limit, K::name()),
        verdict,
        kind,
        note,
        fill_end,
        limit_at,
    }
}

#[cfg(test)]
mod tests;
