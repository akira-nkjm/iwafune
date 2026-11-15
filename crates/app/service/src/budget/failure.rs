//! バジェットが「失敗」かどうかの判定（`fune budget` の終了コード）。HTTP は判定を `BudgetView` のまま返すので、この関数は CLI が使う。

use crate::dto::{BudgetPropellant, BudgetView};
use sat_analysis::spec::Verdict;

/// バジェットが失敗か: 入力が足りない、合計が上限に対して違反（質量は乾燥）、または推薬込みの合計が違反。
/// 警告は失敗にしない。`--json` でも表でも同じ判定を使う。
pub fn fails(b: &BudgetView) -> bool {
    let wet = match &b.propellant {
        Some(BudgetPropellant::Counted { wet, .. }) => wet.verdict,
        _ => None,
    };
    !b.missing.is_empty()
        || b.verdict == Some(Verdict::Violation)
        || wet == Some(Verdict::Violation)
}
