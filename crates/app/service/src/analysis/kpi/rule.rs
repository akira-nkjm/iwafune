//! KPI ごとの扱い（`Rule`）: どの check に結ぶか（判定の対象か）・塗りの比・判定を数えるか・測れたときだけ出すか。
//! **KPI と解析の check の対応はここだけ。** check のキーは sat-analysis が決める（キーを変えると `rule_tests` が落ちる）。

use crate::dto::{CheckView, KpiKey, KpiTally, OverviewKpi};
use sat_analysis::spec::Verdict;
use sat_analysis::thermal::range_item;
use std::collections::HashMap;

/// KPI の判定にする check の選び方（`CheckView.key` との照合）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum CheckMatch {
    /// 判定の対象でない KPI（発生量・可視時間など。判定は null）
    Unjudged,
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
            Self::Unjudged => Vec::new(),
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

/// 判定の数え方。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum Tally {
    /// 数えない
    None,
    /// 温度範囲の項目（節点 × 種類）ごとに 1 つ数え、項目の判定は項目の check の最悪。上限側と下限側（`range_high:` / `range_low:`）は
    /// 同じ節点・種類の 1 項目にまとめる（側ごとに数えると分母が倍になる）。温度範囲でない check（ヒータなど）は数えない
    Items,
}

impl Tally {
    /// 判定の数（数えない KPI は None）。
    pub(super) fn count(self, checks: &[CheckView]) -> Option<KpiTally> {
        match self {
            Self::None => None,
            Self::Items => {
                // 項目: 温度範囲は節点:種類。範囲でない check は数えない。並びは数えるだけなので順序は問わない
                let mut items: HashMap<&str, bool> = HashMap::new();
                for c in checks {
                    let Some(id) = range_item(&c.key) else {
                        continue;
                    };
                    // 項目の判定は最悪。OK でないのが 1 つでもあれば項目は OK でない
                    *items.entry(id).or_insert(false) |= c.verdict != Verdict::Ok;
                }
                let count = |n: usize| u32::try_from(n).unwrap_or(u32::MAX);
                Some(KpiTally {
                    checked: count(items.len()),
                    flagged: count(items.values().filter(|flagged| **flagged).count()),
                })
            }
        }
    }
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
    pub(super) tally: Tally,
}

use CheckMatch::{Exact, OneOf, Prefix, Unjudged};

/// 熱の温度範囲の判定は上限側（`range_high:<節点>:<種類>`）と下限側（`range_low:<節点>:<種類>`）に分かれる。
/// 最高温度・非定常の最高温度の KPI は上限側、最低温度の KPI は下限側だけを見る（反対側の違反を巻き込まない）。
/// 非定常の最低温度の KPI は無い（作るときは下限側を使う）。
const THERMAL_HIGH: CheckMatch = Prefix("range_high:");
const THERMAL_LOW: CheckMatch = Prefix("range_low:");
/// ヒータの判定（`heater:<部品>`。温度範囲の判定ではない）。
const THERMAL_HEATER: CheckMatch = Prefix("heater:");

pub(super) fn rule(key: KpiKey) -> Rule {
    use KpiKey as K;
    let (checks, fill, tally) = match key {
        // 最大 DoD（噴射中の収支の分も同じ `dod`）。周回の収支（`energy_balance`）は別の量
        K::MaxDod | K::ThrustMaxDod => (Exact("dod"), Fill::Value, Tally::None),
        K::StorageMargin => (Exact("storage_margin"), Fill::Value, Tally::None),
        K::SapMargin => (Exact("sap_margin"), Fill::Value, Tally::None),
        K::BatteryMargin => (Exact("battery_margin"), Fill::Value, Tally::None),
        K::TankVolumeMargin => (Exact("tank_volume"), Fill::Value, Tally::None),
        K::ImpulseMargin => (Exact("total_impulse"), Fill::Value, Tally::None),
        // 温度範囲の判定（`range_high:` と `range_low:`）の数（`tally`）と OK の割合で見せる。ヒータなど範囲でない判定は数えない
        K::NodesInRange => (
            OneOf(&[THERMAL_HIGH, THERMAL_LOW]),
            Fill::OkShare,
            Tally::Items,
        ),
        K::OrbitBalance => (Exact("energy_balance"), Fill::None, Tally::None),
        // 軌道寿命は、ミッション期間との比較と寿命の上限の 2 つ（推薬で保てる期間 `propellant_life` は別の量）
        K::OrbitLifetime => (
            OneOf(&[Exact("lifetime_vs_mission"), Exact("max_lifetime")]),
            Fill::None,
            Tally::None,
        ),
        // 周期成分だけで容量を超えると間隔が 0 になる（`momentum_capacity`）。トルクの余裕は別の量
        K::UnloadInterval => (
            OneOf(&[Exact("unload_interval"), Exact("momentum_capacity")]),
            Fill::None,
            Tally::None,
        ),
        // 中心天体の磁場で評価できないとき（時間が無いとき）は `mtq_field_model`
        K::MtqUnloadTime => (
            OneOf(&[Exact("mtq_time"), Exact("mtq_field_model")]),
            Fill::None,
            Tally::None,
        ),
        // 捨てられない向きがあるとき（時間が無いとき）は `rcs_direction`
        K::RcsUnloadTime => (
            OneOf(&[Exact("rcs_time"), Exact("rcs_direction")]),
            Fill::None,
            Tally::None,
        ),
        K::SensorIntrusion => (Prefix("intrusion:"), Fill::None, Tally::None),
        K::SlewTime => (Exact("slew_time"), Fill::None, Tally::None),
        K::LinkMargin => (Prefix("margin:"), Fill::None, Tally::None),
        // 1 日の送信可能量は「発生量 ≤ 送信可能量」の判定、最大の滞留は「滞留 ≤ ストレージ」の判定
        K::DownlinkPerDay => (Exact("capacity"), Fill::None, Tally::None),
        K::MaxBacklog => (Exact("storage"), Fill::None, Tally::None),
        K::MaxTemperature | K::TransientMaxTemperature => (THERMAL_HIGH, Fill::None, Tally::None),
        K::MinTemperature => (THERMAL_LOW, Fill::None, Tally::None),
        K::MaxHeaterDuty | K::TransientMaxHeaterDuty => (THERMAL_HEATER, Fill::Value, Tally::None),
        K::TotalMass => (Exact("mass"), Fill::None, Tally::None),
        K::MinSafetyMargin => (Prefix("interface:"), Fill::None, Tally::None),
        K::MinFrequency => (
            OneOf(&[Prefix("panel:"), Prefix("lateral:"), Prefix("axial:")]),
            Fill::None,
            Tally::None,
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
        | K::ComShift => (Unjudged, Fill::None, Tally::None),
    };
    Rule {
        checks,
        fill,
        tally,
        when_measured: matches!(key, K::MtqUnloadTime | K::RcsUnloadTime),
    }
}

/// 概要のカード: 見出しにする KPI と、フッターに値を添える KPI。**概要に出す KPI の選び方と並びはここだけ**（web は一覧を持たない）。
/// 並びは下の表の順（`OverviewKpi.order`）。測れない KPI のカードは web が出さない（値も判定も判定の数も無いとき）。
const OVERVIEW: &[(KpiKey, &[KpiKey])] = &[
    (KpiKey::OrbitBalance, &[]),
    (KpiKey::MaxDod, &[]),
    (KpiKey::UnloadInterval, &[KpiKey::PeakMomentum]),
    (KpiKey::StorageMargin, &[KpiKey::DataPerDay]),
    (KpiKey::LinkMargin, &[]),
    (
        KpiKey::NodesInRange,
        &[KpiKey::MaxTemperature, KpiKey::MinTemperature],
    ),
    (KpiKey::DvWithMargin, &[KpiKey::LoadedPropellant]),
];

/// 概要のカードの見出しにする KPI なら、その並びとフッターの KPI。
pub(super) fn overview(key: KpiKey) -> Option<OverviewKpi> {
    let (order, (_, footer)) = OVERVIEW.iter().enumerate().find(|(_, (k, _))| *k == key)?;
    Some(OverviewKpi {
        order: u32::try_from(order).unwrap_or(u32::MAX),
        footer: footer.to_vec(),
    })
}
