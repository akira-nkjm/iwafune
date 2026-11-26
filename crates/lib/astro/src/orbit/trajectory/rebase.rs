//! 軌道の始めの時刻の付け替え（解析ケースの `epoch`）。**「軌道のこの時刻から始める」を作るのはここだけ**。
//!
//! 付け替えた軌道は、元の軌道の `start` 以降と同じ軌道（同じ状態の列）を、`start` を元期として表したもの。
//! 要素の軌道は要素を `start` まで進め（Kepler・J2 は永年変化、数値伝播は接触要素）、状態ベクトルの軌道は `start` での状態を元期の状態にし、
//! 回転座標系の軌跡は、元期を付け替えて表の位相を持たせる（同じ軌跡）。1 回限りの期間を持つ軌道は期間を残りの長さにする（期間の外は付け替えられない）。
//! 弧の連鎖は、始めの時刻を含む弧の残りと、それより後の弧になる（境ちょうどは後の弧。`chain.rs`）。付け替えは伝播を足さない
//! （要素の軌道は解析式で進め、状態ベクトルの軌道は既存の数値伝播で `start` まで 1 回進める）。

use super::{OrbitSpec, Trajectory};
use crate::AstroError;
use crate::orbit::{KeplerElements, OrbitElements, Propagator, StateSpec, osculating_elements};
use crate::time::Epoch;
use sat_units::typed::Qty;

impl OrbitSpec {
    /// `start` を元期にした同じ軌道。要素は `start` での値（Kepler・J2 は永年変化を進めた平均要素、数値伝播は接触要素）。
    /// `start` は元期より前でもよい（要素を遡らせる）。
    pub fn rebased_at(&self, start: Epoch) -> Result<OrbitSpec, AstroError> {
        let dt = start.seconds_since(self.epoch);
        if dt == 0.0 {
            return Ok(self.clone());
        }
        let el = match self.propagator {
            Propagator::Numerical { .. } => {
                osculating_elements(&self.state_at(dt)?, self.central.gm())?
            }
            Propagator::Kepler | Propagator::J2 => self.mean_elements_at(dt)?,
        };
        Ok(OrbitSpec {
            epoch: start,
            elements: elements_of(&el),
            ..self.clone()
        })
    }
}

fn elements_of(el: &KeplerElements) -> OrbitElements {
    OrbitElements {
        semi_major_axis: Qty::from_raw(el.a_m),
        eccentricity: el.e,
        inclination: Qty::from_raw(el.i_rad),
        raan: Qty::from_raw(el.raan_rad),
        arg_perigee: Qty::from_raw(el.argp_rad),
        mean_anomaly: Qty::from_raw(el.mean_anomaly_rad),
    }
}

impl Trajectory {
    /// `start` を元期にした同じ軌道（解析ケースの `epoch`。周期的な軌道は `start` が元期の前後どちらでもよい）。
    /// 1 回限りの期間を持つ軌道は、`start` が期間の中（元期以上・終わり未満）でなければ `InvalidInput`
    /// （終わりちょうどは残りの期間が 0 で解析できない）。期間は残りの長さ（終わりまで）になる。
    pub fn starting_at(&self, start: Epoch) -> Result<Self, AstroError> {
        let dt = start.seconds_since(self.epoch());
        if let Some(span_s) = self.span_s() {
            check_inside(self.epoch(), span_s, start, dt)?;
        }
        match self {
            Self::Elements(o) => Ok(Self::Elements(o.rebased_at(start)?)),
            Self::State(s) => {
                let state = if dt == 0.0 {
                    s.state
                } else {
                    s.states_at(&[dt])?.pop().ok_or_else(|| {
                        AstroError::InvalidInput("Numerical propagation returned no result".into())
                    })?
                };
                Ok(Self::State(StateSpec {
                    epoch: start,
                    state,
                    span_s: s.span_s - dt,
                    ..s.clone()
                }))
            }
            Self::Synodic(s) => Ok(Self::Synodic(s.rebased_at(start)?)),
            Self::Spanned { orbit, span_s } => Ok(Self::Spanned {
                orbit: Box::new(orbit.starting_at(start)?),
                span_s: span_s - dt,
            }),
            Self::Arcs(chain) => chain.starting_at(start),
        }
    }
}

/// 1 回限りの期間 `[epoch, epoch + span_s)` の中に `start`（元期から `dt` 秒後）があるか。
fn check_inside(epoch: Epoch, span_s: f64, start: Epoch, dt: f64) -> Result<(), AstroError> {
    let end = epoch.plus_seconds(span_s);
    if !(0.0..=span_s).contains(&dt) {
        return Err(AstroError::InvalidInput(format!(
            "The start time {} is outside the span of the orbit ({} to {})",
            start.to_iso(),
            epoch.to_iso(),
            end.to_iso()
        )));
    }
    if dt >= span_s {
        return Err(AstroError::InvalidInput(format!(
            "The start time {} is the end of the span of the orbit, so no time remains to analyse",
            start.to_iso()
        )));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ephem::CentralBody;
    use crate::force::{ForceConfig, GravityModel};
    use crate::state::OrbitState;

    fn epoch() -> Epoch {
        Epoch::parse("2027-04-01T00:00:00Z").expect("epoch")
    }

    fn spec(propagator: Propagator) -> OrbitSpec {
        let mut o = OrbitSpec::circular_about(
            CentralBody::Earth,
            epoch(),
            Qty::from_raw(550e3),
            Qty::from_raw(1.1),
            Qty::from_raw(0.4),
        );
        o.elements.eccentricity = 0.01;
        o.elements.mean_anomaly = Qty::from_raw(0.7);
        o.propagator = propagator;
        o
    }

    fn close(a: OrbitState, b: OrbitState, tol_m: f64) {
        assert!((a.r_eci - b.r_eci).norm() < tol_m, "{a:?} {b:?}");
    }

    /// 付け替えた軌道の `t` 秒後は、元の軌道の `dt + t` 秒後と同じ（Kepler・J2 は解析式なので桁落ちの範囲、数値伝播は積分の許容の範囲）。
    #[test]
    fn rebased_elements_continue_the_same_orbit() {
        let numerical = Propagator::Numerical {
            forces: ForceConfig::gravity_only(GravityModel::J2),
        };
        for (p, tol) in [
            (Propagator::Kepler, 1e-6),
            (Propagator::J2, 1e-6),
            (numerical, 1.0),
        ] {
            let o = spec(p);
            let start = epoch().plus_seconds(3.0 * 86_400.0 + 1234.0);
            let r = o.rebased_at(start).unwrap();
            assert_eq!(r.epoch, start);
            let dt = start.seconds_since(o.epoch);
            for t in [0.0, 600.0, 5400.0, 40_000.0] {
                close(r.state_at(t).unwrap(), o.state_at(dt + t).unwrap(), tol);
            }
        }
    }

    /// 周期的な軌道は元期より前にも付け替えられる。
    #[test]
    fn periodic_orbits_can_start_before_the_epoch() {
        let o = spec(Propagator::J2);
        let start = epoch().plus_seconds(-86_400.0);
        let r = o.rebased_at(start).unwrap();
        close(
            r.state_at(86_400.0 + 100.0).unwrap(),
            o.state_at(100.0).unwrap(),
            1e-6,
        );
    }

    /// 1 回限りの期間は残りの長さになり、期間の外・終わりは付け替えられない。
    #[test]
    fn one_off_span_becomes_the_remaining_length() {
        let t = Trajectory::Elements(spec(Propagator::Kepler)).with_span(10.0 * 86_400.0);
        let start = epoch().plus_seconds(4.0 * 86_400.0);
        let r = t.starting_at(start).unwrap();
        assert_eq!(r.epoch(), start);
        assert_eq!(r.span_s(), Some(6.0 * 86_400.0));
        assert!(t.starting_at(epoch().plus_seconds(-1.0)).is_err());
        let late = t.starting_at(epoch().plus_seconds(11.0 * 86_400.0));
        assert!(late.unwrap_err().to_string().contains("outside the span"));
        let end = t.starting_at(epoch().plus_seconds(10.0 * 86_400.0));
        assert!(end.unwrap_err().to_string().contains("end of the span"));
    }

    /// 状態ベクトルの軌道は、`start` での状態が元期の状態になり、残りの軌跡は元の軌道と同じ。
    #[test]
    fn state_orbits_restart_from_the_state_at_the_start() {
        let s = StateSpec::with_defaults(
            epoch(),
            CentralBody::Earth,
            OrbitState {
                r_eci: crate::math::Vec3::new(7.0e6, 0.0, 0.0),
                v_eci: crate::math::Vec3::new(0.0, 7.5e3, 1.0e3),
            },
            86_400.0,
            None,
        );
        let t = Trajectory::State(s);
        let r = t.starting_at(epoch().plus_seconds(3600.0)).unwrap();
        assert_eq!(r.span_s(), Some(86_400.0 - 3600.0));
        close(
            r.state_at(1800.0).unwrap(),
            t.state_at(5400.0).unwrap(),
            1.0,
        );
    }
}
