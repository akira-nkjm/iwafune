//! 判定 1 つ（[`Check`]）と、その余裕の比べ方（[`margin_order`]）。判定の型は解析の共通の型（`spec`）に属し、ここはその一部。

use super::Verdict;
use sat_units::typed::Fraction;
use serde::{Deserialize, Serialize};

/// 要求や制約との比較（例: DoD が上限を超えていないか）。
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct Check {
    pub key: String,
    pub label: String,
    pub verdict: Verdict,
    pub message: String,
    /// 基準に対する余裕（小さいほど厳しい。負なら基準を満たさない）。基準との比較で判定を出す判定だけが持つ（事実の記述や
    /// 「確かめていない」の注意は持たない）。定義と比べ方は [`Check::with_margin`] と [`margin_order`]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub margin: Option<f64>,
}

impl Check {
    pub fn new(key: &str, label: &str, verdict: Verdict, message: impl Into<String>) -> Self {
        Self {
            key: key.to_owned(),
            label: label.to_owned(),
            verdict,
            message: message.into(),
            margin: None,
        }
    }

    /// 基準に対する余裕を添える（基準と比べて判定を出す判定だけが呼ぶ）。**判定の余裕の定義はここと [`margin_order`] だけ。**
    ///
    /// `margin` は、その判定を出した余裕そのもの（[`super::margin_verdict`] の `margin`、比の余裕 [`super::ratio_margin`]、
    /// バジェットの上限までの量など）で、小さいほど基準に近く、負なら基準を満たさない。量の種類（dB・比・温度差・SI の量）は判定ごとに違い、
    /// 異なる種類の余裕どうしは比べられない。**比べるのは、同じ要求のキーに当たる判定どうし（同じ種類）だけ**なので、
    /// 種類の中で単調（小さいほど厳しい）であれば足りる。数でない・無限の余裕、`None` は添えない（余裕が無いのと同じ）。
    #[must_use]
    pub fn with_margin(mut self, margin: impl Into<Option<f64>>) -> Self {
        self.margin = margin.into().filter(|m| m.is_finite());
        self
    }

    /// 比の余裕 [`super::ratio_margin`] を添える（余裕が無限大の `None` は添えない）。
    #[must_use]
    pub fn with_ratio_margin(self, margin: Option<Fraction>) -> Self {
        match margin {
            Some(m) => self.with_margin(m.value()),
            None => self,
        }
    }
}

/// 余裕の順（厳しい = 小さいほうが先）。余裕を持たない判定は、持つ判定より後（余裕が無い = 比べられない）。
/// どちらも持たない・同じ値なら `Equal`（呼び出し側の安定な選び方〔`min_by`〕で、先に出たほうが残る）。
/// **「どの判定が最も厳しいか」の比べ方はこれだけ**（適合表が使う）。
pub fn margin_order(a: Option<f64>, b: Option<f64>) -> std::cmp::Ordering {
    match (a, b) {
        (Some(x), Some(y)) => x.total_cmp(&y),
        (Some(_), None) => std::cmp::Ordering::Less,
        (None, Some(_)) => std::cmp::Ordering::Greater,
        (None, None) => std::cmp::Ordering::Equal,
    }
}
