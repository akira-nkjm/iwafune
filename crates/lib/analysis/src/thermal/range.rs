//! 節点の温度と 3 種の温度範囲（保管・動作・性能）の比較と、解析マージンの方針（`thermal.steady` と `thermal.transient` で共有）。
//! **温度範囲の判定と、解析マージンの既定値はここだけ。**
//!
//! 判定は「予測の最低 − 不確かさ」と「予測の最高 + 不確かさ」を範囲と比べる。範囲を外れれば違反、
//! 端から警告の幅以内なら警告（`spec::margin_verdict`）。範囲の種類ごとに 1 つの判定（key は `range:<節点>:<種類>`）を出す。

use crate::spec::quantity_text;
use crate::spec::{Check, Verdict, margin_verdict};
use sat_units::typed::{Qty, Temperature};
use serde::{Deserialize, Serialize};

/// 範囲の端からこの幅 [K] 以内なら警告にする（解析マージンの方針の `warning_band` の既定値）。
pub const RANGE_WARNING_BAND_K: f64 = 5.0;

/// 予測温度の不確かさの既定値 [K]（解析マージンの方針の `uncertainty` の既定値）。0 なら予測温度をそのまま比べる。
pub const DEFAULT_UNCERTAINTY_K: f64 = 0.0;

/// 温度の範囲。
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct ThermalTempRange {
    pub min: Qty<Temperature>,
    pub max: Qty<Temperature>,
}

impl ThermalTempRange {
    /// 2 つの範囲の共通部分（重ならなければ min > max になる）。
    pub fn intersect(self, other: Self) -> Self {
        Self {
            min: Qty::from_raw(self.min.value().max(other.min.value())),
            max: Qty::from_raw(self.max.value().min(other.max.value())),
        }
    }
}

/// 温度範囲の種類。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(rename_all = "snake_case")]
pub enum ThermalRangeKind {
    /// 保管（非動作で壊れない範囲。電源を切っていても守る）
    Storage,
    /// 動作（機能を維持する範囲）
    Operating,
    /// 性能（仕様の性能を満たす範囲）
    Performance,
}

impl ThermalRangeKind {
    /// 判定を出す順（広い範囲から）。
    pub const ALL: [Self; 3] = [Self::Storage, Self::Operating, Self::Performance];

    /// 判定の key の末尾（`range:<節点>:<種類>`）。
    pub fn key(self) -> &'static str {
        match self {
            Self::Storage => "storage",
            Self::Operating => "operating",
            Self::Performance => "performance",
        }
    }

    /// 判定文に入れる範囲の呼び名（判定文は日本語。他の解析の判定文と同じ）。
    fn noun(self) -> &'static str {
        match self {
            Self::Storage => "保管範囲",
            Self::Operating => "動作範囲",
            Self::Performance => "性能範囲",
        }
    }
}

/// 部品（または節点）の 3 種の温度範囲。書かれていない種類は `None`（判定しない）。
#[derive(Debug, Clone, Copy, Default, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct ThermalRanges {
    /// 保管（非動作）の範囲
    #[serde(default)]
    pub storage: Option<ThermalTempRange>,
    /// 動作の範囲
    #[serde(default)]
    pub operating: Option<ThermalTempRange>,
    /// 性能の範囲
    #[serde(default)]
    pub performance: Option<ThermalTempRange>,
}

impl ThermalRanges {
    /// 種類の範囲。
    pub fn get(&self, kind: ThermalRangeKind) -> Option<ThermalTempRange> {
        match kind {
            ThermalRangeKind::Storage => self.storage,
            ThermalRangeKind::Operating => self.operating,
            ThermalRangeKind::Performance => self.performance,
        }
    }

    /// 種類ごとの共通部分（片方にしか無い種類はその範囲）。
    pub fn intersect(self, other: Self) -> Self {
        let both = |a: Option<ThermalTempRange>, b: Option<ThermalTempRange>| match (a, b) {
            (Some(a), Some(b)) => Some(a.intersect(b)),
            (a, b) => a.or(b),
        };
        Self {
            storage: both(self.storage, other.storage),
            operating: both(self.operating, other.operating),
            performance: both(self.performance, other.performance),
        }
    }

    /// 書かれている種類と範囲（[`ThermalRangeKind::ALL`] の順）。
    pub fn iter(&self) -> impl Iterator<Item = (ThermalRangeKind, ThermalTempRange)> + '_ {
        ThermalRangeKind::ALL
            .into_iter()
            .filter_map(|k| self.get(k).map(|r| (k, r)))
    }
}

/// 解析マージンの方針。書かれていない値は解析の既定値（取り出しはメソッド、使った値は [`Self::used`]）。
#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct ThermalMarginPolicy {
    /// 予測温度の不確かさ（最高に足し、最低から引いてから範囲と比べる。既定 [`DEFAULT_UNCERTAINTY_K`]）
    #[serde(default)]
    pub uncertainty: Option<Qty<Temperature>>,
    /// 範囲の端からこの幅以内を警告にする（既定 [`RANGE_WARNING_BAND_K`]）
    #[serde(default)]
    pub warning_band: Option<Qty<Temperature>>,
}

impl ThermalMarginPolicy {
    /// 予測温度の不確かさ [K]（省略時 [`DEFAULT_UNCERTAINTY_K`]）。
    pub fn uncertainty(&self) -> Qty<Temperature> {
        self.uncertainty
            .unwrap_or(Qty::from_raw(DEFAULT_UNCERTAINTY_K))
    }

    /// 警告の幅 [K]（省略時 [`RANGE_WARNING_BAND_K`]）。
    pub fn warning_band(&self) -> Qty<Temperature> {
        self.warning_band
            .unwrap_or(Qty::from_raw(RANGE_WARNING_BAND_K))
    }

    /// 使った値（既定値を埋めたもの）。出力に返す。
    pub fn used(&self) -> ThermalMarginUsed {
        ThermalMarginUsed {
            uncertainty: self.uncertainty(),
            warning_band: self.warning_band(),
        }
    }
}

/// 解析に使った解析マージン（既定値を埋めたもの）。
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct ThermalMarginUsed {
    /// 予測温度の不確かさ（温度差 [K]）
    pub uncertainty: Qty<Temperature>,
    /// 範囲の端からこの幅以内を警告にした（温度差 [K]。Web の温度表もこれを使う）
    pub warning_band: Qty<Temperature>,
}

impl ThermalMarginUsed {
    /// 入力を検査する（不確かさと警告の幅は 0 以上の有限の値）。
    pub(crate) fn validate(&self) -> Result<(), crate::spec::AnalysisError> {
        let ok = |v: f64| v.is_finite() && v >= 0.0;
        if ok(self.uncertainty.value()) && ok(self.warning_band.value()) {
            Ok(())
        } else {
            Err(crate::spec::AnalysisError::InvalidInput(
                "解析マージンの不確かさと警告の幅は 0 以上にしてください".to_owned(),
            ))
        }
    }
}

/// 1 種類の温度範囲との比較。
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct ThermalRangeMargin {
    pub kind: ThermalRangeKind,
    pub range: ThermalTempRange,
    /// 下限からの余裕 T_min − min（不確かさを引く前。温度差）
    pub margin_low: Qty<Temperature>,
    /// 上限までの余裕 max − T_max（不確かさを足す前。温度差）
    pub margin_high: Qty<Temperature>,
    /// 不確かさを含めた判定
    pub verdict: Verdict,
}

/// 最低 `lo`・最高 `hi` [K] を書かれている種類の範囲と比べる（[`ThermalRangeKind::ALL`] の順）。
pub(crate) fn range_margins(
    lo: f64,
    hi: f64,
    ranges: &ThermalRanges,
    margin: &ThermalMarginUsed,
) -> Vec<ThermalRangeMargin> {
    let u = margin.uncertainty.value();
    ranges
        .iter()
        .map(|(kind, r)| {
            let (low, high) = (lo - r.min.value(), r.max.value() - hi);
            ThermalRangeMargin {
                kind,
                range: r,
                margin_low: Qty::from_raw(low),
                margin_high: Qty::from_raw(high),
                verdict: margin_verdict((low - u).min(high - u), margin.warning_band.value()),
            }
        })
        .collect()
}

/// 比較のうち最も悪い判定（範囲が 1 つも無ければ Ok）。
pub(crate) fn worst_verdict(margins: &[ThermalRangeMargin]) -> Verdict {
    Verdict::worst(margins.iter().map(|m| m.verdict)).unwrap_or(Verdict::Ok)
}

/// 種類ごとの判定（key は `range:<節点>:<種類>`）。温度の表示は sat-units に委譲する。
pub(crate) fn range_checks(
    node: &str,
    lo: f64,
    hi: f64,
    margins: &[ThermalRangeMargin],
    margin: &ThermalMarginUsed,
) -> Vec<Check> {
    let t = quantity_text::<Temperature>;
    let u = margin.uncertainty.value();
    margins
        .iter()
        .map(|m| {
            let predicted = if u > 0.0 {
                format!(
                    "{}〜{}、不確かさ込み {}〜{}",
                    t(lo),
                    t(hi),
                    t(lo - u),
                    t(hi + u)
                )
            } else {
                format!("{}〜{}", t(lo), t(hi))
            };
            Check::new(
                &format!("range:{node}:{}", m.kind.key()),
                &format!("{node} の温度（{}）", m.kind.noun()),
                m.verdict,
                format!(
                    "{predicted}（{} {}〜{}）",
                    m.kind.noun(),
                    t(m.range.min.value()),
                    t(m.range.max.value())
                ),
            )
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn range(min: f64, max: f64) -> ThermalTempRange {
        ThermalTempRange {
            min: Qty::from_raw(min),
            max: Qty::from_raw(max),
        }
    }

    fn verdicts(
        lo: f64,
        hi: f64,
        ranges: &ThermalRanges,
        margin: ThermalMarginPolicy,
    ) -> Vec<Verdict> {
        range_margins(lo, hi, ranges, &margin.used())
            .iter()
            .map(|m| m.verdict)
            .collect()
    }

    /// 動作範囲 270〜320 K: 280〜300 K は端から 10 K 離れて Ok、316 K は上端まで 4 K（< 5 K）で警告、
    /// 321 K は超過。範囲が無ければ比較も判定文も無く、全体の判定は Ok。
    #[test]
    fn verdict_by_distance_to_the_ends() {
        let r = ThermalRanges {
            operating: Some(range(270.0, 320.0)),
            ..ThermalRanges::default()
        };
        let p = ThermalMarginPolicy::default();
        assert_eq!(verdicts(280.0, 300.0, &r, p), [Verdict::Ok]);
        assert_eq!(verdicts(280.0, 316.0, &r, p), [Verdict::Warning]);
        assert_eq!(verdicts(280.0, 321.0, &r, p), [Verdict::Violation]);
        assert_eq!(verdicts(268.0, 300.0, &r, p), [Verdict::Violation]);
        let none = range_margins(0.0, 1.0e4, &ThermalRanges::default(), &p.used());
        assert!(none.is_empty());
        assert_eq!(worst_verdict(&none), Verdict::Ok);
        let m = range_margins(280.0, 316.0, &r, &p.used());
        let checks = range_checks("box", 280.0, 316.0, &m, &p.used());
        assert_eq!(checks[0].key, "range:box:operating");
        assert!((m[0].margin_low.value() - 10.0).abs() < 1e-12);
        assert!((m[0].margin_high.value() - 4.0).abs() < 1e-12);
    }

    /// 保管 250〜340 K・動作 270〜320 K・性能 285〜305 K に 280〜300 K を当てる: 保管は端から 30 K（Ok）、
    /// 動作は 10 K（Ok）、性能は下端を 5 K 下回る（違反）。不確かさ 8 K を足すと動作は 2 K（< 5 K）で警告、
    /// 保管は 22 K で Ok のまま。警告の幅を 25 K にすると保管も警告。
    #[test]
    fn three_ranges_and_uncertainty_by_hand() {
        let r = ThermalRanges {
            storage: Some(range(250.0, 340.0)),
            operating: Some(range(270.0, 320.0)),
            performance: Some(range(285.0, 305.0)),
        };
        use Verdict::{Ok, Violation, Warning};
        let none = ThermalMarginPolicy::default();
        assert_eq!(verdicts(280.0, 300.0, &r, none), [Ok, Ok, Violation]);
        let u8 = ThermalMarginPolicy {
            uncertainty: Some(Qty::from_raw(8.0)),
            warning_band: None,
        };
        assert_eq!(verdicts(280.0, 300.0, &r, u8), [Ok, Warning, Violation]);
        let wide = ThermalMarginPolicy {
            warning_band: Some(Qty::from_raw(25.0)),
            ..u8
        };
        assert_eq!(
            verdicts(280.0, 300.0, &r, wide),
            [Warning, Warning, Violation]
        );
        let m = range_margins(280.0, 300.0, &r, &u8.used());
        let keys: Vec<String> = range_checks("n", 280.0, 300.0, &m, &u8.used())
            .into_iter()
            .map(|c| c.key)
            .collect();
        assert_eq!(
            keys,
            [
                "range:n:storage",
                "range:n:operating",
                "range:n:performance"
            ]
        );
        assert_eq!(worst_verdict(&m), Violation);
    }

    /// 種類ごとの共通部分: 片方にしか無い種類はその範囲のまま。
    #[test]
    fn ranges_intersect_per_kind() {
        let a = ThermalRanges {
            operating: Some(range(233.0, 343.0)),
            storage: Some(range(223.0, 353.0)),
            performance: None,
        };
        let b = ThermalRanges {
            operating: Some(range(253.0, 358.0)),
            storage: None,
            performance: Some(range(273.0, 303.0)),
        };
        let c = a.intersect(b);
        assert_eq!(c.operating, Some(range(253.0, 343.0)));
        assert_eq!(c.storage, a.storage);
        assert_eq!(c.performance, b.performance);
    }
}
