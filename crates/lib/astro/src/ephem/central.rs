//! 軌道の中心天体（地球・月・太陽・惑星）。μ・半径・J2・位置を 1 か所で引く。
//!
//! 軌道の位置・速度は中心天体からの相対ベクトルで、**向きは J2000 平均赤道（ECI）のまま**（原点だけ動かす）。

use super::{Body, planets, position_eci_m};
pub use crate::body::CentralBody;
#[cfg(test)]
use crate::constants::*;
use crate::constants::{A_MOON, GM_SUN, SUN_MEAN_MOTION};
use crate::math::Vec3;
use crate::state::OrbitState;
use crate::time::Epoch;
impl CentralBody {
    /// 親の天体（[`CentralBody::parent`]）を回る軌道の長半径 [m]。月は [`A_MOON`]、地球と惑星は
    /// Standish の表の J2000 の長半径。太陽は `None`。パッチドコニックの遷移は円軌道とみなしてこれを使う。
    pub fn orbit_radius_m(self) -> Option<f64> {
        match self {
            CentralBody::Sun => None,
            CentralBody::Moon => Some(A_MOON),
            other => planets::semi_major_axis_m(other.as_body()),
        }
    }

    /// 中心天体が太陽を回る平均運動 [rad/s]（太陽同期軌道の「太陽」の追従率）。
    /// 地球は [`SUN_MEAN_MOTION`]（恒星年でなく回帰年。従来の値）。惑星は日心軌道の長半径から
    /// `√(GM☉/a³)`（惑星の質量は無視）。月（地球とともに太陽を回る）と太陽は `None`。
    pub fn sun_mean_motion_rad_s(self) -> Option<f64> {
        match self {
            CentralBody::Earth => Some(SUN_MEAN_MOTION),
            CentralBody::Moon | CentralBody::Sun => None,
            other => other
                .orbit_radius_m()
                .map(|a| (GM_SUN / (a * a * a)).sqrt()),
        }
    }

    /// 地心から中心天体への位置（ECI）[m]。地球は 0。
    pub fn position_eci_m(self, epoch: Epoch) -> Vec3 {
        self.as_body()
            .map_or(Vec3::ZERO, |b| position_eci_m(b, epoch))
    }

    /// 地心に対する中心天体の速度（ECI）[m/s]。暦の中心差分（±60 s）で求める。
    pub fn velocity_eci_m_s(self, epoch: Epoch) -> Vec3 {
        if self == CentralBody::Earth {
            return Vec3::ZERO;
        }
        let h = 60.0;
        (self.position_eci_m(epoch.plus_seconds(h)) - self.position_eci_m(epoch.plus_seconds(-h)))
            / (2.0 * h)
    }

    /// 親の天体（[`CentralBody::parent`]）に対する位置・速度（ECI の向き）[m, m/s]。
    /// 惑星・地球は太陽から、月は地球から見る。位置は [`CentralBody::position_eci_m`] の差、
    /// 速度は [`CentralBody::velocity_eci_m_s`]（暦の中心差分）の差。親が無い（太陽）なら `None`。
    pub fn state_about_parent(self, epoch: Epoch) -> Option<OrbitState> {
        let parent = self.parent()?;
        Some(OrbitState {
            r_eci: self.position_eci_m(epoch) - parent.position_eci_m(epoch),
            v_eci: self.velocity_eci_m_s(epoch) - parent.velocity_eci_m_s(epoch),
        })
    }

    /// 影響圏の半径 `a (m/M)^(2/5)` [m]（Laplace の影響圏。`a` は [`CentralBody::orbit_radius_m`]、
    /// `m`・`M` は天体と親の重力定数）。親が無い（太陽）なら `None`。
    pub fn soi_radius_m(self) -> Option<f64> {
        let a = self.orbit_radius_m()?;
        let parent = self.parent()?;
        Some(a * (self.gm() / parent.gm()).powf(0.4))
    }

    /// 同じ親を回る別々の 2 天体の会合周期 `2π / |n₁ − n₂|` [s]（`n` は親の重力定数と
    /// [`CentralBody::orbit_radius_m`] から決める円軌道の平均運動。パッチドコニックの遷移と同じ軌道の半径）。
    /// 親が違う・親が無い・同じ天体なら `None`。日付の探索の出発の窓の長さの既定値に使う。
    pub fn synodic_period_s(self, other: CentralBody) -> Option<f64> {
        let parent = self.parent().filter(|p| other.parent() == Some(*p))?;
        if self == other {
            return None;
        }
        let n = |b: CentralBody| {
            let a = b.orbit_radius_m()?;
            Some((parent.gm() / (a * a * a)).sqrt())
        };
        let dn = (n(self)? - n(other)?).abs();
        (dn > 0.0).then(|| core::f64::consts::TAU / dn)
    }

    /// 中心天体から見た天体 `body` の位置（ECI の向き）[m]。
    pub fn relative_position_m(self, body: Body, epoch: Epoch) -> Vec3 {
        position_eci_m(body, epoch) - self.position_eci_m(epoch)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn earth_uses_existing_constants() {
        let e = CentralBody::Earth;
        assert_eq!(e.gm(), MU_EARTH);
        assert_eq!(e.radius_m(), R_EARTH);
        assert_eq!(e.j2(), J2_EARTH);
        assert_eq!(e.position_eci_m(Epoch { j2000_s: 0.0 }), Vec3::ZERO);
        assert_eq!(CentralBody::default(), CentralBody::Earth);
    }

    /// 会合周期: 地球 − 火星は約 780 日（記憶による参考値。検証していない）、月は親が違うので `None`。
    #[test]
    fn synodic_periods() {
        let days = |a: CentralBody, b: CentralBody| a.synodic_period_s(b).map(|s| s / 86_400.0);
        let em = days(CentralBody::Earth, CentralBody::Mars).expect("sibling");
        assert!((em - 780.0).abs() < 3.0, "{em}");
        assert_eq!(
            days(CentralBody::Earth, CentralBody::Mars),
            days(CentralBody::Mars, CentralBody::Earth)
        );
        // 定義どおりの手計算（円軌道の平均運動の差）
        let n = |b: CentralBody| (GM_SUN / b.orbit_radius_m().unwrap().powi(3)).sqrt();
        let hand = core::f64::consts::TAU / (n(CentralBody::Earth) - n(CentralBody::Mars)).abs();
        let got = CentralBody::Earth
            .synodic_period_s(CentralBody::Mars)
            .unwrap();
        assert!((got - hand).abs() < 1e-6 * hand);
        assert_eq!(
            CentralBody::Earth.synodic_period_s(CentralBody::Earth),
            None
        );
        assert_eq!(CentralBody::Earth.synodic_period_s(CentralBody::Moon), None);
        assert_eq!(CentralBody::Sun.synodic_period_s(CentralBody::Earth), None);
    }

    #[test]
    fn orbit_radii_and_parents() {
        let au = |b: CentralBody| b.orbit_radius_m().expect("radius") / AU;
        assert!((au(CentralBody::Earth) - 1.0).abs() < 1e-5);
        assert!((au(CentralBody::Mars) - 1.5237).abs() < 1e-4);
        assert_eq!(CentralBody::Moon.orbit_radius_m(), Some(A_MOON));
        assert_eq!(CentralBody::Sun.orbit_radius_m(), None);
        assert_eq!(CentralBody::Moon.parent(), Some(CentralBody::Earth));
        assert_eq!(CentralBody::Mars.parent(), Some(CentralBody::Sun));
        assert_eq!(CentralBody::Moon.nominal_sun_distance_m(), Some(AU));
        assert_eq!(CentralBody::Mars.nominal_sun_distance_m(), None);
    }

    /// 大気・磁場のモデルは地球だけ。月は「無い」（0 が正しい）、火星・木星などは「有るがモデルが無い」。
    #[test]
    fn environment_models_distinguish_absent_from_missing() {
        use crate::body::EnvironmentModel::{Absent, Modelled, NotModelled};
        assert_eq!(CentralBody::Earth.atmosphere_model(), Modelled);
        assert_eq!(CentralBody::Earth.magnetic_field_model(), Modelled);
        assert_eq!(CentralBody::Moon.atmosphere_model(), Absent);
        assert_eq!(CentralBody::Moon.magnetic_field_model(), Absent);
        assert_eq!(CentralBody::Mars.atmosphere_model(), NotModelled);
        assert_eq!(CentralBody::Mars.magnetic_field_model(), Absent);
        assert_eq!(CentralBody::Venus.atmosphere_model(), NotModelled);
        assert_eq!(CentralBody::Jupiter.magnetic_field_model(), NotModelled);
        assert_eq!(CentralBody::Sun.atmosphere_model(), NotModelled);
    }

    #[test]
    fn sun_direction_is_undefined_for_the_sun() {
        let e = Epoch { j2000_s: 8.6e8 };
        assert!(crate::ephem::sun_direction_from(CentralBody::Sun, e).is_none());
        let s = crate::ephem::sun_direction_from(CentralBody::Moon, e).expect("direction");
        assert!((s.norm() - 1.0).abs() < 1e-12);
    }

    /// 影響圏 `a (m/M)^(2/5)` の手計算: 地球 `a = 1.0000 AU`、`m/M = 3.986e14/1.327e20 = 3.003e-6`、
    /// `(m/M)^0.4 = exp(0.4 · ln 3.003e-6) = exp(−5.0865) = 6.18e-3` → `9.25e8 m`（Vallado §12.3 の 9.25e5 km）。
    /// 月は `A_MOON · (GM☾/μ⊕)^0.4 = 3.844e8 · 0.01230^0.4 ≈ 6.6e7 m`。
    #[test]
    fn soi_radii_follow_the_hand_calculation() {
        let earth = CentralBody::Earth.soi_radius_m().expect("soi");
        assert!((earth / 1e3 - 9.25e5).abs() < 0.02e5, "{earth}");
        let direct =
            CentralBody::Earth.orbit_radius_m().expect("a") * (MU_EARTH / GM_SUN).powf(0.4);
        assert_eq!(earth, direct);
        let moon = CentralBody::Moon.soi_radius_m().expect("soi");
        assert!((moon / 1e3 - 6.6e4).abs() < 0.2e4, "{moon}");
        assert!(CentralBody::Sun.soi_radius_m().is_none());
    }

    /// 地球の日心状態: 距離は約 1 AU、速さは約 29.8 km/s、速度は位置を短い区間で積分したものと合う。
    /// 月の地心状態は `position_eci_m`・`velocity_eci_m_s` そのもの。太陽は `None`。
    #[test]
    fn state_about_parent_is_consistent() {
        let e = Epoch::parse("2027-04-01T00:00:00Z").expect("epoch");
        let s = CentralBody::Earth.state_about_parent(e).expect("state");
        assert!((s.r_eci.norm() / AU - 1.0).abs() < 0.02);
        let v = s.v_eci.norm();
        assert!(v > 29.2e3 && v < 30.4e3, "{v}");
        let later = CentralBody::Earth
            .state_about_parent(e.plus_seconds(600.0))
            .expect("state");
        // 区間の平均の速度は両端の速度の平均（台形則。曲率の誤差は 1e-4 m/s 以下）
        let mean_v = (s.v_eci + later.v_eci) * 0.5;
        assert!(((later.r_eci - s.r_eci) / 600.0 - mean_v).norm() < 0.01);
        // 速度は軌道面内（r · v は小さい）で、日心の速さは vis-viva と合う
        let a = CentralBody::Earth.orbit_radius_m().expect("a");
        let vv = crate::transfer::vis_viva_m_s(GM_SUN, s.r_eci.norm(), a);
        assert!((v - vv).abs() < 150.0, "{v} {vv}");
        let moon = CentralBody::Moon.state_about_parent(e).expect("state");
        assert_eq!(moon.r_eci, CentralBody::Moon.position_eci_m(e));
        assert_eq!(moon.v_eci, CentralBody::Moon.velocity_eci_m_s(e));
        assert!(CentralBody::Sun.state_about_parent(e).is_none());
    }

    #[test]
    fn moon_velocity_is_about_one_km_per_second() {
        let v = CentralBody::Moon
            .velocity_eci_m_s(Epoch { j2000_s: 8.6e8 })
            .norm();
        assert!(v > 960.0 && v < 1090.0, "{v}");
        let ratio = MU_EARTH / CentralBody::Moon.gm();
        assert!((ratio - 81.3).abs() < 0.1);
    }
}
