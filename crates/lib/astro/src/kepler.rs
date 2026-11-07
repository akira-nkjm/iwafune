//! Kepler 方程式と軌道面の回転（暦・軌道で共有）。
use core::f64::consts::TAU;
/// Kepler 方程式 `M = E − e sin E` を解いて離心近点角 E [rad] を返す（楕円、Newton 法）。
pub fn solve_kepler(mean_anomaly_rad: f64, e: f64) -> f64 {
    let m = mean_anomaly_rad.rem_euclid(TAU);
    let mut ea = if e < 0.8 { m } else { core::f64::consts::PI };
    for _ in 0..50 {
        let delta = (ea - e * ea.sin() - m) / (1.0 - e * ea.cos());
        ea -= delta;
        if delta.abs() < 1e-14 {
            break;
        }
    }
    ea
}

/// 近点座標系（P: 近点方向、Q: 進行方向）→ 基準の座標系の回転行列。**この回転はここだけ**（惑星暦も使う）。
pub fn perifocal_to_eci(raan: f64, inc: f64, argp: f64) -> crate::math::Mat3 {
    let (so, co) = raan.sin_cos();
    let (si, ci) = inc.sin_cos();
    let (sw, cw) = argp.sin_cos();
    crate::math::Mat3 {
        m: [
            [co * cw - so * sw * ci, -co * sw - so * cw * ci, so * si],
            [so * cw + co * sw * ci, -so * sw + co * cw * ci, -co * si],
            [sw * si, cw * si, ci],
        ],
    }
}
