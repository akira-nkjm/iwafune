//! 解析の共通の型。**解析の trait・エラー・時系列・区間・判定の型はここだけで定義する。**

use sat_astro::timeline::time_mean;
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

/// 完全な周回ごとのサンプルの添字の範囲（半開区間）。k 周目は t0 + k·T ≤ t < t0 + (k+1)·T のサンプル。
/// 回り切っていない最後の周回は含めない（境界の判定には 1e-6·T の許容を持たせる）。
/// `range.end` は次の周回の先頭の添字で、最後の周回では最後のサンプルの添字になることがある。
/// 周回平均・周回ごとの積算に使う。**周回の切り出しはここだけ。**
pub fn orbit_ranges(t: &[f64], period_s: f64) -> Vec<std::ops::Range<usize>> {
    if t.is_empty() || period_s.is_nan() || period_s <= 0.0 {
        return Vec::new();
    }
    let t0 = t[0];
    let tol = 1e-6 * period_s;
    let mut out = Vec::new();
    let mut start = 0;
    let mut k = 1.0;
    for (i, &ti) in t.iter().enumerate() {
        if ti - t0 >= k * period_s - tol {
            out.push(start..i);
            start = i;
            k += 1.0;
        }
    }
    out
}

/// 時系列の中の区間と、その抽出。実体は sat-astro（下の層）にあり、ここでは再公開するだけ。
pub use sat_astro::timeline::{Window, windows};

/// 完全な周回（[`orbit_ranges`] の半開区間）にわたる時間平均（台形則、`sat_astro::timeline::time_mean`）。
/// 区間 `s..e` は時刻 `t[s]` から次の周回の先頭 `t[e]` まで（`e` が末尾を越えるときは最後の標本まで）を積分する。
/// 周回が 1 つも無ければ全期間の時間平均。空なら 0。**周回平均はこれを使う。**
pub fn orbit_mean(t: &[f64], values: &[f64], ranges: &[std::ops::Range<usize>]) -> f64 {
    let n = t.len().min(values.len());
    if n == 0 {
        return 0.0;
    }
    if ranges.is_empty() {
        let m = time_mean(&t[..n], &values[..n]);
        return if m.is_finite() { m } else { 0.0 };
    }
    let (mut area, mut span) = (0.0, 0.0);
    for r in ranges {
        let (s, e) = (r.start.min(n - 1), r.end.min(n - 1));
        let duration = t[e] - t[s];
        if duration > 0.0 {
            area += time_mean(&t[s..=e], &values[s..=e]) * duration;
            span += duration;
        }
    }
    if span > 0.0 {
        area / span
    } else {
        values[ranges[0].start.min(n - 1)]
    }
}

/// `mask` が真の時間だけの時間平均（台形則。`∫ v·m dt / ∫ m dt`、m は 0/1）。真の時間が無ければ `None`。
/// 例: 日照中の平均発電。**条件付きの時間平均はこれを使う。**
pub fn masked_time_mean(t: &[f64], values: &[f64], mask: &[bool]) -> Option<f64> {
    let m: Vec<f64> = mask.iter().map(|&b| f64::from(u8::from(b))).collect();
    let vm: Vec<f64> = values.iter().zip(&m).map(|(v, w)| v * w).collect();
    let den = time_mean(t, &m);
    if den.is_finite() && den > 0.0 {
        Some(time_mean(t, &vm) / den)
    } else {
        None
    }
}

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
    fn orbit_mean_is_time_weighted() {
        use super::{orbit_mean, orbit_ranges};
        // 周期 100 s、刻みが不揃い（0, 10, 100, 110, 200）。v = t mod 100 の鋸波の 1 周の時間平均
        let t = [0.0, 10.0, 100.0, 110.0, 200.0];
        let v = [0.0, 10.0, 0.0, 10.0, 0.0];
        let r = orbit_ranges(&t, 100.0);
        assert_eq!(r.len(), 2);
        // 台形則: [0,10] 平均 5 ×10 + [10,100] 平均 5 ×90 → 5
        assert!((orbit_mean(&t, &v, &r) - 5.0).abs() < 1e-12);
        // 周回が無ければ全期間の時間平均
        assert!((orbit_mean(&t[..2], &v[..2], &[]) - 5.0).abs() < 1e-12);
        assert_eq!(orbit_mean(&[], &[], &[]), 0.0);
    }

    #[test]
    fn masked_mean_by_hand() {
        // 0〜20 s が真、v = 4 → 平均 4。真が無ければ None
        let t = [0.0, 10.0, 20.0, 30.0];
        let v = [4.0, 4.0, 4.0, 100.0];
        let m = [true, true, true, false];
        let got = super::masked_time_mean(&t, &v, &m).unwrap();
        assert!((got - 4.0).abs() < 1e-12);
        assert!(super::masked_time_mean(&t, &v, &[false; 4]).is_none());
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

    #[test]
    fn orbit_ranges_full_orbits_only() {
        let t: Vec<f64> = (0..=25).map(|i| f64::from(i) * 10.0).collect(); // 0..250 s
        let r = super::orbit_ranges(&t, 100.0);
        assert_eq!(r, vec![0..10, 10..20]);
        assert!(super::orbit_ranges(&t, 0.0).is_empty());
    }
}
