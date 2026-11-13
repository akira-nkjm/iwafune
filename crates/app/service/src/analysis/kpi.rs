//! KPI（系・前提条件の話題の代表値）。**どの値を代表にするか（最悪値の選び方）・判定・塗りの比・判定の数え方はここだけ。**
//! CLI（`fune analyze`）と Web（系・前提条件のページの数値帯・概要のカード）は、ここで作った `KpiView` を並べるだけにする。
//!
//! - 値: 解析ごとに出力を型に戻し（下の `kpi_table!`）、欄を引く。並び（ケース・センサ・回線・節点など）は
//!   KPI ごとに最大か最小（最悪の側）を選ぶ。次元は型から（`sat_units::typed::NamedKind`）。
//! - 判定: 判定の対象の KPI は解析の判定（`checks` の最悪）。対象でない KPI（発生量・可視時間など）は null。
//! - 塗りの比: 値そのものが比の KPI（DoD・余裕）は値、判定を数える KPI は OK の割合。
//! - 判定の数え方: 判定を数える KPI（節点の動作範囲）は、解析の判定の数と OK でない数。
//!
//! 表示名は `labels::kpi_label`。電力のモードの選び方はバジェットと同じ `budget::budget_mode`。

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
        fn measure(id: &str, output: &Value, ctx: KpiContext<'_>) -> Vec<(KpiKey, Option<Measured>)> {
            $(if id == <$a as Analysis>::ID {
                return match <<$a as Analysis>::Output>::deserialize(output) {
                    Ok(o) => vec![$((KpiKey::$key, $f(&o, ctx))),+],
                    Err(_) => keys(id).iter().map(|k| (*k, None)).collect(),
                };
            })+
            Vec::new()
        }
    };
}

kpi_table! {
    power::ModeLoad { ModeLoad => mode_load }
    power::OrbitGeneration { OrbitGeneration => orbit_generation }
    power::EnergyBalance { OrbitBalance => orbit_balance, MaxDod => max_dod }
    aocs::RwUnloading { UnloadInterval => unload_interval, PeakMomentum => peak_momentum }
    aocs::UnloadTime { MtqUnloadTime => mtq_unload_time }
    aocs::SensorFov { SensorIntrusion => sensor_intrusion }
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
    }
    thermal::ThermalTransient { TransientMaxTemperature => transient_max_temperature }
    structure::MassProperties { TotalMass => total_mass }
    structure::StaticLoad { MinSafetyMargin => min_safety_margin }
    structure::ModalEstimate { MinFrequency => min_frequency }
}

type Ctx<'a> = KpiContext<'a>;

fn mode_load(o: &power::ModeLoadOutput, ctx: Ctx<'_>) -> Option<Measured> {
    one(crate::budget::budget_mode(o, ctx.case_mode)?.total_with_growth)
}
fn orbit_generation(o: &power::OrbitGenerationOutput, _: Ctx<'_>) -> Option<Measured> {
    one(o.orbit_average)
}
fn orbit_balance(o: &power::EnergyBalanceOutput, _: Ctx<'_>) -> Option<Measured> {
    smallest(o.cases.iter().map(|c| c.orbit_average_balance))
}
fn max_dod(o: &power::EnergyBalanceOutput, _: Ctx<'_>) -> Option<Measured> {
    largest(o.cases.iter().map(|c| c.max_dod))
}
fn unload_interval(o: &aocs::RwUnloadingOutput, _: Ctx<'_>) -> Option<Measured> {
    one(o.unload_interval?)
}
fn peak_momentum(o: &aocs::RwUnloadingOutput, _: Ctx<'_>) -> Option<Measured> {
    one(o.peak_momentum)
}
fn mtq_unload_time(o: &aocs::UnloadTimeOutput, _: Ctx<'_>) -> Option<Measured> {
    one(o.mtq.as_ref()?.time)
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

/// 塗りの比の決め方。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Fill {
    /// 塗らない
    None,
    /// 値そのもの（比の KPI）
    Value,
    /// OK の判定の割合（判定を数える KPI）
    OkShare,
}

/// KPI ごとの扱い: 判定の対象か・塗りの比・判定を数えるか。
struct Rule {
    judged: bool,
    fill: Fill,
    tally: bool,
}

fn rule(key: KpiKey) -> Rule {
    use KpiKey as K;
    let (judged, fill, tally) = match key {
        K::MaxDod | K::StorageMargin => (true, Fill::Value, false),
        K::NodesInRange => (true, Fill::OkShare, true),
        K::OrbitBalance
        | K::UnloadInterval
        | K::MtqUnloadTime
        | K::SensorIntrusion
        | K::LinkMargin
        | K::DownlinkPerDay
        | K::MaxBacklog
        | K::MaxTemperature
        | K::MinTemperature
        | K::TransientMaxTemperature
        | K::TotalMass
        | K::MinSafetyMargin
        | K::MinFrequency => (true, Fill::None, false),
        K::ModeLoad
        | K::OrbitGeneration
        | K::PeakMomentum
        | K::DataPerDay
        | K::StorageRequired
        | K::ContactPerDay => (false, Fill::None, false),
    };
    Rule {
        judged,
        fill,
        tally,
    }
}

/// KPI の定義（キーと表示名）。
fn def(key: KpiKey) -> KpiDef {
    let l = kpi_label(key);
    KpiDef {
        key,
        label: l.label,
        label_en: l.label_en,
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
    let verdict = Verdict::worst(checks.iter().map(|c| c.verdict));
    let tally = KpiTally {
        checked: u32::try_from(checks.len()).unwrap_or(u32::MAX),
        flagged: u32::try_from(checks.iter().filter(|c| c.verdict != Verdict::Ok).count())
            .unwrap_or(u32::MAX),
    };
    measure(id, output, ctx)
        .into_iter()
        .map(|(key, value)| view(key, value, verdict, tally))
        .collect()
}

fn view(
    key: KpiKey,
    value: Option<Measured>,
    verdict: Option<Verdict>,
    tally: KpiTally,
) -> KpiView {
    let r = rule(key);
    let tally = r.tally.then_some(tally);
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
        verdict: if r.judged { verdict } else { None },
        fill,
        tally,
    }
}

#[cfg(test)]
mod tests;
