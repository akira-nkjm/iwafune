//! KPI ごとの扱い（`Rule`）: どの check に結ぶか（判定の対象か）・塗りの比・判定を数えるか・測れたときだけ出すか。
//! **KPI と解析の check の対応はここだけ。** check のキーは sat-analysis が決める（キーを変えると `rule_tests` が落ちる）。

use crate::dto::{CheckView, KpiKey};
use sat_analysis::spec::Verdict;

/// KPI の判定にする check の選び方（`CheckView.key` との照合）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum CheckMatch {
    /// 判定の対象でない KPI（発生量・可視時間など。判定は null）
    Unjudged,
    /// 解析の全 check（解析全体を代表する KPI だけ）
    Any,
    /// このキーの check
    Exact(&'static str),
    /// このキーで始まる check の並び（節点・回線・センサなどの要素ごとの check。最悪を使う）
    Prefix(&'static str),
    /// いずれかに当たる check
    OneOf(&'static [CheckMatch]),
}

impl CheckMatch {
    /// 判定の対象か。
    pub(super) fn is_judged(self) -> bool {
        self != Self::Unjudged
    }

    /// check のキーがこの選び方に当たるか。
    pub(super) fn matches(self, key: &str) -> bool {
        match self {
            Self::Unjudged => false,
            Self::Any => true,
            Self::Exact(k) => key == k,
            Self::Prefix(p) => key.starts_with(p),
            Self::OneOf(ms) => ms.iter().any(|m| m.matches(key)),
        }
    }

    /// KPI の判定: 当たる check の最悪（当たる check が無ければ None）。
    pub(super) fn verdict(self, checks: &[CheckView]) -> Option<Verdict> {
        Verdict::worst(
            checks
                .iter()
                .filter(|c| self.matches(&c.key))
                .map(|c| c.verdict),
        )
    }

    /// キーを指定している選び方（`Exact`・`Prefix`）を平らに並べる（検査用）。
    #[cfg(test)]
    pub(super) fn named(self) -> Vec<CheckMatch> {
        match self {
            Self::Unjudged | Self::Any => Vec::new(),
            Self::Exact(_) | Self::Prefix(_) => vec![self],
            Self::OneOf(ms) => ms.iter().flat_map(|m| m.named()).collect(),
        }
    }
}

/// 塗りの比の決め方。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum Fill {
    /// 塗らない
    None,
    /// 値そのもの（比の KPI）
    Value,
    /// OK の判定の割合（判定を数える KPI）
    OkShare,
}

/// KPI ごとの扱い。
pub(super) struct Rule {
    /// 結果に値も判定も無ければ出さない（設計が持たない手段の KPI。MTQ・RCS のアンロード時間）。
    /// 値が無くても判定があれば「—」と判定で出す（捨てられない向き・評価できない磁場）。
    /// 解析のすべての KPI が出ないときは先頭の 1 つを残す（行に「—」を出すため）
    pub(super) when_measured: bool,
    /// 判定にする check。KPI が見せる量をちょうど判定する check だけを選ぶ（他の量の違反を巻き込まない）
    pub(super) checks: CheckMatch,
    pub(super) fill: Fill,
    pub(super) tally: bool,
}

use CheckMatch::{Any, Exact, OneOf, Prefix, Unjudged};

/// 熱の温度範囲の判定は上限側（`range_high:<節点>:<種類>`）と下限側（`range_low:<節点>:<種類>`）に分かれる。
/// 最高温度・非定常の最高温度の KPI は上限側、最低温度の KPI は下限側だけを見る（反対側の違反を巻き込まない）。
/// 非定常の最低温度の KPI は無い（作るときは下限側を使う）。
const THERMAL_HIGH: CheckMatch = Prefix("range_high:");
const THERMAL_LOW: CheckMatch = Prefix("range_low:");

pub(super) fn rule(key: KpiKey) -> Rule {
    use KpiKey as K;
    let (checks, fill, tally) = match key {
        // 最大 DoD（噴射中の収支の分も同じ `dod`）。周回の収支（`energy_balance`）は別の量
        K::MaxDod | K::ThrustMaxDod => (Exact("dod"), Fill::Value, false),
        K::StorageMargin => (Exact("storage_margin"), Fill::Value, false),
        K::SapMargin => (Exact("sap_margin"), Fill::Value, false),
        K::BatteryMargin => (Exact("battery_margin"), Fill::Value, false),
        K::TankVolumeMargin => (Exact("tank_volume"), Fill::Value, false),
        K::ImpulseMargin => (Exact("total_impulse"), Fill::Value, false),
        // 解析全体を代表する KPI。節点ごとの判定の数（`tally`）と OK の割合で見せる
        K::NodesInRange => (Any, Fill::OkShare, true),
        K::OrbitBalance => (Exact("energy_balance"), Fill::None, false),
        // 軌道寿命は、ミッション期間との比較と寿命の上限の 2 つ（推薬で保てる期間 `propellant_life` は別の量）
        K::OrbitLifetime => (
            OneOf(&[Exact("lifetime_vs_mission"), Exact("max_lifetime")]),
            Fill::None,
            false,
        ),
        // 周期成分だけで容量を超えると間隔が 0 になる（`momentum_capacity`）。トルクの余裕は別の量
        K::UnloadInterval => (
            OneOf(&[Exact("unload_interval"), Exact("momentum_capacity")]),
            Fill::None,
            false,
        ),
        // 中心天体の磁場で評価できないとき（時間が無いとき）は `mtq_field_model`
        K::MtqUnloadTime => (
            OneOf(&[Exact("mtq_time"), Exact("mtq_field_model")]),
            Fill::None,
            false,
        ),
        // 捨てられない向きがあるとき（時間が無いとき）は `rcs_direction`
        K::RcsUnloadTime => (
            OneOf(&[Exact("rcs_time"), Exact("rcs_direction")]),
            Fill::None,
            false,
        ),
        K::SensorIntrusion => (Prefix("intrusion:"), Fill::None, false),
        K::SlewTime => (Exact("slew_time"), Fill::None, false),
        K::LinkMargin => (Prefix("margin:"), Fill::None, false),
        // 1 日の送信可能量は「発生量 ≤ 送信可能量」の判定、最大の滞留は「滞留 ≤ ストレージ」の判定
        K::DownlinkPerDay => (Exact("capacity"), Fill::None, false),
        K::MaxBacklog => (Exact("storage"), Fill::None, false),
        K::MaxTemperature | K::TransientMaxTemperature => (THERMAL_HIGH, Fill::None, false),
        K::MinTemperature => (THERMAL_LOW, Fill::None, false),
        K::TotalMass => (Exact("mass"), Fill::None, false),
        K::MinSafetyMargin => (Prefix("interface:"), Fill::None, false),
        K::MinFrequency => (
            OneOf(&[Prefix("panel:"), Prefix("lateral:"), Prefix("axial:")]),
            Fill::None,
            false,
        ),
        K::LongestEclipse
        | K::BetaMin
        | K::BetaMax
        | K::DecayPerDay
        | K::StationKeepingDeltaV
        | K::ModeLoad
        | K::OrbitGeneration
        | K::SunlitFraction
        | K::PeakMomentum
        | K::DataPerDay
        | K::StorageRequired
        | K::ContactPerDay
        | K::LongestTransfer
        | K::DvWithMargin
        | K::LoadedPropellant
        | K::ManeuversEnd
        | K::ComShift => (Unjudged, Fill::None, false),
    };
    Rule {
        checks,
        fill,
        tally,
        when_measured: matches!(key, K::MtqUnloadTime | K::RcsUnloadTime),
    }
}
