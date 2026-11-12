//! モードごとのデータレートを共通の台数評価器で求める。

use super::*;
use crate::modes::{UnlistedParts, evaluate};

/// 部品のレートの種類（HK とミッションデータ）。
type Pick = fn(&DataBudgetPart) -> &StateValues<DataRate>;

/// モード中の部品ごとの [HK, ミッション] の平均レート [bit/s]（係数を掛ける前）と、合計から除いた部品の警告。
/// 平均と除外の扱いは `modes::evaluate`。レートを持たない部品は渡さないので、モードにはあるがレートの一覧に
/// 無い部品は C&DH では正常として警告にしない（[`UnlistedParts::Ignore`]）。
pub(super) fn mode_rates(
    mode: &ModeDef,
    parts: &[DataBudgetPart],
) -> Result<(Vec<[f64; 2]>, Vec<Check>), AnalysisError> {
    let kinds: [(&str, &str, Pick); 2] = [
        ("hk", "HK レート", |p| &p.hk_rate),
        ("mission", "ミッションデータのレート", |p| {
            &p.mission_rate
        }),
    ];
    let mut rates = vec![[0.0; 2]; parts.len()];
    let mut checks = Vec::new();
    for (k, (key, label, pick)) in kinds.into_iter().enumerate() {
        let ids = parts
            .iter()
            .filter(|p| !pick(p).is_empty())
            .map(|p| p.id.as_str());
        let ev = evaluate(
            mode,
            ids,
            UnlistedParts::Ignore,
            |id| {
                parts
                    .iter()
                    .find(|p| p.id == id)
                    .map(DataBudgetPart::population)
                    .unwrap_or(crate::modes::Population {
                        count: 0,
                        redundancy: crate::modes::Redundancy::None,
                        states: &[],
                    })
            },
            |id, state| {
                parts
                    .iter()
                    .find(|p| p.id == id)
                    .and_then(|p| pick(p).get(state))
                    .map(Qty::value)
            },
        )?;
        for (id, mean) in &ev.means {
            if let (Some(i), Some(m)) = (parts.iter().position(|p| &p.id == id), mean) {
                rates[i][k] = *m;
            }
        }
        checks.extend(ev.checks(label).into_iter().map(|mut c| {
            c.key = format!("{key}:{}:{}", c.key, mode.id);
            c
        }));
    }
    Ok((rates, checks))
}
