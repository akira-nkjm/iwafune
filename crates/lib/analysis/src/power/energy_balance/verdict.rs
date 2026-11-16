//! 電力収支の判定（DoD の上限・周回の収支・電池切れ）。連続・並び・最長継続時間のどの評価もここで判定する。

use crate::spec::{Check, Verdict, quantity_text};
use sat_units::typed::{Energy, Fraction, Qty};

/// DoD の上限との比較の許容（SOC の積分の丸めで上限ちょうどが超過にならないように）。
const DOD_TOLERANCE: f64 = 1e-12;

/// 判定に使う値（どの運用で評価したかによらない形）。
pub(super) struct Judged {
    pub max_dod: f64,
    /// 1 周あたりのエネルギー余裕（回り切った周回が無ければ None）
    pub margin: Option<Qty<Energy>>,
    /// 解析期間全体の収支（1 周に満たないときだけ）
    pub margin_over_span: Option<Qty<Energy>>,
    pub depleted: bool,
}

/// DoD の上限・周回の収支・電池切れの判定。`basis` はどの運用で評価したか（文の括弧に入れる）。
pub(super) fn verdicts(limit: f64, j: &Judged, basis: &str) -> Vec<Check> {
    let dod_ok = j.max_dod <= limit + DOD_TOLERANCE;
    let mut checks = vec![
        Check::new(
            "dod",
            "DoD の上限",
            Verdict::from_ok(dod_ok),
            format!(
                "最大 DoD {}（上限 {}、{basis}）",
                quantity_text::<Fraction>(j.max_dod),
                quantity_text::<Fraction>(limit)
            ),
        ),
        energy_balance_check(j, basis),
    ];
    if j.depleted {
        checks.push(Check::new(
            "depleted",
            "電池切れ",
            Verdict::Violation,
            format!("電池が空になり、負荷をまかなえない時間があります（{basis}）"),
        ));
    }
    checks
}

fn energy_balance_check(j: &Judged, basis: &str) -> Check {
    let (verdict, message) = match j.margin {
        Some(margin) => (
            Verdict::from_ok(margin.value() >= 0.0),
            format!(
                "1 周あたりのエネルギー余裕 {}（電池の効率込み、{basis}）",
                quantity_text::<Energy>(margin.value())
            ),
        ),
        None => (
            Verdict::Warning,
            format!(
                "解析期間が 1 周に満たないため 1 周あたりの余裕を判定できない{}",
                j.margin_over_span
                    .map_or_else(String::new, |margin| format!(
                        "（解析期間の収支は {}、電池の効率込み）",
                        quantity_text::<Energy>(margin.value())
                    ))
            ),
        ),
    };
    Check::new("energy_balance", "周回の電力収支", verdict, message)
}
