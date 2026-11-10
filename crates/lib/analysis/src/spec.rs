//! 解析の共通の型。**解析の trait・エラー・時系列・区間・判定の型はここだけで定義する。**
//!
//! 時系列の集計の部品（日照の判定・周回の切り出しと平均・時間の割合・最長の区間・期間不足の注意）は子の `orbit` に置き、ここで再公開する。

use serde::{Deserialize, Serialize, de::DeserializeOwned};

/// 解析が属する系。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(rename_all = "snake_case")]
pub enum System {
    Power,
    Attitude,
    Cdh,
    Comm,
    Thermal,
    Structure,
}

/// 定常か時系列か。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(rename_all = "snake_case")]
pub enum AnalysisKind {
    Steady,
    TimeSeries,
}

/// 解析の説明（一覧の表示と `describe` に使う）。
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct AnalysisInfo {
    /// 例: `"power.energy_balance"`
    pub id: String,
    pub title: String,
    /// 英語の題名（GUI の英語表示）
    pub title_en: String,
    pub system: System,
    pub kind: AnalysisKind,
    pub description: String,
    /// 英語の説明（GUI の英語表示）
    pub description_en: String,
}

/// すべての解析が実装する trait。
pub trait Analysis {
    /// 例: `"power.energy_balance"`（`<系>.<名前>`）
    const ID: &'static str;
    const TITLE: &'static str;
    const SYSTEM: System;
    const KIND: AnalysisKind;
    const DESCRIPTION: &'static str;
    /// 英語の題名と説明（GUI の英語表示）
    const TITLE_EN: &'static str;
    const DESCRIPTION_EN: &'static str;

    type Input: Serialize + DeserializeOwned;
    type Output: Serialize;

    /// 解析を実行する。純粋な関数として振る舞う（I/O しない、乱数を使わない）。
    fn run(input: &Self::Input) -> Result<Self::Output, AnalysisError>;

    fn info() -> AnalysisInfo {
        AnalysisInfo {
            id: Self::ID.to_owned(),
            title: Self::TITLE.to_owned(),
            title_en: Self::TITLE_EN.to_owned(),
            system: Self::SYSTEM,
            kind: Self::KIND,
            description: Self::DESCRIPTION.to_owned(),
            description_en: Self::DESCRIPTION_EN.to_owned(),
        }
    }
}

/// 解析のエラー。
#[derive(Debug, thiserror::Error)]
pub enum AnalysisError {
    #[error("入力が不正です: {0}")]
    InvalidInput(String),
    #[error("未知の解析です: {0}")]
    Unknown(String),
    #[error("入力の JSON を解釈できません: {0}")]
    Json(String),
    #[error(transparent)]
    Astro(#[from] sat_astro::AstroError),
}

/// 時系列。値は SI で持ち、表示情報は service が付加する。
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct Series {
    /// 元期からの経過 [s]
    pub t: Vec<f64>,
    pub channels: Vec<Channel>,
}

/// 時系列の 1 系列。
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct Channel {
    /// 機械向けの名前（例: `"generation"`、`"soc"`）
    pub key: String,
    /// 表示名（例: `"発電電力"`）
    pub label: String,
    /// 次元の名前（`sat_units::named_dimensions` の名前。例: `"Power"`）
    pub dimension: String,
    /// SI 値
    pub values: Vec<f64>,
}

/// 判定文の量を文字列にする。換算と桁の整形は sat-units に委譲する。
/// 未知の次元なら単位を付けず、sat-units の有効数字で数だけを返す。
pub fn quantity_text(si: f64, dimension: &str) -> String {
    use sat_units::format::{SIGNIFICANT_DIGITS, format_number, format_quantity};
    format_quantity(si, dimension)
        .map(|q| q.text)
        .unwrap_or_else(|_| format_number(si, SIGNIFICANT_DIGITS))
}

/// 判定の結果。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(rename_all = "snake_case")]
pub enum Verdict {
    Ok,
    Warning,
    Violation,
}

/// 要求や制約との比較（例: DoD が上限を超えていないか）。
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct Check {
    pub key: String,
    pub label: String,
    pub verdict: Verdict,
    pub message: String,
}

impl Channel {
    /// 系列を作る（値は SI、表示単位は持たない）。
    pub fn new(key: &str, label: &str, dimension: &str, values: Vec<f64>) -> Self {
        Self {
            key: key.to_owned(),
            label: label.to_owned(),
            dimension: dimension.to_owned(),
            values,
        }
    }
}

impl Verdict {
    /// 最も悪い判定。判定が無ければ None。
    pub fn worst(verdicts: impl IntoIterator<Item = Self>) -> Option<Self> {
        verdicts.into_iter().max_by_key(|v| match v {
            Self::Ok => 0,
            Self::Warning => 1,
            Self::Violation => 2,
        })
    }
}

impl Check {
    pub fn new(key: &str, label: &str, verdict: Verdict, message: impl Into<String>) -> Self {
        Self {
            key: key.to_owned(),
            label: label.to_owned(),
            verdict,
            message: message.into(),
        }
    }
}

impl Verdict {
    /// 満たしていれば `Ok`、満たしていなければ `Violation`（二段階の判定）。
    pub fn from_ok(ok: bool) -> Self {
        if ok { Self::Ok } else { Self::Violation }
    }
}

/// 余裕の三段階の判定。`margin < 0` なら `Violation`、`0 ≤ margin < warn_threshold` なら `Warning`、それ以上は `Ok`。
/// `margin` と `warn_threshold` は同じ尺度（比でも SI の量でもよい）。**余裕の判定はこれを使う。**
pub fn margin_verdict(margin: f64, warn_threshold: f64) -> Verdict {
    if margin.is_nan() || margin < 0.0 {
        Verdict::Violation
    } else if margin < warn_threshold {
        Verdict::Warning
    } else {
        Verdict::Ok
    }
}

/// 員数（入力の `count`）の既定値 1。**`count` の serde の既定はこれだけを使う**（`#[serde(default = "crate::spec::default_count")]`）。
pub fn default_count() -> u32 {
    1
}

/// `v` が 0〜1 の有限値か調べる（割合・効率・比）。違えば `name` を含む `InvalidInput`。
pub fn ensure_fraction(name: &str, v: f64) -> Result<(), AnalysisError> {
    if v.is_finite() && (0.0..=1.0).contains(&v) {
        Ok(())
    } else {
        Err(AnalysisError::InvalidInput(format!(
            "{name} は 0〜1 にしてください（{v}）"
        )))
    }
}

/// `v` が正の有限値か調べる（容量・電圧・推力など）。違えば `name` を含む `InvalidInput`。
pub fn ensure_positive(name: &str, v: f64) -> Result<(), AnalysisError> {
    if v.is_finite() && v > 0.0 {
        Ok(())
    } else {
        Err(AnalysisError::InvalidInput(format!(
            "{name} は正の値にしてください（{v}）"
        )))
    }
}

/// `v` が 0 以上の有限値か調べる（面積・成長余裕・倍率など）。違えば `name` を含む `InvalidInput`。
pub fn ensure_non_negative(name: &str, v: f64) -> Result<(), AnalysisError> {
    if v.is_finite() && v >= 0.0 {
        Ok(())
    } else {
        Err(AnalysisError::InvalidInput(format!(
            "{name} は 0 以上にしてください（{v}）"
        )))
    }
}

mod orbit;
pub use orbit::{
    SUNLIT_THRESHOLD, SpanFallback, is_sunlit, longest_window, masked_time_mean, orbit_mean,
    orbit_ranges, orbit_segment, orbit_segments, short_span_check, time_fraction,
};

/// 時系列の中の区間と、その抽出。実体は sat-astro（下の層）にあり、ここでは再公開するだけ。
pub use sat_astro::timeline::{Window, windows};

/// 標本の間を線形補間して状態を積分する 1 刻み。実体は `sat_astro::ode`（積分器はそこだけ）にあり、ここでは再公開する。
pub use sat_astro::ode::rk4_linear_step;

#[cfg(test)]
mod tests {
    #[test]
    fn quantity_text_delegates_units_and_unknown_dimensions() {
        assert_eq!(
            super::quantity_text(0.125, sat_units::typed::Fraction::KIND),
            "12.5 %"
        );
        assert_eq!(super::quantity_text(3600.0, "Energy"), "1.00 Wh");
        assert_eq!(super::quantity_text(12.5, "unknown"), "12.5");
    }

    #[test]
    fn verdict_helpers() {
        assert_eq!(super::Verdict::worst([]), None);
        assert_eq!(
            super::Verdict::worst([super::Verdict::Warning, super::Verdict::Ok]),
            Some(super::Verdict::Warning)
        );
        assert_eq!(
            super::Verdict::worst([super::Verdict::Violation, super::Verdict::Warning]),
            Some(super::Verdict::Violation)
        );
        use super::{Verdict, ensure_fraction, ensure_positive, margin_verdict};
        assert_eq!(Verdict::from_ok(true), Verdict::Ok);
        assert_eq!(Verdict::from_ok(false), Verdict::Violation);
        assert_eq!(margin_verdict(-0.01, 0.2), Verdict::Violation);
        assert_eq!(margin_verdict(0.1, 0.2), Verdict::Warning);
        assert_eq!(margin_verdict(0.2, 0.2), Verdict::Ok);
        assert_eq!(margin_verdict(f64::NAN, 0.2), Verdict::Violation);
        assert!(ensure_fraction("効率", 0.5).is_ok());
        assert!(ensure_fraction("効率", 1.5).is_err());
        assert!(ensure_positive("容量", 0.0).is_err());
        assert!(ensure_positive("容量", f64::INFINITY).is_err());
    }
}
