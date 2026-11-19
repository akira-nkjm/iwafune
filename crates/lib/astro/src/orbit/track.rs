//! 任意の時刻の軌道の状態を何度も求める道具。時系列の標本の間（熱の積分の右辺など）で使う。
//!
//! 解析的な伝搬（Kepler・J2）は時刻から直接求める。数値伝搬は 1 回の積分で格子の状態の表を作り、格子の間は
//! 位置と速度から 3 次のエルミート補間で求める（時刻ごとに元期から積分し直さない）。

use super::analytic::Analytic;
use super::{OrbitSpec, OrbitState, Propagator};
use crate::AstroError;
use crate::ode::step_offsets;

/// 任意の時刻の状態を求める道具（[`OrbitSpec::track`]）。
pub struct OrbitTrack {
    kind: Kind,
}

enum Kind {
    Analytic(Analytic),
    /// 数値伝搬の格子（時刻 [s] と状態）
    Table {
        t: Vec<f64>,
        s: Vec<OrbitState>,
    },
}

impl OrbitSpec {
    /// 元期から `span_s` 秒までの任意の時刻の状態を求める道具を作る。数値伝搬は刻み `max_grid_s` 以下の格子で
    /// 積分して表にする（解析的な伝搬は格子を使わない）。格子の点は `step_offsets(span_s, 格子の刻み)` なので、
    /// 刻みが `max_grid_s` の整数倍の標本の時刻は格子の点に重なる。
    pub fn track(&self, span_s: f64, max_grid_s: f64) -> Result<OrbitTrack, AstroError> {
        match &self.propagator {
            Propagator::Kepler | Propagator::J2 => Ok(OrbitTrack {
                kind: Kind::Analytic(Analytic::new(self)?),
            }),
            Propagator::Numerical { .. } => {
                let t = step_offsets(span_s, max_grid_s);
                let s = self.states_at(&t)?;
                Ok(OrbitTrack {
                    kind: Kind::Table { t, s },
                })
            }
        }
    }
}

impl OrbitTrack {
    /// 元期から `t_s` 秒後の状態（表の範囲の外は端の区間の補間式の延長）。
    pub fn state_at(&self, t_s: f64) -> OrbitState {
        match &self.kind {
            Kind::Analytic(a) => a.state_at(t_s),
            Kind::Table { t, s } => hermite(t, s, t_s),
        }
    }
}

/// 格子 `t`・状態 `s` を位置と速度から 3 次のエルミート補間で `t_s` の状態にする（速度はその導関数）。
fn hermite(t: &[f64], s: &[OrbitState], t_s: f64) -> OrbitState {
    let Some(last) = t.len().checked_sub(1).filter(|n| *n > 0) else {
        return s[0];
    };
    let k = t.partition_point(|x| *x <= t_s).clamp(1, last) - 1;
    let h = t[k + 1] - t[k];
    let u = (t_s - t[k]) / h;
    let (a, b) = (&s[k], &s[k + 1]);
    let (u2, u3) = (u * u, u * u * u);
    let (h00, h10, h01, h11) = (
        2.0 * u3 - 3.0 * u2 + 1.0,
        u3 - 2.0 * u2 + u,
        -2.0 * u3 + 3.0 * u2,
        u3 - u2,
    );
    let (d00, d10, d01, d11) = (
        (6.0 * u2 - 6.0 * u) / h,
        3.0 * u2 - 4.0 * u + 1.0,
        (-6.0 * u2 + 6.0 * u) / h,
        3.0 * u2 - 2.0 * u,
    );
    OrbitState {
        r_eci: a.r_eci * h00 + a.v_eci * (h10 * h) + b.r_eci * h01 + b.v_eci * (h11 * h),
        v_eci: a.r_eci * d00 + a.v_eci * d10 + b.r_eci * d01 + b.v_eci * d11,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::orbit::OrbitElements;
    use crate::time::Epoch;
    use sat_units::typed::Qty;

    fn leo(propagator: Propagator) -> OrbitSpec {
        let a =
            crate::orbit::semi_major_axis_from_altitude(crate::ephem::CentralBody::Earth, 550e3);
        OrbitSpec::new(
            Epoch::parse("2027-04-01T00:00:00Z").unwrap(),
            OrbitElements {
                semi_major_axis: Qty::from_raw(a),
                eccentricity: 0.001,
                inclination: Qty::from_raw(97.6_f64.to_radians()),
                raan: Qty::from_raw(0.3),
                arg_perigee: Qty::from_raw(0.0),
                mean_anomaly: Qty::from_raw(0.0),
            },
            propagator,
        )
    }

    /// 解析的な伝搬はそのまま `state_at` と同じ。
    #[test]
    fn analytic_track_equals_state_at() {
        let o = leo(Propagator::J2);
        let tr = o.track(6000.0, 30.0).unwrap();
        assert_eq!(tr.state_at(1234.5), o.state_at(1234.5).unwrap());
    }

    /// 数値伝搬の表の補間は、格子の点で表と一致し、格子の間でも直接の積分と 位置は 1 m・速度は 1 cm/s 以内で一致する。
    #[test]
    fn numerical_table_interpolates_within_a_metre() {
        let o = leo(Propagator::Numerical {
            forces: crate::force::ForceConfig::gravity_only(crate::force::GravityModel::J2),
        });
        let tr = o.track(3000.0, 30.0).unwrap();
        let times = [0.0, 30.0, 45.3, 1000.0, 1517.77, 3000.0];
        let direct = o.states_at(&times).unwrap();
        for (t, d) in times.iter().zip(&direct) {
            let s = tr.state_at(*t);
            let dr = (s.r_eci - d.r_eci).norm();
            let dv = (s.v_eci - d.v_eci).norm();
            assert!(dr < 1.0 && dv < 1e-2, "t={t}: {dr} m, {dv} m/s");
        }
    }
}
