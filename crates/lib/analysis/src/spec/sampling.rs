//! 時系列の標本の取り方の既定（周回数・刻み・1 日）と、設計が書かなかったときにそれを当てる構築。
//! **時系列の期間と刻みの既定はここだけ**（周回数が書かれていなければ [`DEFAULT_ORBITS`]、刻みは
//! `sat_astro::timeline::DEFAULT_STEP_S`）。設計から入力を作る側（`fune-projection`）は既定を持たず、書かれた値（`None` かもしれない）を渡す。

use sat_astro::constants::SECONDS_PER_DAY;
use sat_astro::orbit::OrbitSpec;
use sat_astro::timeline::{DEFAULT_STEP_S, OrbitSampling};
use sat_units::typed::Qty;

/// 周回数が書かれていないときに解く周回数（電源の収支・熱の非定常・RW のアンロードで共通）。
/// 電池の充放電や熱の履歴が初期条件から落ち着いて見える程度の数として、概念検討で使う 3 周とする。
pub const DEFAULT_ORBITS: u32 = 3;

/// 解く周回数。書かれていなければ [`DEFAULT_ORBITS`]、0 は 1 周に直す。
pub fn orbit_count(orbits: Option<u32>) -> u32 {
    orbits.unwrap_or(DEFAULT_ORBITS).max(1)
}

/// 軌道 [`orbit_count`] 周分の標本の取り方（刻みは既定、β の指定なし）。
pub fn orbits_sampling(orbit: OrbitSpec, orbits: Option<u32>) -> OrbitSampling {
    let span = orbit.period_s() * f64::from(orbit_count(orbits));
    OrbitSampling::new(orbit, Qty::from_raw(span), Qty::from_raw(DEFAULT_STEP_S))
}

/// 1 日分の標本の取り方（地上局の可視・送信量。刻みは既定、β の指定なし）。
pub fn daily_sampling(orbit: OrbitSpec) -> OrbitSampling {
    OrbitSampling::new(
        orbit,
        Qty::from_raw(SECONDS_PER_DAY),
        Qty::from_raw(DEFAULT_STEP_S),
    )
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
            Epoch::parse("2027-01-01T00:00:00Z").expect("元期"),
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
        assert_eq!(s.step.value(), DEFAULT_STEP_S);
        let s = orbits_sampling(o, Some(5));
        assert!((s.span.value() - 5.0 * period).abs() < 1e-9);
    }

    /// 1 日の標本は 86 400 s。
    #[test]
    fn daily_sampling_spans_one_day() {
        let s = daily_sampling(orbit());
        assert_eq!(s.span.value(), 86_400.0);
    }
}
