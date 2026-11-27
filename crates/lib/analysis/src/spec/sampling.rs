//! 時系列のサンプルの取り方の既定（周回数・刻み・1 日）と、設計が書かなかったときにそれを当てる構築。
//! **時系列の期間と刻みの既定はここだけ**。
//! - 期間: 周回数が書かれていなければ [`DEFAULT_ORBITS`]。1 周で数える量（周回平均・割合）は [`SamplingSpan::OrbitAverage`]（1 周）。
//! - 太陽を回る軌道（惑星間の巡航。周期が数か月〜数年）では、周回数が書かれていなければ 1 周にする（[`DEFAULT_HELIOCENTRIC_ORBITS`]。
//!   3 周では数年の時系列になり、積分の費用も 3 倍になる。書かれた周回数はそのまま使う）。
//! - 刻み: [`orbit_step_s`]。周期の一定の割合（1 周あたり最大 [`MAX_STEPS_PER_ORBIT`] サンプル）を超えない刻み（下限 `sat_astro::timeline::DEFAULT_STEP_S`）。
//!   周期が 100 分までの周回軌道では従来どおり 30 s のまま、静止軌道や月周回のような長い周期でも 1 周のサンプル数が増えすぎない。
//! - 1 回限りの軌道（[`Cycle::OneOff`]。日付つきの遷移の巡航など）: 周回数は使わず（書かれていても無視）、期間そのものを解く。
//!   刻みは周期と同じ規則で期間から決める（`max(30 s, 期間 / 200)`。300 日の巡航で 1.5 日、サンプルは 200 点）。
//!   周回の集計（`orbit_ranges`）は期間全体を 1 周回として数える。設計が刻みを書いたとき（熱の `time_step`）は、周期的な軌道ではそのまま、
//!   1 回限りの軌道では既定の刻みより細かくしない（[`requested_step_s`]）。
//!
//! 設計から入力を作る側（`fune-projection`）は既定を持たず、書かれた値（`None` かもしれない）か、解析が決める期間の種類
//! （[`SamplingSpan`]）を渡す。

use sat_astro::constants::SECONDS_PER_DAY;
use sat_astro::ephem::CentralBody;
use sat_astro::orbit::{Cycle, Trajectory};
use sat_astro::timeline::{DEFAULT_STEP_S, OrbitSampling};
use sat_units::typed::Qty;

/// 周回数が書かれていないときに解く周回数（電源の収支・熱の非定常・RW のアンロードで共通）。
/// 電池の充放電や熱の履歴が初期条件から落ち着いて見える程度の数として、概念検討で使う 3 周とする。
pub const DEFAULT_ORBITS: u32 = 3;

/// 太陽を回る軌道で、周回数が書かれていないときに解く周回数（[`DEFAULT_ORBITS`] の代わり）。
pub const DEFAULT_HELIOCENTRIC_ORBITS: u32 = 1;

/// 1 周の平均や割合を数える量（センサの禁止角への侵入の割合・アンロード時間の地磁気の平均・熱の軌道平均）が解く周回数。
/// 周期的な量なので 1 周で足りる（複数周は同じ値の繰り返し）。
pub const ORBIT_AVERAGE_ORBITS: u32 = 1;

/// 1 周あたりのサンプル数の上限（[`orbit_step_s`]）。100 分（6000 s）の周回軌道が下限の刻み 30 s と一致する 200 サンプル。
pub const MAX_STEPS_PER_ORBIT: f64 = 200.0;

/// 時系列の期間の取り方。解析が自分の量に合う期間を選び、設計は周回数だけを書く。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SamplingSpan {
    /// 履歴を見る期間。書かれた周回数（無ければ [`DEFAULT_ORBITS`]、0 は 1 周）
    Series(Option<u32>),
    /// 1 周の平均・割合を数える期間（[`ORBIT_AVERAGE_ORBITS`] 周。設計の周回数は使わない）
    OrbitAverage,
}

impl SamplingSpan {
    /// 解く周回数。
    pub fn orbits(self) -> u32 {
        match self {
            Self::Series(orbits) => orbit_count(orbits),
            Self::OrbitAverage => ORBIT_AVERAGE_ORBITS,
        }
    }
}

/// 解く周回数。書かれていなければ [`DEFAULT_ORBITS`]、0 は 1 周に直す。
pub fn orbit_count(orbits: Option<u32>) -> u32 {
    orbits.unwrap_or(DEFAULT_ORBITS).max(1)
}

/// 周期 `period_s` の軌道の時系列の既定の刻み [s]: 下限 `floor_s`、ただし 1 周あたり `max_steps_per_orbit` サンプルを超えない
/// （`max(floor_s, period / max_steps_per_orbit)`）。**刻みを周期の割合で決める規則はここだけ**（影のサンプル `SHADOW_STEP_S` も同じ規則）。
pub fn step_for_period(period_s: f64, floor_s: f64, max_steps_per_orbit: f64) -> f64 {
    floor_s.max(period_s / max_steps_per_orbit)
}

/// 繰り返し `cycle` の軌道の時系列の既定の刻み [s]（[`step_for_period`] に 30 s・[`MAX_STEPS_PER_ORBIT`]。
/// 長さは [`Cycle::length_s`]）。1 回限りの軌道（[`Cycle::OneOff`]）は期間を周期とみなして同じ式で刻む
/// （`max(30 s, 期間 / 200)`。期間が 100 分までなら 30 s、300 日なら 1.5 日）。
pub fn orbit_step_s(cycle: Cycle) -> f64 {
    step_for_period(cycle.length_s(), DEFAULT_STEP_S, MAX_STEPS_PER_ORBIT)
}

/// 設計が決めた刻み `requested` を、軌道の繰り返しに合わせて使う刻み [s] にする。周期的な軌道はそのまま（周回の分解能の指定）。
/// 1 回限りの軌道（[`Cycle::OneOff`]）は、期間から決めた既定の刻み（[`orbit_step_s`]）より細かくしない
/// （数か月の巡航を周回の刻み 30 s で刻むとサンプルが数十万点になるため。粗くする指定はそのまま）。
pub fn requested_step_s(cycle: Cycle, requested: f64) -> f64 {
    match cycle {
        Cycle::Periodic { .. } => requested,
        Cycle::OneOff { .. } => requested.max(orbit_step_s(cycle)),
    }
}

/// 期間の取り方 `span` のサンプルの取り方（刻みは繰り返しの長さから、β の指定なし）。
/// 1 回限りの軌道（[`Cycle::OneOff`]）は周回数を使わず、その期間そのものを解く（周回数の指定は無視）。
pub fn sampling(orbit: impl Into<Trajectory>, span: SamplingSpan) -> OrbitSampling {
    let orbit = orbit.into();
    let cycle = orbit.cycle();
    let orbits = match (cycle, span) {
        (Cycle::OneOff { .. }, _) => 1,
        (_, SamplingSpan::Series(None)) if orbit.central() == CentralBody::Sun => {
            DEFAULT_HELIOCENTRIC_ORBITS
        }
        _ => span.orbits(),
    };
    OrbitSampling::new(
        orbit,
        Qty::from_raw(cycle.length_s() * f64::from(orbits)),
        Qty::from_raw(orbit_step_s(cycle)),
    )
}

/// 軌道 [`orbit_count`] 周分のサンプルの取り方（刻みは周期から、β の指定なし）。
pub fn orbits_sampling(orbit: impl Into<Trajectory>, orbits: Option<u32>) -> OrbitSampling {
    sampling(orbit, SamplingSpan::Series(orbits))
}

/// 1 日分のサンプル（地上局の可視・送信量）の刻みの上限 [s]。パスの出入りは地球の自転（15°/h）で決まるので、
/// 周期の長い軌道（日心軌道など。周期の割合の刻みが 1 日を超える）でも 10 分より粗くしない。
/// 静止軌道までの周期（刻み 430 s 以下）では周期から決めた刻みのまま。
pub const MAX_DAILY_STEP_S: f64 = 600.0;

/// 1 日分のサンプルの取り方（地上局の可視・送信量。刻みは周期から、ただし [`MAX_DAILY_STEP_S`] まで。β の指定なし）。
/// 1 日より短い 1 回限りの軌道（[`Cycle::OneOff`]）は、その期間まで（軌道の外は解かない）。
pub fn daily_sampling(orbit: impl Into<Trajectory>) -> OrbitSampling {
    let orbit = orbit.into();
    let cycle = orbit.cycle();
    let step = orbit_step_s(cycle).min(MAX_DAILY_STEP_S);
    let span = match cycle {
        Cycle::OneOff { span_s } => span_s.min(SECONDS_PER_DAY),
        Cycle::Periodic { .. } => SECONDS_PER_DAY,
    };
    OrbitSampling::new(orbit, Qty::from_raw(span), Qty::from_raw(step))
}

#[cfg(test)]
mod tests {
    use super::*;
    use sat_astro::orbit::{OrbitElements, OrbitSpec, Propagator};
    use sat_astro::time::Epoch;

    fn periodic(period_s: f64) -> Cycle {
        Cycle::Periodic { period_s }
    }

    fn orbit() -> OrbitSpec {
        let elements = OrbitElements::with_defaults(
            Qty::from_raw(7.0e6),
            Qty::from_raw(0.9),
            None,
            None,
            None,
            None,
        );
        OrbitSpec::new(
            Epoch::parse("2027-01-01T00:00:00Z").expect("epoch"),
            elements,
            Propagator::Kepler,
        )
    }

    /// 周回数を書かなければ 3 周、書けばその周回数、0 は 1 周。刻みは既定。
    #[test]
    fn orbit_count_defaults_and_clamps() {
        let o = orbit();
        let period = o.period_s();
        assert_eq!(orbit_count(None), 3);
        assert_eq!(orbit_count(Some(0)), 1);
        let s = orbits_sampling(o.clone(), None);
        assert!((s.span.value() - 3.0 * period).abs() < 1e-9);
        // 軌道は半長軸 7000 km（周期 5828 s < 6000 s）なので従来の 30 s
        assert_eq!(s.step.value(), DEFAULT_STEP_S);
        let s = orbits_sampling(o, Some(5));
        assert!((s.span.value() - 5.0 * period).abs() < 1e-9);
    }

    /// 刻みは周期の割合で決まる: 100 分（6000 s）までは従来の 30 s のまま（1 周 200 サンプル以下）、
    /// それより長い周期は周期 / 200（静止軌道 86 164 s なら 430.82 s、1 周は 200 サンプル）。
    #[test]
    fn step_is_a_fraction_of_the_period_with_a_floor() {
        assert_eq!(orbit_step_s(periodic(5400.0)), 30.0);
        assert_eq!(orbit_step_s(periodic(6000.0)), 30.0);
        assert!((orbit_step_s(periodic(86_164.0)) - 86_164.0 / 200.0).abs() < 1e-12);
        // 1 周のサンプル数は、周期が 6000 s 以下なら周期 / 30 s（従来どおり）、それ以上は 200 で頭打ち
        let steps = |period: f64| period / orbit_step_s(periodic(period));
        assert_eq!(steps(5400.0), 180.0);
        assert!((steps(86_164.0) - 200.0).abs() < 1e-9);
        assert!((steps(6.0e5) - 200.0).abs() < 1e-9);
        // 影のサンプルも同じ規則（下限 5 s・1 周 1200 サンプル）
        assert_eq!(step_for_period(5700.0, 5.0, 1200.0), 5.0);
        assert!((step_for_period(86_164.0, 5.0, 1200.0) - 86_164.0 / 1200.0).abs() < 1e-12);
    }

    /// 期間の取り方: 履歴を見る期間は書かれた周回数（無ければ 3）、周回平均は常に 1 周。
    #[test]
    fn spans_choose_their_orbits() {
        let o = orbit();
        let period = o.period_s();
        assert_eq!(SamplingSpan::Series(None).orbits(), 3);
        assert_eq!(SamplingSpan::Series(Some(5)).orbits(), 5);
        assert_eq!(SamplingSpan::OrbitAverage.orbits(), 1);
        let s = sampling(o, SamplingSpan::OrbitAverage);
        assert!((s.span.value() - period).abs() < 1e-9);
    }

    /// 太陽を回る軌道では、周回数を書かなければ 1 周（地球周回の 3 周ではない）。書けばその周回数、周回平均は 1 周のまま。
    #[test]
    fn heliocentric_default_is_one_orbit() {
        let mut o = orbit();
        o.central = CentralBody::Sun;
        let period = o.period_s();
        let s = sampling(o.clone(), SamplingSpan::Series(None));
        assert!((s.span.value() - period).abs() < 1e-6);
        let s = sampling(o.clone(), SamplingSpan::Series(Some(2)));
        assert!((s.span.value() - 2.0 * period).abs() < 1e-6);
        let s = sampling(o, SamplingSpan::OrbitAverage);
        assert!((s.span.value() - period).abs() < 1e-6);
    }

    /// 1 日のサンプルは 86 400 s。
    #[test]
    fn daily_sampling_spans_one_day() {
        let s = daily_sampling(orbit());
        assert_eq!(s.span.value(), 86_400.0);
    }

    /// 1 日のサンプルの刻みは 10 分まで: 周期が 1 年の日心軌道（周期 / 200 = 1.6 日）でも 600 s、
    /// 静止軌道（430.82 s）や周回軌道（30 s）は周期から決めた刻みのまま。
    #[test]
    fn daily_step_is_capped_for_long_periods() {
        let heliocentric = OrbitSpec::new(
            Epoch::parse("2027-01-01T00:00:00Z").expect("epoch"),
            OrbitElements::with_defaults(
                Qty::from_raw(1.5e11),
                Qty::from_raw(0.0),
                None,
                None,
                None,
                None,
            ),
            Propagator::Kepler,
        );
        let mut sun = heliocentric;
        sun.central = sat_astro::ephem::CentralBody::Sun;
        assert_eq!(daily_sampling(sun).step.value(), MAX_DAILY_STEP_S);
        let leo = OrbitSpec::circular(
            Epoch::parse("2027-01-01T00:00:00Z").expect("epoch"),
            Qty::from_raw(550.0e3),
            Qty::from_raw(1.0),
            Qty::from_raw(0.0),
        );
        assert_eq!(daily_sampling(leo).step.value(), 30.0);
    }

    /// 1 回限りの軌道: 周回数（書かれていても平均の指定でも）を使わず、期間そのものを解く。刻みは `max(30 s, 期間 / 200)`。
    #[test]
    fn one_off_orbit_samples_its_span_with_a_span_based_step() {
        let day = 86_400.0;
        let cruise = Trajectory::from(orbit()).with_span(300.0 * day);
        for span in [
            SamplingSpan::Series(None),
            SamplingSpan::Series(Some(7)),
            SamplingSpan::OrbitAverage,
        ] {
            let s = sampling(cruise.clone(), span);
            assert_eq!(s.span.value(), 300.0 * day);
            assert_eq!(s.step.value(), 300.0 * day / 200.0);
        }
        // 太陽を回る要素の軌道の既定（1 周）は、1 回限りの期間では使わない
        let mut sun = orbit();
        sun.central = CentralBody::Sun;
        let s = sampling(
            Trajectory::from(sun).with_span(100.0 * day),
            SamplingSpan::Series(None),
        );
        assert_eq!(s.span.value(), 100.0 * day);
        // 短い期間は下限の刻み（30 s）
        let s = sampling(
            Trajectory::from(orbit()).with_span(3000.0),
            SamplingSpan::Series(None),
        );
        assert_eq!((s.span.value(), s.step.value()), (3000.0, DEFAULT_STEP_S));
        // 1 日のサンプルは、1 日より短い期間までで、刻みは 10 分まで
        let d = daily_sampling(Trajectory::from(orbit()).with_span(3000.0));
        assert_eq!(d.span.value(), 3000.0);
        let d = daily_sampling(cruise);
        assert_eq!((d.span.value(), d.step.value()), (day, MAX_DAILY_STEP_S));
    }

    /// 設計が書いた刻み: 周期的な軌道はそのまま。1 回限りの軌道は期間の既定（`max(30 s, 期間 / 200)`）より細かくしない。
    #[test]
    fn a_requested_step_is_not_finer_than_the_one_off_default() {
        assert_eq!(requested_step_s(periodic(5400.0), 30.0), 30.0);
        assert_eq!(requested_step_s(periodic(5400.0), 1.0), 1.0);
        let one_off = Cycle::OneOff { span_s: 3.0e7 };
        assert_eq!(requested_step_s(one_off, 30.0), 3.0e7 / 200.0);
        assert_eq!(requested_step_s(one_off, 1.0e6), 1.0e6);
    }
}
