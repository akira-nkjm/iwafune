//! 節点の温度と 3 種の温度範囲（保管・動作・性能）の比較と、解析マージンの方針（`thermal.steady` と `thermal.transient` で共有）。
//! **温度範囲の判定と、解析マージンの既定値はここだけ。**
//!
//! 判定は「予測の最低 − 不確かさ」と「予測の最高 + 不確かさ」を範囲と比べる。範囲を外れれば違反、
//! 端から警告の幅以内なら警告（`spec::margin_verdict`）。範囲の種類ごとに、上限側（key は `range_high:<節点>:<種類>`）と
//! 下限側（`range_low:<節点>:<種類>`）の 2 つの判定を出す（KPI の最高温度は上限側、最低温度は下限側だけを見るため）。
//!
//! モードのあるケースでは、部品のそのモードでの電源の状態（`modes::power_states`）で当てる範囲を選ぶ
//! （[`ThermalRangeKind::applies`]: オフの部品は保管だけ、オンの部品は動作と性能）。外した部品は判定文（`checks`）に書く。

use crate::modes::PowerStates;
use crate::spec::{Verdict, margin_verdict};
use sat_units::typed::{Qty, Temperature, TemperatureDifference};
use serde::{Deserialize, Serialize};

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
            Self::Storage => "storage range",
            Self::Operating => "operating range",
            Self::Performance => "performance range",
        }
    }

    /// モードでの電源の状態 `s` の部品にこの種類の範囲を当てるか。保管はオフの時間がある部品、動作と性能はオンの時間が
    /// ある部品（duty や予備機で両方あれば 3 種すべて）。**範囲の選び方はここだけ。**
    pub fn applies(self, s: PowerStates) -> bool {
        match self {
            Self::Storage => s.off,
            Self::Operating | Self::Performance => s.on,
        }
    }
}

/// 部品のモードでの電源（判定から外した理由）。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(rename_all = "snake_case")]
pub enum ThermalPowerState {
    /// オン（保管の範囲から外した）
    On,
    /// オフ（動作・性能の範囲から外した）
    Off,
}

impl ThermalPowerState {
    fn noun(self) -> &'static str {
        match self {
            Self::On => "powered-on",
            Self::Off => "powered-off",
        }
    }
}

/// モードでの電源の状態で、ある種類の温度範囲の判定から外した部品。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct ThermalRangeExclusion {
    /// 部品 ID
    pub part: String,
    /// モードの ID
    pub mode: String,
    /// そのモードでの電源
    pub power_state: ThermalPowerState,
}

/// 節点の 1 種類の温度範囲（当てる部品の範囲の共通部分）と、電源の状態で外した部品。
#[derive(Debug, Clone, PartialEq)]
pub struct ThermalNodeRange {
    pub kind: ThermalRangeKind,
    pub range: ThermalTempRange,
    pub excluded: Vec<ThermalRangeExclusion>,
    /// 部品の範囲と違う限界を与えた要求（`ThermalRequiredRange::tighten`）
    pub required: Vec<ThermalRequiredUse>,
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

/// 1 種類の温度範囲との比較。
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct ThermalRangeMargin {
    pub kind: ThermalRangeKind,
    pub range: ThermalTempRange,
    /// 下限からの余裕 T_min − min（不確かさを引く前。温度差）
    #[serde(serialize_with = "sat_units::typed::TemperatureDifferenceKind::serialize")]
    pub margin_low: Qty<TemperatureDifference>,
    /// 上限までの余裕 max − T_max（不確かさを足す前。温度差）
    #[serde(serialize_with = "sat_units::typed::TemperatureDifferenceKind::serialize")]
    pub margin_high: Qty<TemperatureDifference>,
    /// 不確かさを含めた判定
    pub verdict: Verdict,
    /// モードでの電源の状態でこの範囲から外した部品（モードの無いケースでは空）
    #[serde(default)]
    pub excluded: Vec<ThermalRangeExclusion>,
    /// 部品の範囲と違う限界を与えた要求（厳しくして効いたもの・緩くて効かなかったもの。無ければ空）
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub required: Vec<ThermalRequiredUse>,
}

/// 温度範囲の片側（判定は側ごとに出す）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ThermalRangeSide {
    /// 下限側（予測の最低 − 不確かさ と下限）
    Low,
    /// 上限側（予測の最高 + 不確かさ と上限）
    High,
}

impl ThermalRangeSide {
    /// 両側（`range_item` が側を見分けるのに使う）。
    pub const ALL: [Self; 2] = [Self::Low, Self::High];

    /// 判定の key の先頭（`range_low` / `range_high`。KPI の対応は `fune-service` の `kpi::rule`）。
    pub fn key_prefix(self) -> &'static str {
        match self {
            Self::Low => sat_checkkey::family::RANGE_LOW,
            Self::High => sat_checkkey::family::RANGE_HIGH,
        }
    }
}

/// 温度範囲の判定の key から、側を除いた項目（`<節点>:<種類>`）を取り出す。温度範囲の判定でなければ None。
/// 同じ節点・種類の上限側と下限側は同じ項目になる（KPI が節点・種類ごとに数えるとき用）。
pub fn range_item(check_key: &str) -> Option<&str> {
    ThermalRangeSide::ALL.iter().find_map(|side| {
        check_key
            .strip_prefix(side.key_prefix())
            .and_then(|rest| rest.strip_prefix(':'))
    })
}

impl ThermalRangeMargin {
    /// 片側の余裕 [K]（限界までの差から不確かさを引いた値。負なら限界を超える）。判定（`side_verdict`）と判定の余裕の比較（`Check::margin`）が使う。
    pub(crate) fn side_margin(&self, side: ThermalRangeSide, margin: &ThermalMarginUsed) -> f64 {
        let m = match side {
            ThermalRangeSide::Low => self.margin_low,
            ThermalRangeSide::High => self.margin_high,
        };
        m.value() - margin.uncertainty.value()
    }

    /// 片側の判定（不確かさを引いてから。範囲全体の `verdict` は両側の悪いほう）。
    pub(crate) fn side_verdict(
        &self,
        side: ThermalRangeSide,
        margin: &ThermalMarginUsed,
    ) -> Verdict {
        margin_verdict(self.side_margin(side, margin), margin.warning_band.value())
    }

    /// 余裕から比べた予測の最低・最高 [K]（`range.min + margin_low`・`range.max − margin_high`）。
    fn predicted(&self) -> (f64, f64) {
        (
            self.range.min.value() + self.margin_low.value(),
            self.range.max.value() - self.margin_high.value(),
        )
    }
}

/// 余裕 `low`・`high` [K] からの 1 つの比較（判定は不確かさを引いてから）。
fn judged(
    kind: ThermalRangeKind,
    range: ThermalTempRange,
    (low, high): (f64, f64),
    (excluded, required): (Vec<ThermalRangeExclusion>, Vec<ThermalRequiredUse>),
    margin: &ThermalMarginUsed,
) -> ThermalRangeMargin {
    let mut m = ThermalRangeMargin {
        kind,
        range,
        margin_low: Qty::from_raw(low),
        margin_high: Qty::from_raw(high),
        verdict: Verdict::Ok,
        excluded,
        required,
    };
    // 全体の判定は両側の悪いほう（側ごとの判定は `side_verdict` だけ）
    let sides = [ThermalRangeSide::Low, ThermalRangeSide::High].map(|s| m.side_verdict(s, margin));
    m.verdict = Verdict::worst(sides).unwrap_or(Verdict::Ok);
    m
}

/// 最低 `lo`・最高 `hi` [K] を節点の範囲（`ThermalNode::ranges_in`。[`ThermalRangeKind::ALL`] の順）と比べる。
pub(crate) fn range_margins(
    lo: f64,
    hi: f64,
    ranges: &[ThermalNodeRange],
    margin: &ThermalMarginUsed,
) -> Vec<ThermalRangeMargin> {
    ranges
        .iter()
        .map(|r| {
            let low_high = (lo - r.range.min.value(), r.range.max.value() - hi);
            let extra = (r.excluded.clone(), r.required.clone());
            judged(r.kind, r.range, low_high, extra, margin)
        })
        .collect()
}

/// ケースごとの比較（`per_case`）を種類ごとにまとめる（定常の全ケースの判定）。種類はその範囲を当てたケースだけで見て、
/// 下限の側は下限までの余裕が最も小さいケース、上限の側は上限までの余裕が最も小さいケースの範囲と余裕を使う
/// （範囲がケースで変わらなければ、全ケースの最低・最高を範囲と比べるのと同じ）。外した部品は両方のケースの分。
pub(crate) fn envelope_margins<'a>(
    per_case: impl Iterator<Item = &'a [ThermalRangeMargin]> + Clone,
    margin: &ThermalMarginUsed,
) -> Vec<ThermalRangeMargin> {
    ThermalRangeKind::ALL
        .into_iter()
        .filter_map(|kind| {
            let of_kind = per_case.clone().flatten().filter(|m| m.kind == kind);
            let by = |f: fn(&ThermalRangeMargin) -> f64| {
                of_kind.clone().min_by(|a, b| f(a).total_cmp(&f(b)))
            };
            let low = by(|m| m.margin_low.value())?;
            let high = by(|m| m.margin_high.value())?;
            let range = ThermalTempRange {
                min: low.range.min,
                max: high.range.max,
            };
            let mut excluded = low.excluded.clone();
            excluded.extend(
                high.excluded
                    .iter()
                    .filter(|e| !low.excluded.contains(e))
                    .cloned(),
            );
            // 要求の使われ方は、その側の判定に使うケースのものだけ
            let mut required: Vec<ThermalRequiredUse> = low
                .required
                .iter()
                .filter(|u| u.is_side(ThermalRangeSide::Low))
                .cloned()
                .collect();
            required.extend(
                high.required
                    .iter()
                    .filter(|u| u.is_side(ThermalRangeSide::High))
                    .cloned(),
            );
            let low_high = (low.margin_low.value(), high.margin_high.value());
            Some(judged(kind, range, low_high, (excluded, required), margin))
        })
        .collect()
}

/// 比較のうち最も悪い判定（範囲が 1 つも無ければ Ok）。
pub(crate) fn worst_verdict(margins: &[ThermalRangeMargin]) -> Verdict {
    Verdict::worst(margins.iter().map(|m| m.verdict)).unwrap_or(Verdict::Ok)
}

mod checks;
mod policy;
mod required;
#[cfg(test)]
use checks::excluded_text;
pub(crate) use checks::range_checks;
pub use policy::{
    DEFAULT_HEATER_DUTY_WARNING, DEFAULT_UNCERTAINTY_K, RANGE_WARNING_BAND_K, ThermalMarginPolicy,
    ThermalMarginUsed,
};
pub use required::{ThermalRequiredBound, ThermalRequiredRange, ThermalRequiredUse};

#[cfg(test)]
mod tests;
