//! バジェット（質量・電力・データ量）の集計と判定。**バジェットの判定はここだけで行う**（service は行を並べるだけ）。
//!
//! 約束:
//! - 個数 `count` はすべての示量（値）に掛ける: `拡張値 = value × count`。
//! - 成長余裕は 1 回だけ掛ける: `成長込み = value × count × (1 + growth)`。
//! - 判定: `成長込みの合計 ≤ limit × (1 − system_margin)` なら `ok`、超えれば `violation`。
//! - 値は SI の f64（質量 kg・電力 W・データ量 bit など）。何の量かは呼び出し側が知っている。
//! - 解析の中で個数 × 値を数えるだけ・上限と比べるだけのときは [`counted_total`]・[`limit_check`]
//!   （行は [`BudgetItem::counted`]）を使い、[`budget_rollup`] を呼ぶ定型を書かない。

use crate::spec::quantity_text;
use crate::spec::{AnalysisError, Check, Verdict, ensure_fraction, ensure_non_negative};
use sat_units::typed::{Fraction, NamedKind};
use serde::{Deserialize, Serialize};

/// バジェットの 1 行（部品など）。
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct BudgetItem {
    pub id: String,
    /// まとまり（例: `"AOCS"`）。小計に使う
    #[serde(default)]
    pub group: String,
    /// 1 個あたりの値（SI）
    pub value: f64,
    /// 個数（省略時 1）
    #[serde(default = "crate::spec::default_count")]
    pub count: u32,
    /// 成長余裕（0.2 = +20 %、省略時 0）
    #[serde(default)]
    pub growth: f64,
}

impl BudgetItem {
    /// まとまりも成長余裕も無い 1 行（個数 × 値を数えるだけの集計・上限の判定に使う）。
    pub fn counted(id: impl Into<String>, value: f64, count: u32) -> Self {
        Self {
            id: id.into(),
            group: String::new(),
            value,
            count,
            growth: 0.0,
        }
    }
}

/// Σ 値 × 個数（上限もシステム余裕も無い [`budget_rollup`]）。例: ストレージの容量・パネルに載る質量。
pub fn counted_total(items: &[BudgetItem]) -> Result<f64, AnalysisError> {
    Ok(budget_rollup(items, None, 0.0)?.total)
}

/// 成長余裕込みの合計 ≤ 上限 `limit` の判定（システム余裕なしの [`budget_rollup`] と [`BudgetRollup::check`]）。
/// 例: 総質量・1 日の送信可能量・未送信データとストレージ。値の次元は型 `K`（`limit_check::<Information>(…)`）。
pub fn limit_check<K: NamedKind>(
    items: &[BudgetItem],
    limit: f64,
    key: &str,
    label: &str,
) -> Result<Option<Check>, AnalysisError> {
    Ok(budget_rollup(items, Some(limit), 0.0)?.check::<K>(key, label))
}

/// 1 行の集計。
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct BudgetRollupRow {
    pub id: String,
    pub group: String,
    pub value: f64,
    pub count: u32,
    pub growth: f64,
    /// value × count
    pub total: f64,
    /// value × count × (1 + growth)
    pub total_with_growth: f64,
}

/// まとまりごとの小計。
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct BudgetGroup {
    pub group: String,
    pub total: f64,
    pub total_with_growth: f64,
    /// 個数の合計
    pub count: u32,
}

/// 集計と判定の結果。
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct BudgetRollup {
    /// Σ value × count
    pub total: f64,
    /// Σ value × count × (1 + growth)
    pub total_with_growth: f64,
    /// まとまりごとの小計（まとまりの名前順）
    pub by_group: Vec<BudgetGroup>,
    /// 行ごとの集計（入力の順）
    pub rows: Vec<BudgetRollupRow>,
    /// 上限（与えたとき）
    pub limit: Option<f64>,
    /// システム余裕（0〜1）
    pub system_margin: f64,
    /// 使ってよい量 = limit × (1 − system_margin)
    pub allowed: Option<f64>,
    /// 余裕 = allowed − total_with_growth（負なら超過）
    pub margin: Option<f64>,
    /// 判定（上限を与えたとき）
    pub verdict: Option<Verdict>,
}

impl BudgetRollup {
    /// 判定を `Check` にする（上限が無ければ `None`）。`key`・`label` と値の次元の型 `K` は呼び出し側が決める。
    pub fn check<K: NamedKind>(&self, key: &str, label: &str) -> Option<Check> {
        let (allowed, verdict, limit) = (self.allowed?, self.verdict?, self.limit?);
        Some(
            Check::new(
                key,
                label,
                verdict,
                format!(
                    "With growth margin {} / usable {} (limit {} × (1 − system margin {}))",
                    quantity_text::<K>(self.total_with_growth),
                    quantity_text::<K>(allowed),
                    quantity_text::<K>(limit),
                    quantity_text::<Fraction>(self.system_margin)
                ),
            )
            .with_margin(self.margin),
        )
    }
}

/// 行を集計し、上限 `limit`（SI、省略可）とシステム余裕 `system_margin`（0 以上 1 未満）で判定する。
pub fn budget_rollup(
    items: &[BudgetItem],
    limit: Option<f64>,
    system_margin: f64,
) -> Result<BudgetRollup, AnalysisError> {
    ensure_fraction("System margin", system_margin)?;
    // 1 だと使える量が 0 になる（設計の検査 `fune check` の budget.margin と同じ範囲）
    if system_margin >= 1.0 {
        return Err(AnalysisError::InvalidInput(
            "System margin must be less than 1".to_owned(),
        ));
    }
    if let Some(l) = limit {
        ensure_non_negative("Budget limit", l)?;
    }
    let mut rows = Vec::with_capacity(items.len());
    let mut by_group: Vec<BudgetGroup> = Vec::new();
    for it in items {
        if !it.value.is_finite() {
            return Err(AnalysisError::InvalidInput(format!(
                "Value of {} is not finite",
                it.id
            )));
        }
        ensure_non_negative(&format!("Growth margin of {}", it.id), it.growth)?;
        let total = it.value * f64::from(it.count);
        let row = BudgetRollupRow {
            id: it.id.clone(),
            group: it.group.clone(),
            value: it.value,
            count: it.count,
            growth: it.growth,
            total,
            total_with_growth: total * (1.0 + it.growth),
        };
        add_to_group(&mut by_group, &row);
        rows.push(row);
    }
    let total = rows.iter().map(|r| r.total).sum();
    let total_with_growth: f64 = rows.iter().map(|r| r.total_with_growth).sum();
    let allowed = limit.map(|l| l * (1.0 - system_margin));
    let margin = allowed.map(|a| a - total_with_growth);
    Ok(BudgetRollup {
        total,
        total_with_growth,
        by_group,
        rows,
        limit,
        system_margin,
        allowed,
        margin,
        verdict: margin.map(|m| Verdict::from_ok(m >= 0.0)),
    })
}

fn add_to_group(groups: &mut Vec<BudgetGroup>, row: &BudgetRollupRow) {
    let i = match groups.binary_search_by(|g| g.group.cmp(&row.group)) {
        Ok(i) => i,
        Err(i) => {
            groups.insert(
                i,
                BudgetGroup {
                    group: row.group.clone(),
                    total: 0.0,
                    total_with_growth: 0.0,
                    count: 0,
                },
            );
            i
        }
    };
    groups[i].total += row.total;
    groups[i].total_with_growth += row.total_with_growth;
    groups[i].count += row.count;
}

#[cfg(test)]
mod tests {
    use super::*;
    use sat_units::typed::{Information, Mass};

    fn item(id: &str, group: &str, value: f64, count: u32, growth: f64) -> BudgetItem {
        BudgetItem {
            id: id.to_owned(),
            group: group.to_owned(),
            value,
            count,
            growth,
        }
    }

    #[test]
    fn mass_budget_by_hand() {
        // RW 0.15 kg × 3 台 +5 %、OBC 0.1 kg +20 %:
        // 合計 0.55 kg、成長込み 0.4725 + 0.12 = 0.5925 kg。上限 0.7 kg・システム余裕 10 % → 0.63 kg まで
        let items = [
            item("RW", "AOCS", 0.15, 3, 0.05),
            item("OBC", "CDH", 0.1, 1, 0.2),
        ];
        let r = budget_rollup(&items, Some(0.7), 0.1).unwrap();
        assert!((r.total - 0.55).abs() < 1e-12);
        assert!((r.total_with_growth - 0.5925).abs() < 1e-12);
        assert!((r.allowed.unwrap() - 0.63).abs() < 1e-12);
        assert!((r.margin.unwrap() - 0.0375).abs() < 1e-12);
        assert_eq!(r.verdict, Some(Verdict::Ok));
        assert_eq!(r.by_group[0].group, "AOCS");
        assert_eq!(r.by_group[0].count, 3);
        assert!((r.rows[0].total - 0.45).abs() < 1e-12);
        // 上限 0.65 kg なら 0.585 kg までなので超過
        let over = budget_rollup(&items, Some(0.65), 0.1).unwrap();
        assert_eq!(over.verdict, Some(Verdict::Violation));
        assert_eq!(
            over.check::<Mass>("mass", "Mass").unwrap().verdict,
            Verdict::Violation
        );
    }

    #[test]
    fn counted_helpers_by_hand() {
        // 容量 64 GB × 2 台 + 32 GB × 1 台 = 160 GB（成長余裕なし）
        let gb = 8.0e9;
        let storage = [
            BudgetItem::counted("OBC", 64.0 * gb, 2),
            BudgetItem::counted("CAM", 32.0 * gb, 1),
        ];
        assert!((counted_total(&storage).unwrap() - 160.0 * gb).abs() < 1.0);
        // 160 GB ≤ 200 GB は ok、≤ 150 GB は超過
        let ok = limit_check::<Information>(&storage, 200.0 * gb, "k", "l").unwrap();
        assert_eq!(ok.unwrap().verdict, Verdict::Ok);
        let over = limit_check::<Information>(&storage, 150.0 * gb, "k", "l").unwrap();
        assert_eq!(over.unwrap().verdict, Verdict::Violation);
        assert!(counted_total(&[BudgetItem::counted("A", f64::NAN, 1)]).is_err());
    }

    #[test]
    fn no_limit_means_no_verdict() {
        let r = budget_rollup(&[item("A", "", 1.0, 2, 0.0)], None, 0.0).unwrap();
        assert!((r.total - 2.0).abs() < 1e-12);
        assert!(r.verdict.is_none() && r.check::<Mass>("k", "l").is_none());
        assert!(budget_rollup(&[item("A", "", 1.0, 1, -0.1)], None, 0.0).is_err());
        assert!(budget_rollup(&[], Some(1.0), 1.5).is_err());
    }
}
