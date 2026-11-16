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
            "DoD limit",
            Verdict::from_ok(dod_ok),
            format!(
                "Max DoD {} (limit {}, {basis})",
                quantity_text::<Fraction>(j.max_dod),
                quantity_text::<Fraction>(limit)
            ),
        ),
        energy_balance_check(j, basis),
    ];
    if j.depleted {
        checks.push(Check::new(
            "depleted",
            "Battery depleted",
            Verdict::Violation,
            format!(
                "The battery runs empty and the load cannot be supplied for some time ({basis})"
            ),
        ));
    }
    checks
}

fn energy_balance_check(j: &Judged, basis: &str) -> Check {
    let (verdict, message) = match j.margin {
        Some(margin) => (
            Verdict::from_ok(margin.value() >= 0.0),
            format!(
                "Energy margin per orbit {} (including battery efficiency, {basis})",
                quantity_text::<Energy>(margin.value())
            ),
        ),
        None => (
            Verdict::Warning,
            format!(
                "The analysis span is shorter than one orbit, so the per-orbit margin cannot be judged{}",
                j.margin_over_span
                    .map_or_else(String::new, |margin| format!(
                        " (balance over the span is {}, including battery efficiency)",
                        quantity_text::<Energy>(margin.value())
                    ))
            ),
        ),
    };
    Check::new("energy_balance", "Orbital energy balance", verdict, message)
}
