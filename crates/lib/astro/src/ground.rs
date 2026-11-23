//! 地上局と可視判定。**見通し角とパスの検出はここだけ。**

use crate::AstroError;
use crate::earth::{eci_to_ecef, geodetic_to_ecef};
use crate::ephem::CentralBody;
use crate::math::{Vec3, bisect, golden_section_max};
use crate::orbit::OrbitSpec;
use crate::time::Epoch;
use crate::timeline::sample_times;
use sat_units::typed::{Angle, Length, Qty, Time};
use serde::{Deserialize, Serialize};

mod earth_range;
mod ground_track;

pub use earth_range::{EarthRange, earth_range};
pub use ground_track::{GroundTrackPoint, ground_track};

/// 地上局。
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct GroundStation {
    pub id: String,
    pub latitude: Qty<Angle>,
    pub longitude: Qty<Angle>,
    pub altitude: Qty<Length>,
    /// 運用できる最低仰角
    pub min_elevation: Qty<Angle>,
}

impl GroundStation {
    /// 書かれていない値を慣例の値で埋めて作る: 標高 0（楕円体面）、最低仰角 0（地平線）。
    pub fn with_defaults(
        id: String,
        latitude: Qty<Angle>,
        longitude: Qty<Angle>,
        altitude: Option<Qty<Length>>,
        min_elevation: Option<Qty<Angle>>,
    ) -> Self {
        Self {
            id,
            latitude,
            longitude,
            altitude: altitude.unwrap_or(Qty::ZERO),
            min_elevation: min_elevation.unwrap_or(Qty::ZERO),
        }
    }
}

/// 地上局から見た衛星の方向と距離。
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct LookAngles {
    pub azimuth_rad: f64,
    pub elevation_rad: f64,
    pub range_m: f64,
}

/// 1 回のパス。
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct GroundPass {
    pub station: String,
    pub aos: Epoch,
    pub los: Epoch,
    /// 最大仰角の時刻
    pub tca: Epoch,
    pub max_elevation: Qty<Angle>,
    pub duration: Qty<Time>,
}

/// 地上局から見た見通し角。
pub fn look_angles(station: &GroundStation, epoch: Epoch, r_eci_m: Vec3) -> LookAngles {
    Topocentric::new(station).look(eci_to_ecef(r_eci_m, epoch))
}

/// 半径 `body_radius_m` の球の天体の表面の局から、高度 `altitude_m` の衛星を仰角 `elevation_rad` で見たときの
/// 斜距離 [m] = `√((R + h)² − (R cos E)²) − R sin E`。R は中心天体の半径（`CentralBody::radius_m`）。
/// **仰角と高度から斜距離を求めるのはここだけ**（回線計算が使う）。
pub fn slant_range_m(body_radius_m: f64, altitude_m: f64, elevation_rad: f64) -> f64 {
    let r = body_radius_m + altitude_m;
    let (sin_e, cos_e) = elevation_rad.sin_cos();
    (r * r - (body_radius_m * cos_e).powi(2)).sqrt() - body_radius_m * sin_e
}

/// `start` から `span_s` 秒の間のパスを、`step_s` 秒刻みの探索と二分法で求める。
pub fn find_passes(
    station: &GroundStation,
    orbit: &OrbitSpec,
    start: Epoch,
    span_s: f64,
    step_s: f64,
) -> Result<Vec<GroundPass>, AstroError> {
    let track = Track::new(station, orbit, start, span_s, step_s)?;
    Ok(track.passes())
}

/// 地上局の位置と局所水平座標系（ECEF）。
struct Topocentric {
    pos: Vec3,
    east: Vec3,
    north: Vec3,
    up: Vec3,
}

impl Topocentric {
    fn new(station: &GroundStation) -> Self {
        let (lat, lon) = (station.latitude.value(), station.longitude.value());
        let (sl, cl) = lat.sin_cos();
        let (so, co) = lon.sin_cos();
        Self {
            pos: geodetic_to_ecef(lat, lon, station.altitude.value()),
            east: Vec3::new(-so, co, 0.0),
            north: Vec3::new(-sl * co, -sl * so, cl),
            up: Vec3::new(cl * co, cl * so, sl),
        }
    }

    fn look(&self, r_ecef_m: Vec3) -> LookAngles {
        let rho = r_ecef_m - self.pos;
        let range_m = rho.norm();
        let (e, n, u) = (rho.dot(self.east), rho.dot(self.north), rho.dot(self.up));
        LookAngles {
            azimuth_rad: e.atan2(n).rem_euclid(core::f64::consts::TAU),
            elevation_rad: if range_m > 0.0 {
                (u / range_m).clamp(-1.0, 1.0).asin()
            } else {
                core::f64::consts::FRAC_PI_2
            },
            range_m,
        }
    }
}

/// 粗い刻みの軌道の標本。刻みの間は位置と速度の 3 次 Hermite 補間で埋める
/// （LEO・刻み 60 s で誤差 1 mm 未満）。数値伝搬でも積分は 1 回で済む。
struct Track<'a> {
    station: &'a GroundStation,
    topo: Topocentric,
    start: Epoch,
    /// `start` からの秒
    times: Vec<f64>,
    states: Vec<crate::orbit::OrbitState>,
    min_el: f64,
    /// 地球以外の中心天体（衛星が天体の陰に入ると見えない。地球が中心なら地平線が遮るので `None`）
    occulter: Option<CentralBody>,
}

/// 二分法・黄金分割の打ち切り幅 [s]。
const TIME_TOL_S: f64 = 1e-3;
/// 二分法・黄金分割の最大反復回数。
const MAX_ITER: usize = 200;

impl<'a> Track<'a> {
    fn new(
        station: &'a GroundStation,
        orbit: &OrbitSpec,
        start: Epoch,
        span_s: f64,
        step_s: f64,
    ) -> Result<Self, AstroError> {
        let min_el = station.min_elevation.value();
        let lat = station.latitude.value();
        if !min_el.is_finite() || lat.is_nan() || lat.abs() > core::f64::consts::FRAC_PI_2 {
            return Err(AstroError::InvalidInput(format!(
                "Ground station {} has an invalid latitude or minimum elevation",
                station.id
            )));
        }
        let times = sample_times(span_s, step_s)?;
        let offset = start.seconds_since(orbit.epoch);
        let abs: Vec<f64> = times.iter().map(|t| t + offset).collect();
        // 地上局は地球にあるので、地球以外が中心の軌道は地心の位置・速度に直す。
        let central = orbit.central;
        let states = orbit
            .states_at(&abs)?
            .into_iter()
            .zip(times.iter())
            .map(|(s, &t)| {
                let e = start.plus_seconds(t);
                crate::orbit::OrbitState {
                    r_eci: s.r_eci + central.position_eci_m(e),
                    v_eci: s.v_eci + central.velocity_eci_m_s(e),
                }
            })
            .collect();
        Ok(Self {
            station,
            topo: Topocentric::new(station),
            start,
            times,
            states,
            min_el,
            occulter: (central != CentralBody::Earth).then_some(central),
        })
    }

    /// `start` から `t` 秒後の ECI 位置（補間）。
    fn position(&self, t: f64) -> Vec3 {
        let last = self.times.len() - 1;
        let i = match self.times.partition_point(|&x| x <= t) {
            0 => 0,
            k => (k - 1).min(last.saturating_sub(1)),
        };
        if last == 0 {
            return self.states[0].r_eci;
        }
        let (t0, t1) = (self.times[i], self.times[i + 1]);
        let (a, b) = (&self.states[i], &self.states[i + 1]);
        let h = t1 - t0;
        let s = ((t - t0) / h).clamp(0.0, 1.0);
        let (s2, s3) = (s * s, s * s * s);
        a.r_eci * (2.0 * s3 - 3.0 * s2 + 1.0)
            + a.v_eci * (h * (s3 - 2.0 * s2 + s))
            + b.r_eci * (-2.0 * s3 + 3.0 * s2)
            + b.v_eci * (h * (s3 - s2))
    }

    fn elevation(&self, t: f64) -> f64 {
        let epoch = self.start.plus_seconds(t);
        self.topo
            .look(eci_to_ecef(self.position(t), epoch))
            .elevation_rad
    }

    /// 見えるかどうかの水準 [rad]（`min_el` 以上なら見える）。仰角そのもので、
    /// 中心天体が地球以外のときは天体の陰に入る間は `min_el` 未満に下げる（[`Track::occultation_margin`]）。
    fn level(&self, t: f64) -> f64 {
        let el = self.elevation(t);
        match self.occulter {
            None => el,
            Some(body) => el.min(self.min_el + self.occultation_margin(body, t)),
        }
    }

    /// 局から衛星への線分が天体 `body`（球）に当たらない余裕 [rad 相当] = (線分と天体の中心の最短距離 − 半径) / 局から中心までの距離。
    /// 負なら衛星は天体の陰（局から見て天体の向こう側）にいる。
    fn occultation_margin(&self, body: CentralBody, t: f64) -> f64 {
        let epoch = self.start.plus_seconds(t);
        let sat = eci_to_ecef(self.position(t), epoch);
        let centre = eci_to_ecef(body.position_eci_m(epoch), epoch);
        let (to_sat, to_centre) = (sat - self.topo.pos, centre - self.topo.pos);
        let along = (to_centre.dot(to_sat) / to_sat.dot(to_sat)).clamp(0.0, 1.0);
        let closest = to_sat * along;
        ((closest - to_centre).norm() - body.radius_m()) / to_centre.norm()
    }

    /// 水準 `level` が最低仰角を横切る時刻（`lo` と `hi` で符号が違う前提）。
    fn crossing(&self, lo: f64, hi: f64) -> f64 {
        bisect(
            |t| self.level(t) - self.min_el,
            lo,
            hi,
            TIME_TOL_S,
            MAX_ITER,
        )
        .unwrap_or(0.5 * (lo + hi))
    }

    /// 区間内で `f` が最大の時刻と値（黄金分割。パスの中で仰角は単峰）。
    fn peak(&self, f: impl Fn(f64) -> f64, lo: f64, hi: f64) -> (f64, f64) {
        golden_section_max(f, lo, hi, TIME_TOL_S, MAX_ITER)
    }

    fn passes(&self) -> Vec<GroundPass> {
        let el: Vec<f64> = self.times.iter().map(|&t| self.level(t)).collect();
        let n = el.len();
        let mut windows: Vec<(f64, f64)> = Vec::new();
        let mut aos: Option<f64> = if el[0] >= self.min_el {
            Some(self.times[0])
        } else {
            None
        };
        for i in 1..n {
            let (prev, cur) = (el[i - 1] >= self.min_el, el[i] >= self.min_el);
            match (prev, cur) {
                (false, true) => aos = Some(self.crossing(self.times[i - 1], self.times[i])),
                (true, false) => {
                    if let Some(a) = aos.take() {
                        windows.push((a, self.crossing(self.times[i - 1], self.times[i])));
                    }
                }
                (false, false) if i + 1 < n && el[i] > el[i - 1] && el[i] >= el[i + 1] => {
                    // 粗い刻みの間に収まる短いパス（標本はどれも最低仰角未満）を拾う。
                    windows.extend(self.grazing(self.times[i - 1], self.times[i + 1]));
                }
                _ => {}
            }
        }
        if let Some(a) = aos {
            windows.push((a, self.times[n - 1]));
        }
        windows.into_iter().map(|(a, b)| self.pass(a, b)).collect()
    }

    /// 局所最大の前後 `[lo, hi]` で、最大仰角が最低仰角を超えていればその区間を返す。
    fn grazing(&self, lo: f64, hi: f64) -> Option<(f64, f64)> {
        let (t_max, el_max) = self.peak(|t| self.level(t), lo, hi);
        if el_max < self.min_el {
            return None;
        }
        Some((self.crossing(lo, t_max), self.crossing(t_max, hi)))
    }

    fn pass(&self, aos: f64, los: f64) -> GroundPass {
        let (tca, max_el) = self.peak(|t| self.elevation(t), aos, los);
        GroundPass {
            station: self.station.id.clone(),
            aos: self.start.plus_seconds(aos),
            los: self.start.plus_seconds(los),
            tca: self.start.plus_seconds(tca),
            max_elevation: Qty::from_raw(max_el),
            duration: Qty::from_raw(los - aos),
        }
    }
}

#[cfg(test)]
mod tests;
