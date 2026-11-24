//! 2 天体の回転座標系での軌跡: ラグランジュ点そのもの、またはそのまわりのハロー軌道（[`super::Trajectory::Synodic`]）。
//!
//! **円制限三体問題（CR3BP）の軌道を、暦の実際の 2 天体の位置に載せたもの**であって、力モデルの伝搬ではない
//! （太陽・月・惑星の摂動、太陽輻射圧、2 天体の離心率による力の変化は入らない。軌道維持は ΔV の余裕として別に書く）。
//! 数学（質量比・共線点・補正・回転系と慣性系の変換）は [`super::cr3bp`] にあり、ここは暦との結びつけだけを持つ。
//!
//! # 載せ方（脈動座標系）
//! 各時刻に、副天体の主天体まわりの状態（[`CentralBody::state_about_parent`]。暦の位置と速度）から、瞬間の距離を長さの単位、
//! 瞬間の角速度を回転の角速度とする座標系を決め（[`super::cr3bp::Cr3bp::rotating_to_inertial`]）、CR3BP の回転系の状態を慣性系にする。
//! 2 天体の軌道が円なら通常の CR3BP の正規化に一致し、楕円（地球の離心率 0.017）なら距離の変化に合わせて軌跡が伸び縮みする。
//!
//! - **質量比**: 主天体の GM と副天体の GM。副天体が地球のときは月を足した GM（太陽–地球系で普通の取り方。暦の地球の位置も
//!   地球と月の重心に近い）、それ以外はその天体だけ（月は地球が主天体）。主天体は副天体の親（[`CentralBody::parent`]）でなければならない。
//! - **無次元の時間（位相）**: 副天体が主天体のまわりで掃いた角（元期から。2π 周で 1 周）。CR3BP の無次元時間 1 が 1 rad にあたる
//!   （円軌道では n·t と同じ）。掃いた角の巻き数は平均運動 `n = √((GM₁+GM₂)/a³)`（`a` は副天体の軌道の長半径。
//!   [`CentralBody::orbit_radius_m`]）で数える。
//! - **ハロー軌道**: 周期軌道を 1 つ補正して 1 周期を表にし（[`super::OrbitTrack`] と同じ 3 次のエルミート補間）、位相の周期の余りで引く。
//!   元期は表の始め（xz 平面を横切る点）。z 振幅は `a` で割って正規化する（元期の距離が `a` と違えば、実際の振幅は距離の比だけ変わる）。
//!   北・南は z → −z の鏡像。
//! - **原点の天体（[`SynodicSpec::central`]）**: 副天体。L1・L2 のまわりの軌道の直下点・日陰・遮りの基準は副天体
//!   （太陽–地球 L2 なら地球。地球の本影・半影が使える）。状態は副天体の中心からの相対。
//! - **繰り返し（[`SynodicSpec::period_s`]）**: ハローは補正した周期（位相の周期 / 平均運動）。固定点は 2 天体の公転周期
//!   （恒星周期 `2π/n`。太陽との幾何が 1 周で繰り返す）。

use super::cr3bp::{
    Branch, CorrectorConfig, Cr3bp, Cr3bpError, LibrationPoint, SampledOrbit, Secondary, State6,
};
use super::{OrbitState, OrbitTrack};
use crate::AstroError;
use crate::ephem::CentralBody;
use crate::math::Vec3;
use crate::time::Epoch;
use core::f64::consts::TAU;
use serde::{Deserialize, Serialize};
use std::sync::{Arc, OnceLock};

/// ハロー軌道の表の区間の数（1 周期）。3 次のエルミート補間の誤差は `(2π/400)⁴/384` 程度。
const HALO_TABLE_INTERVALS: usize = 400;

/// ハロー軌道の指定（z 振幅と枝）。
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct HaloSpec {
    /// z 方向の振幅 [m]（CR3BP の解の z の最大値。2 天体の平均距離 `a` で正規化して補正する）
    pub amplitude_z_m: f64,
    /// 北（元期に z > 0）か南か
    pub branch: Branch,
}

/// 2 天体の回転座標系での軌跡（[`super::Trajectory::Synodic`]）。
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct SynodicSpec {
    /// 元期（時刻 0。ハローは表の始めの位相、固定点は意味を持たない）
    pub epoch: Epoch,
    /// 主天体（副天体の親）
    pub primary: CentralBody,
    /// 副天体（位置・速度の原点）
    pub secondary: CentralBody,
    /// ラグランジュ点
    pub point: LibrationPoint,
    /// ハロー軌道（無ければ点そのもの）。L1・L2 だけ
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub halo: Option<HaloSpec>,
    /// 元期での表の位相 [rad]（ハローだけ。`starting_at` で元期を付け替えたときに残る。書かなければ 0）
    #[serde(default, skip_serializing_if = "is_zero")]
    pub phase_rad: f64,
    /// 補正した 1 周期の表（初めて要るときに 1 回だけ作る）
    #[serde(skip)]
    #[cfg_attr(feature = "ts", ts(skip))]
    table: OnceLock<Result<Arc<SampledOrbit>, String>>,
}

fn is_zero(x: &f64) -> bool {
    *x == 0.0
}

/// 軌跡の形（回転系の正規化した状態）。
enum Shape {
    Fixed(State6),
    Periodic { track: OrbitTrack, period: f64 },
}

/// 状態の評価に要る道具（質量比・形・位相の基準）。
struct Prepared {
    cr: Cr3bp,
    shape: Shape,
    /// 平均運動 [rad/s]
    n_mean: f64,
    /// 位相の基準の向き（元期の副天体の方向と、その軌道面内の直交方向）
    ex0: Vec3,
    ey0: Vec3,
}

impl SynodicSpec {
    /// 点そのもの（`halo = None`）またはハロー軌道の仕様を作る。値の検査は [`SynodicSpec::validate`]。
    pub fn new(
        epoch: Epoch,
        primary: CentralBody,
        secondary: CentralBody,
        point: LibrationPoint,
        halo: Option<HaloSpec>,
    ) -> Self {
        Self {
            epoch,
            primary,
            secondary,
            point,
            halo,
            phase_rad: 0.0,
            table: OnceLock::new(),
        }
    }

    /// 原点の天体（位置・速度の基準、直下点・日陰・遮りの基準）。副天体。
    pub fn central(&self) -> CentralBody {
        self.secondary
    }

    /// 副天体のほかに日陰を作る天体。主天体が太陽でなければ主天体（地球–月系の L1・L2 では地球が月の向こうの点に影を落とす）。
    pub fn extra_shadow_caster(&self) -> Option<CentralBody> {
        (self.primary != CentralBody::Sun).then_some(self.primary)
    }

    /// 質量比に使う副天体の質量の取り方。
    fn secondary_mass(&self) -> Secondary {
        if self.secondary == CentralBody::Earth {
            Secondary::EarthMoonSystem
        } else {
            Secondary::Body(self.secondary)
        }
    }

    /// 2 天体の CR3BP（質量比）。
    pub fn cr3bp(&self) -> Result<Cr3bp, AstroError> {
        Cr3bp::from_bodies(self.primary, self.secondary_mass()).map_err(invalid)
    }

    /// 2 天体の平均距離 `a` [m]（副天体の軌道の長半径）。正規化の長さの単位。
    fn distance_unit_m(&self) -> Result<f64, AstroError> {
        self.secondary.orbit_radius_m().ok_or_else(|| {
            AstroError::InvalidInput(format!(
                "The mean distance of {:?} about {:?} is unknown",
                self.secondary, self.primary
            ))
        })
    }

    /// 平均運動 `n = √((GM₁+GM₂)/a³)` [rad/s]。
    pub fn mean_motion_rad_s(&self) -> Result<f64, AstroError> {
        let a = self.distance_unit_m()?;
        Ok(((self.primary.gm() + self.secondary_mass().gm()) / (a * a * a)).sqrt())
    }

    /// 値を検査する（主天体が副天体の親であること、振幅は正の有限、ハローは L1・L2 だけ。ハローは補正できることも確かめる）。
    pub fn validate(&self) -> Result<(), AstroError> {
        if self.secondary.parent() != Some(self.primary) {
            return Err(AstroError::InvalidInput(format!(
                "{:?} is not a primary of {:?} in the ephemeris (a libration orbit needs a body and the body it orbits)",
                self.primary, self.secondary
            )));
        }
        self.mean_motion_rad_s()?;
        if let Some(h) = &self.halo {
            if !(h.amplitude_z_m.is_finite() && h.amplitude_z_m > 0.0) {
                return Err(AstroError::InvalidInput(format!(
                    "The z-amplitude of a halo orbit must be positive ({} m)",
                    h.amplitude_z_m
                )));
            }
            if !matches!(self.point, LibrationPoint::L1 | LibrationPoint::L2) {
                return Err(AstroError::InvalidInput(format!(
                    "A halo orbit exists only about L1 and L2, not {:?}",
                    self.point
                )));
            }
            self.halo_table()?;
        }
        Ok(())
    }

    /// 補正した 1 周期の表（初めて要るときに 1 回だけ作る）。
    fn halo_table(&self) -> Result<Arc<SampledOrbit>, AstroError> {
        let built = self
            .table
            .get_or_init(|| self.build_halo().map(Arc::new).map_err(|e| e.to_string()));
        built
            .clone()
            .map_err(|e| AstroError::InvalidInput(format!("Cannot build the halo orbit ({e})")))
    }

    fn build_halo(&self) -> Result<SampledOrbit, AstroError> {
        let halo = self
            .halo
            .ok_or_else(|| AstroError::InvalidInput("The orbit is not a halo orbit".into()))?;
        let cr = self.cr3bp()?;
        let z = halo.amplitude_z_m / self.distance_unit_m()?;
        let cfg = CorrectorConfig::default();
        let corrected = cr
            .halo_orbit(self.point, halo.branch, z, &cfg)
            .map_err(invalid)?;
        cr.sample_orbit(&corrected.orbit, HALO_TABLE_INTERVALS, &cfg.integrator)
            .map_err(invalid)
    }

    /// 繰り返しの長さ [s]。ハローは補正した周期、固定点（と、表が作れなかったとき）は 2 天体の公転周期 `2π/n`。
    pub fn period_s(&self) -> f64 {
        let sidereal = self.mean_motion_rad_s().map_or(f64::NAN, |n| TAU / n);
        match (&self.halo, self.mean_motion_rad_s()) {
            (Some(_), Ok(n)) => self.halo_table().map_or(sidereal, |t| t.period / n),
            _ => sidereal,
        }
    }

    fn prepare(&self) -> Result<Prepared, AstroError> {
        self.validate()?;
        let cr = self.cr3bp()?;
        let shape = match &self.halo {
            Some(_) => {
                let t = self.halo_table()?;
                let (times, states) = t.as_table();
                Shape::Periodic {
                    track: OrbitTrack::from_table(times, states),
                    period: t.period,
                }
            }
            None => {
                let p = cr.libration_point(self.point).map_err(invalid)?;
                Shape::Fixed([p[0], p[1], p[2], 0.0, 0.0, 0.0])
            }
        };
        let s0 = self.secondary_state(self.epoch)?;
        let ex0 = s0.r_eci.normalized().ok_or_else(zero_separation)?;
        let ez0 = s0
            .r_eci
            .cross(s0.v_eci)
            .normalized()
            .ok_or_else(zero_separation)?;
        Ok(Prepared {
            cr,
            shape,
            n_mean: self.mean_motion_rad_s()?,
            ex0,
            ey0: ez0.cross(ex0),
        })
    }

    /// 副天体の主天体まわりの状態（暦から）。
    fn secondary_state(&self, epoch: Epoch) -> Result<OrbitState, AstroError> {
        self.secondary.state_about_parent(epoch).ok_or_else(|| {
            AstroError::InvalidInput(format!(
                "{:?} has no parent body in the ephemeris",
                self.secondary
            ))
        })
    }

    /// 元期から `t_s` 秒後までに副天体が主天体のまわりで掃いた角 [rad]（巻き数は平均運動で数える）。
    fn swept_angle(&self, p: &Prepared, t_s: f64, sec: &OrbitState) -> f64 {
        let wrapped = sec.r_eci.dot(p.ey0).atan2(sec.r_eci.dot(p.ex0));
        wrapped + TAU * ((p.n_mean * t_s - wrapped) / TAU).round()
    }

    /// 元期からの秒 `t_s` での状態（原点は副天体、向きは ECI）。
    fn state_with(&self, p: &Prepared, t_s: f64) -> Result<OrbitState, AstroError> {
        let sec = self.secondary_state(self.epoch.plus_seconds(t_s))?;
        let x = match &p.shape {
            Shape::Fixed(x) => *x,
            Shape::Periodic { track, period } => {
                let tau = (self.phase_rad + self.swept_angle(p, t_s, &sec)).rem_euclid(*period);
                let s = track.state_at(tau);
                [
                    s.r_eci.x, s.r_eci.y, s.r_eci.z, s.v_eci.x, s.v_eci.y, s.v_eci.z,
                ]
            }
        };
        let inertial =
            p.cr.rotating_to_inertial(&sec, &x)
                .ok_or_else(zero_separation)?;
        Ok(OrbitState {
            r_eci: inertial.r_eci - sec.r_eci,
            v_eci: inertial.v_eci - sec.v_eci,
        })
    }

    /// 元期からの秒 `times_s` での状態（副天体の中心から。向きは ECI）。
    pub fn states_at(&self, times_s: &[f64]) -> Result<Vec<OrbitState>, AstroError> {
        let p = self.prepare()?;
        times_s.iter().map(|&t| self.state_with(&p, t)).collect()
    }

    /// `start` を元期にした同じ軌道（ハローは `start` での表の位相を `phase_rad` に持つ）。
    pub fn rebased_at(&self, start: Epoch) -> Result<Self, AstroError> {
        let dt = start.seconds_since(self.epoch);
        let mut out = Self::new(start, self.primary, self.secondary, self.point, self.halo);
        if let Some(t) = self.halo.as_ref().map(|_| self.halo_table()).transpose()? {
            let p = self.prepare()?;
            let sec = self.secondary_state(start)?;
            out.phase_rad = (self.phase_rad + self.swept_angle(&p, dt, &sec)).rem_euclid(t.period);
            // 同じ表を使い回す（作り直さない）
            let _ = out.table.set(Ok(t));
        }
        Ok(out)
    }
}

fn invalid(e: Cr3bpError) -> AstroError {
    AstroError::InvalidInput(format!("Cannot use the libration orbit ({e})"))
}

fn zero_separation() -> AstroError {
    AstroError::InvalidInput(
        "The two bodies of the libration orbit coincide or move in a straight line, so the rotating frame is undefined".into(),
    )
}

#[cfg(test)]
mod tests;
