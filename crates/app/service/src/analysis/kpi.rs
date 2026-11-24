//! KPI（系・前提条件の話題の代表値）。**どの値を代表にするか（最悪値の選び方）・判定・塗りの比・判定の数え方はここだけ。**
//! CLI（`fune analyze`）と Web（系・前提条件のページの数値帯・概要のカード）は、ここで作った `KpiView` を並べるだけにする。
//!
//! - 値: 解析ごとに出力を型に戻し（下の `kpi_table!`）、欄を引く。並び（ケース・センサ・回線・節点など）は
//!   KPI ごとに最大か最小（最悪の側）を選ぶ。次元は型から（`sat_units::typed::NamedKind`）。
//!   電力収支はケースの判定に使った運用の値（最長継続時間で評価したケースは `by_max_duration`）とモードの並びから選ぶ。
//! - 判定: 判定の対象の KPI は、対応する check（`rule::CheckMatch`。キーの完全一致か接頭辞）の最悪。対応する check が無ければ null、
//!   対象でない KPI（発生量・可視時間など）も null。節点の動作範囲の KPI は温度範囲の check（`range_high:`・`range_low:`）の最悪で、ヒータの判定（`heater:`）は含めない。
//! - 塗りの比: 値そのものが比の KPI（DoD・余裕）は値、判定を数える KPI は OK の割合。
//! - 判定の数え方: 判定を数える KPI（節点の動作範囲）は、温度範囲の項目（節点 × 範囲の種類）の数と OK でない数（範囲でない check は数えない）。
//!   上限側と下限側の判定は 1 項目にまとめ、項目の判定は両側の悪いほう（`rule::Tally`）。
//!
//! 表示名は `labels::kpi_label`。電力のモードの選び方はバジェットと同じ `budget::budget_mode`。

use self::rule::{Fill, overview, rule};
use super::labels::kpi_label;
use crate::dto::{CheckView, FillBasis, FillRatio, KpiDef, KpiKey, KpiTally, KpiView};
use crate::units::quantity_auto;
use sat_analysis::spec::{Analysis, Verdict};
use sat_analysis::{aocs, cdh, comm, power, structure, thermal};
use sat_units::typed::{Decibel, Dimension, Fraction, NamedKind, Qty};
use serde::Deserialize;
use serde_json::Value;

/// KPI の値を引くときに、出力の外から要るもの。
#[derive(Debug, Clone, Copy, Default)]
pub(crate) struct KpiContext<'a> {
    /// 解析ケースのモード（電力のバジェットのモードを選ぶのに使う。ケースが無いかモードが無ければ None）
    pub case_mode: Option<&'a str>,
}

/// KPI の読み: 量（値）か、数えた結果（判定を数える KPI。干渉・円錐の遮り）。
#[derive(Debug, Clone, Copy, PartialEq)]
enum Reading {
    Value(Measured),
    Count(KpiTally),
}

/// 値の関数の戻り（量・数）を `Reading` にする。`kpi_table!` が値の型を問わず引けるようにする。
trait IntoReading {
    fn into_reading(self) -> Option<Reading>;
}

impl IntoReading for Option<Measured> {
    fn into_reading(self) -> Option<Reading> {
        self.map(Reading::Value)
    }
}

impl IntoReading for Option<KpiTally> {
    fn into_reading(self) -> Option<Reading> {
        self.map(Reading::Count)
    }
}

/// KPI の量: SI の値と、表示の種類の名前（型から）。
#[derive(Debug, Clone, Copy, PartialEq)]
struct Measured {
    si: f64,
    kind: &'static str,
}

/// 型付きの値 → KPI の量。表示の種類は型の名前（`NamedKind`）なので、次元を文字列で書かない。
trait Measure: Copy {
    fn measured(self) -> Measured;
}

impl<D: Dimension> Measure for Qty<D> {
    fn measured(self) -> Measured {
        Measured {
            si: self.value(),
            kind: <D as NamedKind>::name(),
        }
    }
}

impl Measure for Fraction {
    fn measured(self) -> Measured {
        Measured {
            si: self.value(),
            kind: <Fraction as NamedKind>::name(),
        }
    }
}

impl Measure for Decibel {
    fn measured(self) -> Measured {
        Measured {
            si: self.value(),
            kind: <Decibel as NamedKind>::name(),
        }
    }
}

/// 並びのうち最大（有限の値だけ。無ければ None）。最悪が大きい側の KPI（DoD・温度・侵入の割合）に使う。
fn largest<M: Measure>(values: impl IntoIterator<Item = M>) -> Option<Measured> {
    pick(values, |a, b| a > b)
}

/// 並びのうち最小（有限の値だけ。無ければ None）。最悪が小さい側の KPI（収支・余裕・固有振動数）に使う。
fn smallest<M: Measure>(values: impl IntoIterator<Item = M>) -> Option<Measured> {
    pick(values, |a, b| a < b)
}

fn pick<M: Measure>(
    values: impl IntoIterator<Item = M>,
    better: impl Fn(f64, f64) -> bool,
) -> Option<Measured> {
    values
        .into_iter()
        .map(Measure::measured)
        .filter(|m| m.si.is_finite())
        .reduce(|a, b| if better(b.si, a.si) { b } else { a })
}

/// 1 つの値（有限でなければ None）。
fn one<M: Measure>(value: M) -> Option<Measured> {
    smallest([value])
}

macro_rules! kpi_table {
    ($($a:ty { $($key:ident => $f:path),+ $(,)? })+) => {
        /// 解析が出す KPI のキー（所属する区分のページに並べる順）。KPI を出さない解析は空。
        fn keys(id: &str) -> &'static [KpiKey] {
            $(if id == <$a as Analysis>::ID {
                return &[$(KpiKey::$key),+];
            })+
            &[]
        }

        /// 出力を解析の型に戻し、KPI ごとの値を引く（`keys` と同じ並び）。型に戻せなければ値はすべて None。
        fn measure(id: &str, output: &Value, ctx: KpiContext<'_>) -> Vec<(KpiKey, Option<Reading>)> {
            $(if id == <$a as Analysis>::ID {
                return match <<$a as Analysis>::Output>::deserialize(output) {
                    Ok(o) => vec![$((KpiKey::$key, $f(&o, ctx).into_reading())),+],
                    Err(_) => keys(id).iter().map(|k| (*k, None)).collect(),
                };
            })+
            Vec::new()
        }
    };
}

kpi_table! {
    sat_analysis::orbit::EclipseSeason { LongestEclipse => orbit::longest_eclipse, BetaMin => orbit::beta_min, BetaMax => orbit::beta_max }
    sat_analysis::orbit::OrbitDecay {
        OrbitLifetime => orbit::orbit_lifetime,
        DecayPerDay => orbit::decay_per_day,
        StationKeepingDeltaV => orbit::station_keeping_delta_v,
    }
    power::ModeLoad { ModeLoad => power_kpi::mode_load }
    power::OrbitGeneration {
        OrbitGeneration => power_kpi::orbit_generation,
        SunlitFraction => power_kpi::sunlit_fraction,
    }
    power::EnergyBalance { OrbitBalance => power_kpi::orbit_balance, MaxDod => power_kpi::max_dod }
    power::Sizing { SapMargin => power_kpi::sap_margin, BatteryMargin => power_kpi::battery_margin }
    aocs::RwUnloading { UnloadInterval => unload_interval, PeakMomentum => peak_momentum }
    aocs::UnloadTime { MtqUnloadTime => mtq_unload_time, RcsUnloadTime => rcs_unload_time }
    aocs::SensorFov { SensorIntrusion => sensor_intrusion }
    aocs::SlewTime { SlewTime => slew_time }
    cdh::DataBudget {
        DataPerDay => data_per_day,
        StorageRequired => storage_required,
        StorageMargin => storage_margin,
    }
    comm::LinkBudget { LinkMargin => link_margin }
    comm::Visibility { ContactPerDay => contact_per_day }
    comm::DataThroughput { DownlinkPerDay => downlink_per_day, MaxBacklog => max_backlog }
    thermal::ThermalSteady {
        MaxTemperature => max_temperature,
        MinTemperature => min_temperature,
        NodesInRange => counted_only,
        MaxHeaterDuty => max_heater_duty,
    }
    thermal::ThermalTransient {
        TransientMaxTemperature => transient_max_temperature,
        TransientMaxHeaterDuty => transient_max_heater_duty,
    }
    structure::MassProperties { TotalMass => total_mass }
    structure::StaticLoad { MinSafetyMargin => min_safety_margin }
    structure::ModalEstimate { MinFrequency => min_frequency }
    structure::LaunchFit { LaunchFitMargin => structure_kpi::launch_fit_margin }
    structure::Interference { InterferenceOverlaps => structure_kpi::interference_overlaps }
    structure::ConeClearance { ConeObstructions => structure_kpi::cone_obstructions }
    structure::ExternalGeometryReport { MeanProjectedArea => structure_kpi::projected_area_mean }
    sat_analysis::orbit::Geometry { GreatestEarthDistance => orbit::greatest_earth_distance }
    sat_analysis::propulsion::Maneuvers { LongestTransfer => propulsion::longest_transfer }
    sat_analysis::propulsion::DvBudget { DvWithMargin => propulsion::dv_with_margin }
    sat_analysis::propulsion::Propellant {
        LoadedPropellant => propulsion::loaded_propellant,
        TankVolumeMargin => propulsion::tank_volume_margin,
    }
    sat_analysis::propulsion::ThrusterUsage { ImpulseMargin => propulsion::impulse_margin }
    sat_analysis::propulsion::TransferCheck { TransferMiss => propulsion::transfer_miss }
    sat_analysis::propulsion::LaunchWindow { LaunchWindowDeltaV => propulsion::launch_window_delta_v }
    sat_analysis::propulsion::OrbitHistoryAnalysis { ManeuversEnd => propulsion::maneuvers_end }
    sat_analysis::propulsion::PropellantMassProperties { ComShift => propulsion::com_shift }
    power::ThrustBalance { ThrustMaxDod => propulsion::thrust_max_dod }
}

type Ctx<'a> = KpiContext<'a>;

fn unload_interval(o: &aocs::RwUnloadingOutput, _: Ctx<'_>) -> Option<Measured> {
    one(o.unload_interval?)
}
fn peak_momentum(o: &aocs::RwUnloadingOutput, _: Ctx<'_>) -> Option<Measured> {
    one(o.peak_momentum)
}
/// MTQ でのアンロード時間（MTQ を評価していなければ None）。
fn mtq_unload_time(o: &aocs::UnloadTimeOutput, _: Ctx<'_>) -> Option<Measured> {
    one(o.mtq.as_ref()?.time)
}
/// RCS でのアンロード時間（最悪の向き。スラスタを評価していないか、捨てられない向きがあれば None）。
/// MTQ と RCS は別の手段で、解析の出力はどちらを使うかを持たないので、両方あれば 2 つの KPI として並べる。
fn rcs_unload_time(o: &aocs::UnloadTimeOutput, _: Ctx<'_>) -> Option<Measured> {
    one(o.rcs.as_ref()?.worst.time?)
}
fn slew_time(o: &aocs::SlewTimeOutput, _: Ctx<'_>) -> Option<Measured> {
    one(o.worst.time)
}
fn sensor_intrusion(o: &aocs::SensorFovOutput, _: Ctx<'_>) -> Option<Measured> {
    largest(o.sensors.iter().map(|s| s.intrusion_fraction))
}
fn data_per_day(o: &cdh::DataBudgetOutput, _: Ctx<'_>) -> Option<Measured> {
    one(o.total_per_day)
}
fn storage_required(o: &cdh::DataBudgetOutput, _: Ctx<'_>) -> Option<Measured> {
    one(o.storage_required)
}
fn storage_margin(o: &cdh::DataBudgetOutput, _: Ctx<'_>) -> Option<Measured> {
    one(o.capacity_margin?)
}
fn link_margin(o: &comm::LinkBudgetOutput, _: Ctx<'_>) -> Option<Measured> {
    smallest(o.links.iter().map(|l| l.margin_db))
}
fn contact_per_day(o: &comm::VisibilityOutput, _: Ctx<'_>) -> Option<Measured> {
    one(o.combined.contact_per_day)
}
fn downlink_per_day(o: &comm::DataThroughputOutput, _: Ctx<'_>) -> Option<Measured> {
    one(o.capacity_per_day)
}
fn max_backlog(o: &comm::DataThroughputOutput, _: Ctx<'_>) -> Option<Measured> {
    one(o.max_backlog)
}
fn max_temperature(o: &thermal::ThermalSteadyOutput, _: Ctx<'_>) -> Option<Measured> {
    largest(o.envelope.iter().map(|n| n.max))
}
fn min_temperature(o: &thermal::ThermalSteadyOutput, _: Ctx<'_>) -> Option<Measured> {
    smallest(o.envelope.iter().map(|n| n.min))
}
/// ヒータのデューティの最大（ケースをまたいで、ヒータのうち最大）。ヒータが無ければ None。
fn max_heater_duty(o: &thermal::ThermalSteadyOutput, _: Ctx<'_>) -> Option<Measured> {
    largest(
        o.cases
            .iter()
            .flat_map(|c| c.heaters.iter().map(|h| h.duty_fraction)),
    )
}
/// 非定常のヒータの、最後の 1 周のデューティの最大（判定と同じ量）。ヒータが無ければ None。
fn transient_max_heater_duty(o: &thermal::ThermalTransientOutput, _: Ctx<'_>) -> Option<Measured> {
    largest(o.heaters.iter().map(|h| h.duty_fraction_last_orbit))
}
fn transient_max_temperature(o: &thermal::ThermalTransientOutput, _: Ctx<'_>) -> Option<Measured> {
    largest(o.nodes.iter().map(|n| n.max))
}
fn total_mass(o: &structure::MassPropertiesOutput, _: Ctx<'_>) -> Option<Measured> {
    one(o.summary.total_mass)
}
fn min_safety_margin(o: &structure::StaticLoadOutput, _: Ctx<'_>) -> Option<Measured> {
    smallest(o.mounts.iter().filter_map(|m| m.min_margin))
}
fn min_frequency(o: &structure::ModalEstimateOutput, _: Ctx<'_>) -> Option<Measured> {
    smallest(o.results.iter().map(|r| r.frequency))
}
/// 値を持たず、判定を数えるだけの KPI。
fn counted_only<T>(_: &T, _: Ctx<'_>) -> Option<Measured> {
    None
}

/// KPI の定義（キーと表示名）。
fn def(key: KpiKey) -> KpiDef {
    let l = kpi_label(key);
    KpiDef {
        key,
        label: l.label,
        label_en: l.label_en,
        when_measured: rule(key).when_measured,
        overview: overview(key),
    }
}

/// 解析が出す KPI の定義（`GET /analyses` の `AnalysisEntry.kpis`）。
pub(crate) fn defs(id: &str) -> Vec<KpiDef> {
    keys(id).iter().map(|k| def(*k)).collect()
}

/// 解析の結果の KPI（`AnalysisRunResult.kpis`）。出力が無ければ空。`checks` はその結果の判定（`checks::collect_checks`）。
pub(crate) fn kpis(
    id: &str,
    output: Option<&Value>,
    checks: &[CheckView],
    ctx: KpiContext<'_>,
) -> Vec<KpiView> {
    let Some(output) = output else {
        return Vec::new();
    };
    let judged = measure(id, output, ctx)
        .into_iter()
        .map(|(key, value)| (key, value, rule(key).checks.verdict(checks)))
        .collect();
    only_measured(judged)
        .into_iter()
        .map(|(key, value, verdict)| view(key, value, verdict, checks))
        .collect()
}

/// KPI の値と、対応する check の判定。
type Judged = (KpiKey, Option<Reading>, Option<Verdict>);

/// 値も判定も無い「測れたときだけ出す」KPI（設計が持たない手段）を外す（すべて外れるなら先頭の 1 つを残す）。
/// 値が無くても判定があれば残す（RCS だけの設計で捨てられない向きがあるときの RCS の違反を、KPI の行に出すため）。
fn only_measured(all: Vec<Judged>) -> Vec<Judged> {
    let keep =
        |(k, v, verdict): &Judged| v.is_some() || verdict.is_some() || !rule(*k).when_measured;
    if !all.iter().any(keep) {
        return all.into_iter().take(1).collect();
    }
    all.into_iter().filter(keep).collect()
}

fn view(
    key: KpiKey,
    reading: Option<Reading>,
    verdict: Option<Verdict>,
    checks: &[CheckView],
) -> KpiView {
    let r = rule(key);
    let (value, counted) = match reading {
        Some(Reading::Value(m)) => (Some(m), None),
        Some(Reading::Count(t)) => (None, Some(t)),
        None => (None, None),
    };
    let tally = r.tally.count(checks).or(counted);
    let fill = match r.fill {
        Fill::None => None,
        Fill::Value => value.and_then(|v| FillRatio::of_value(v.si, FillBasis::Value)),
        Fill::OkShare => tally.and_then(KpiTally::ok_share),
    };
    let d = def(key);
    KpiView {
        key,
        label: d.label,
        label_en: d.label_en,
        value: value.map(|v| quantity_auto(v.si, v.kind)),
        verdict: if r.checks.is_judged() { verdict } else { None },
        fill,
        tally,
    }
}

mod orbit;
mod power_kpi;
mod propulsion;
mod rule;
mod structure_kpi;

#[cfg(test)]
mod rule_tests;
#[cfg(test)]
pub(crate) mod tests;
