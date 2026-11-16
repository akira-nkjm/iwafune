//! モードごとのデータレートを共通の台数評価器で求める。

use super::*;
use crate::modes::{UnlistedParts, evaluate};

/// 部品のレートの種類（HK とミッションデータ）。
type Pick = fn(&DataBudgetPart) -> &StateValues<DataRate>;

/// 部品の [HK, ミッション] のレート（不足は None）。
pub type ModePartRates = Vec<[Option<f64>; 2]>;

/// モード中の部品ごとの [HK, ミッション] の平均レート [bit/s]（係数を掛ける前）と、合計から除いた部品の警告。
/// 平均と除外の扱いは `modes::evaluate`。レートを持たない部品は渡さないので、モードにはあるがレートの一覧に
/// 無い部品は C&DH では正常として警告にしない（[`UnlistedParts::Ignore`]）。
pub fn mode_part_rates(
    mode: &ModeDef,
    parts: &[DataBudgetPart],
) -> Result<(ModePartRates, Vec<Check>), AnalysisError> {
    let kinds: [(&str, &str, Pick); 2] = [
        ("hk", "HK rate", |p| &p.hk_rate),
        ("mission", "Mission data rate", |p| &p.mission_rate),
    ];
    let mut rates = vec![[Some(0.0); 2]; parts.len()];
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
            if let Some(i) = parts.iter().position(|p| &p.id == id) {
                rates[i][k] = *mean;
            }
        }
        checks.extend(ev.checks(label).into_iter().map(|mut c| {
            c.key = format!("{key}:{}:{}", c.key, mode.id);
            c
        }));
    }
    Ok((rates, checks))
}

/// データ量の集計用レート。不足の寄与は従来どおり除外し、警告で示す。
pub(super) fn mode_rates(
    mode: &ModeDef,
    parts: &[DataBudgetPart],
) -> Result<(Vec<[f64; 2]>, Vec<Check>), AnalysisError> {
    let (rates, checks) = mode_part_rates(mode, parts)?;
    Ok((
        rates
            .into_iter()
            .map(|r| r.map(|v| v.unwrap_or(0.0)))
            .collect(),
        checks,
    ))
}
