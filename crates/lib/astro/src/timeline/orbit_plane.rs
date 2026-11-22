//! 軌道面の表示用の座標（時系列の位置を、最初の標本で決めた固定の軌道面の枠に射影する）。
//! 軌道の図（インセット）を描くための値で、**ここだけ**で作る（web は回転も射影もしない）。

use super::TimelineSample;
use crate::math::Vec3;
use sat_units::typed::{Angle, Length, Qty};
use serde::{Deserialize, Serialize};

/// 太陽が軌道面の法線にほぼ沿っているとみなす限界（軌道面への射影の長さ = cos(仰角) がこれ未満）。
const SUN_NEAR_NORMAL: f64 = 1e-6;

/// 軌道面の表示枠での軌跡と、図に添える値。時刻は元の時系列の標本と同じ（添字が対応する）。
///
/// **表示枠**は最初の標本で 1 回だけ決める。z = 軌道の法線（r × v）、x = 太陽方向を軌道面に射影した向き
/// （太陽が法線にほぼ沿っていれば最初の位置の向き）、y = z × x。図では太陽が常に +x 側、影が −x 側になる。
/// 枠は走行の間固定（軌道面の歳差や太陽の動きは枠に入れない）。
///
/// **図は模式**。影の幅は円柱影（中心天体の半径）で、実際の日陰は標本ごとの [`OrbitPlaneTrack::eclipse`] が正。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct OrbitPlaneTrack {
    /// 位置の x 成分（表示枠。+x が太陽側）[m]。長い配列なので次元の記録を避けて素の SI 値で持つ（`series` の列と同じ扱い）
    pub x: Vec<f64>,
    /// 位置の y 成分（表示枠）[m]
    pub y: Vec<f64>,
    /// 標本ごとの日陰か（日照率が、呼び出し側の渡すしきい値を下回る）。図と 3D の表示はこれをそのまま使う
    pub eclipse: Vec<bool>,
    /// 標本ごとの太陽方向（機体座標系の単位ベクトル）
    pub sun_dir_body: Vec<[f64; 3]>,
    /// 標本ごとの地心（中心天体）方向（機体座標系の単位ベクトル）
    pub nadir_body: Vec<[f64; 3]>,
    /// 中心天体の半径 [m]
    pub body_radius: Qty<Length>,
    /// 最初の標本での、太陽の軌道面からの仰角（表示枠の x–y 面から。β 角に相当）
    pub sun_elevation: Qty<Angle>,
    /// 図に描く影の帯の半幅 [m]（円柱影なので中心天体の半径に等しい）
    pub shadow_half_width: Qty<Length>,
}

impl Default for OrbitPlaneTrack {
    /// 標本なし（半径 0）。
    fn default() -> Self {
        Self::empty(0.0)
    }
}

impl OrbitPlaneTrack {
    fn empty(body_radius_m: f64) -> Self {
        Self {
            x: Vec::new(),
            y: Vec::new(),
            eclipse: Vec::new(),
            sun_dir_body: Vec::new(),
            nadir_body: Vec::new(),
            body_radius: Qty::from_raw(body_radius_m),
            sun_elevation: Qty::from_raw(0.0),
            shadow_half_width: Qty::from_raw(body_radius_m),
        }
    }
}

/// 時系列を軌道面の表示枠に射影する（[`OrbitPlaneTrack`]）。`body_radius_m` は中心天体の半径 [m]。
/// `sunlit_threshold` は日陰とみなす日照率の上限（これ未満が日陰。値は解析が決める）。
/// 標本が空なら空の軌跡（半径と影の幅だけ入る）。
pub fn orbit_plane_track(
    samples: &[TimelineSample],
    body_radius_m: f64,
    sunlit_threshold: f64,
) -> OrbitPlaneTrack {
    let mut track = OrbitPlaneTrack::empty(body_radius_m);
    let Some(first) = samples.first() else {
        return track;
    };
    let frame = Frame::from_first(first);
    track.sun_elevation = Qty::from_raw(frame.sun_elevation_rad);
    for s in samples {
        track.x.push(s.r_eci.dot(frame.x));
        track.y.push(s.r_eci.dot(frame.y));
        track.eclipse.push(s.sunlit < sunlit_threshold);
        track.sun_dir_body.push(s.sun_dir_body().to_array());
        track.nadir_body.push(s.nadir_body().to_array());
    }
    track
}

/// 表示枠の軸（ECI の単位ベクトル）と、最初の標本での太陽の仰角。
struct Frame {
    x: Vec3,
    y: Vec3,
    sun_elevation_rad: f64,
}

impl Frame {
    fn from_first(first: &TimelineSample) -> Self {
        let r = first.r_eci;
        let z = r
            .cross(first.v_eci)
            .normalized()
            .unwrap_or_else(|| r.any_orthogonal().normalized().unwrap_or(Vec3::Z));
        let sun = first.sun_dir_eci;
        let in_plane = sun - z.scale(sun.dot(z));
        let x = if in_plane.norm() >= SUN_NEAR_NORMAL {
            in_plane.normalized()
        } else {
            r.normalized()
        }
        .unwrap_or(Vec3::X);
        Self {
            x,
            y: z.cross(x),
            sun_elevation_rad: sun.dot(z).clamp(-1.0, 1.0).asin(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::math::Quat;
    use crate::time::Epoch;

    const A: f64 = 7.0e6;

    fn sample(t_s: f64, r: Vec3, v: Vec3, sun: Vec3, sunlit: f64) -> TimelineSample {
        TimelineSample {
            t_s,
            epoch: Epoch::parse("2027-06-01T00:00:00Z").expect("epoch"),
            r_eci: r,
            v_eci: v,
            sun_dir_eci: sun,
            sun_distance_m: 1.5e11,
            sunlit,
            q_body_to_eci: Quat::IDENTITY,
        }
    }

    /// 赤道の円軌道（半径 a、反時計回り）を 8 点、太陽は +X。
    fn equatorial(sun: Vec3) -> Vec<TimelineSample> {
        (0..8)
            .map(|k| {
                let th = core::f64::consts::TAU * f64::from(k) / 8.0;
                let r = Vec3::new(A * th.cos(), A * th.sin(), 0.0);
                let v = Vec3::new(-th.sin(), th.cos(), 0.0).scale(7.5e3);
                sample(f64::from(k), r, v, sun, if k < 6 { 1.0 } else { 0.0 })
            })
            .collect()
    }

    /// 太陽が +X の赤道円軌道: 表示枠は ECI と一致し、点は半径 a の円の上で最初は (a, 0)。
    #[test]
    fn equatorial_circle_with_sun_along_x() {
        let track = orbit_plane_track(&equatorial(Vec3::X), 6.378e6, 0.5);
        assert_eq!(track.x.len(), 8);
        for (x, y) in track.x.iter().zip(&track.y) {
            assert!((x.hypot(*y) - A).abs() < 1e-6);
        }
        assert!((track.x[0] - A).abs() < 1e-6 && track.y[0].abs() < 1e-6);
        // 反時計回りなので 2 点目は +y 側
        assert!(track.y[1] > 0.0);
        assert!(track.sun_elevation.value().abs() < 1e-12);
        assert_eq!(
            track.eclipse,
            vec![false, false, false, false, false, false, true, true]
        );
        assert_eq!(track.body_radius.value(), 6.378e6);
        assert_eq!(track.shadow_half_width.value(), 6.378e6);
    }

    /// 太陽が +Y 側なら、枠は軌道面内で回って太陽が +x に来る（最初の点は (0, −a)）。
    /// 太陽が面から 30° 上なら仰角が 30°。
    #[test]
    fn frame_follows_sun_projection_and_elevation() {
        let e = 30f64.to_radians();
        let sun = Vec3::new(0.0, e.cos(), e.sin());
        let track = orbit_plane_track(&equatorial(sun), 6.378e6, 0.5);
        assert!((track.sun_elevation.value() - e).abs() < 1e-12);
        assert!(track.x[0].abs() < 1e-6);
        assert!((track.y[0] + A).abs() < 1e-6);
        // 4 分の 1 周進んだ点（0, A）は太陽側（+x）に来る
        assert!((track.x[2] - A).abs() < 1e-6);
    }

    /// 太陽が軌道の法線に沿うときは、最初の位置の向きが +x。
    #[test]
    fn sun_along_normal_falls_back_to_first_position() {
        let track = orbit_plane_track(&equatorial(Vec3::Z), 6.378e6, 0.5);
        assert!((track.x[0] - A).abs() < 1e-6 && track.y[0].abs() < 1e-6);
        assert!((track.sun_elevation.value() - core::f64::consts::FRAC_PI_2).abs() < 1e-6);
    }

    /// 機体系の太陽・地心方向は標本の値そのまま（姿勢が単位なら ECI と一致）。空なら空。
    #[test]
    fn body_directions_and_empty() {
        let s = equatorial(Vec3::X);
        let track = orbit_plane_track(&s, 6.378e6, 0.5);
        assert_eq!(track.sun_dir_body[0], [1.0, 0.0, 0.0]);
        assert!((track.nadir_body[0][0] + 1.0).abs() < 1e-12);
        let empty = orbit_plane_track(&[], 6.378e6, 0.5);
        assert!(empty.x.is_empty() && empty.body_radius.value() == 6.378e6);
    }
}
