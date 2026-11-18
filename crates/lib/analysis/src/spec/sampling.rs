//! 時系列の標本の取り方の既定（周回数・刻み・1 日）と、設計が書かなかったときにそれを当てる構築。
//! **時系列の期間と刻みの既定はここだけ**。
//! - 期間: 周回数が書かれていなければ [`DEFAULT_ORBITS`]。1 周で数える量（周回平均・割合）は [`SamplingSpan::OrbitAverage`]（1 周）。
//! - 刻み: [`orbit_step_s`]。周期の一定の割合（1 周あたり最大 [`MAX_STEPS_PER_ORBIT`] 標本）を超えない刻み（下限 `sat_astro::timeline::DEFAULT_STEP_S`）。
//!   周期が 100 分までの周回軌道では従来どおり 30 s のまま、静止軌道や月周回のような長い周期でも 1 周の標本数が増えすぎない。
//!
//! 設計から入力を作る側（`fune-projection`）は既定を持たず、書かれた値（`None` かもしれない）か、解析が決める期間の種類
//! （[`SamplingSpan`]）を渡す。

use sat_astro::constants::SECONDS_PER_DAY;
use sat_astro::orbit::OrbitSpec;
use sat_astro::timeline::{DEFAULT_STEP_S, OrbitSampling};
use sat_units::typed::Qty;

/// 周回数が書かれていないときに解く周回数（電源の収支・熱の非定常・RW のアンロードで共通）。
/// 電池の充放電や熱の履歴が初期条件から落ち着いて見える程度の数として、概念検討で使う 3 周とする。
pub const DEFAULT_ORBITS: u32 = 3;

/// 1 周の平均や割合を数える量（センサの禁止角への侵入の割合・アンロード時間の地磁気の平均・熱の軌道平均）が解く周回数。
/// 周期的な量なので 1 周で足りる（複数周は同じ値の繰り返し）。
pub const ORBIT_AVERAGE_ORBITS: u32 = 1;

/// 1 周あたりの標本数の上限（[`orbit_step_s`]）。100 分（6000 s）の周回軌道が下限の刻み 30 s と一致する 200 標本。
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

/// 周期 `period_s` の軌道の時系列の既定の刻み [s]: 下限 `floor_s`、ただし 1 周あたり `max_steps_per_orbit` 標本を超えない
/// （`max(floor_s, period / max_steps_per_orbit)`）。**刻みを周期の割合で決める規則はここだけ**（影の標本 `SHADOW_STEP_S` も同じ規則）。
pub fn step_for_period(period_s: f64, floor_s: f64, max_steps_per_orbit: f64) -> f64 {
    floor_s.max(period_s / max_steps_per_orbit)
}

/// 周期 `period_s` の軌道の時系列の既定の刻み [s]（[`step_for_period`] に 30 s・[`MAX_STEPS_PER_ORBIT`]）。
pub fn orbit_step_s(period_s: f64) -> f64 {
    step_for_period(period_s, DEFAULT_STEP_S, MAX_STEPS_PER_ORBIT)
}

/// 期間の取り方 `span` の標本の取り方（刻みは周期から、β の指定なし）。
pub fn sampling(orbit: OrbitSpec, span: SamplingSpan) -> OrbitSampling {
    let period = orbit.period_s();
    OrbitSampling::new(
        orbit,
        Qty::from_raw(period * f64::from(span.orbits())),
        Qty::from_raw(orbit_step_s(period)),
    )
}

/// 軌道 [`orbit_count`] 周分の標本の取り方（刻みは周期から、β の指定なし）。
pub fn orbits_sampling(orbit: OrbitSpec, orbits: Option<u32>) -> OrbitSampling {
    sampling(orbit, SamplingSpan::Series(orbits))
}

/// 1 日分の標本の取り方（地上局の可視・送信量。刻みは周期から、β の指定なし）。
pub fn daily_sampling(orbit: OrbitSpec) -> OrbitSampling {
    let step = orbit_step_s(orbit.period_s());
    OrbitSampling::new(orbit, Qty::from_raw(SECONDS_PER_DAY), Qty::from_raw(step))
}

#[cfg(test)]
mod tests {
    use super::*;
    use sat_astro::orbit::{OrbitElements, Propagator};
    use sat_astro::time::Epoch;

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

    /// 刻みは周期の割合で決まる: 100 分（6000 s）までは従来の 30 s のまま（1 周 200 標本以下）、
    /// それより長い周期は周期 / 200（静止軌道 86 164 s なら 430.82 s、1 周は 200 標本）。
    #[test]
    fn step_is_a_fraction_of_the_period_with_a_floor() {
        assert_eq!(orbit_step_s(5400.0), 30.0);
        assert_eq!(orbit_step_s(6000.0), 30.0);
        assert!((orbit_step_s(86_164.0) - 86_164.0 / 200.0).abs() < 1e-12);
        // 1 周の標本数は、周期が 6000 s 以下なら周期 / 30 s（従来どおり）、それ以上は 200 で頭打ち
        let steps = |period: f64| period / orbit_step_s(period);
        assert_eq!(steps(5400.0), 180.0);
        assert!((steps(86_164.0) - 200.0).abs() < 1e-9);
        assert!((steps(6.0e5) - 200.0).abs() < 1e-9);
        // 影の標本も同じ規則（下限 5 s・1 周 1200 標本）
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

    /// 1 日の標本は 86 400 s。
    #[test]
    fn daily_sampling_spans_one_day() {
        let s = daily_sampling(orbit());
        assert_eq!(s.span.value(), 86_400.0);
    }
}
