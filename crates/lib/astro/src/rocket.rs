//! ロケット方程式（Tsiolkovsky）。**推薬と ΔV の換算はここだけ**（アンロードの推薬の ΔV 換算と、ΔV バジェットが共有する）。
//!
//! `Δv = v_e · ln(m₀ / (m₀ − m_p))`、有効排気速度 `v_e = Isp · g₀`。値は SI（m/s・kg・s）。

use crate::constants::G0;

/// 有効排気速度 [m/s]。`isp_s` は比推力 [s]。
pub fn exhaust_velocity(isp_s: f64) -> f64 {
    isp_s * G0
}

/// 推薬 `propellant_kg` を使い切ったときの ΔV [m/s]。`initial_mass_kg` は噴射前の質量。
/// 質量が正でない・推薬が負・推薬が質量以上・有限でない値なら `None`。
pub fn delta_v(exhaust_velocity_m_s: f64, initial_mass_kg: f64, propellant_kg: f64) -> Option<f64> {
    let valid = exhaust_velocity_m_s.is_finite()
        && initial_mass_kg.is_finite()
        && propellant_kg.is_finite()
        && initial_mass_kg > 0.0
        && (0.0..initial_mass_kg).contains(&propellant_kg);
    valid.then(|| exhaust_velocity_m_s * (initial_mass_kg / (initial_mass_kg - propellant_kg)).ln())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn delta_v_by_hand() {
        // Isp 220 s → v_e = 220 × 9.80665 = 2157.463 m/s。10 kg から 1 kg 使う → Δv = 2157.463 × ln(10/9) = 227.31 m/s
        let ve = exhaust_velocity(220.0);
        assert!((ve - 2157.463).abs() < 1e-9);
        let dv = delta_v(ve, 10.0, 1.0).unwrap();
        assert!((dv - 2157.463 * (10.0f64 / 9.0).ln()).abs() < 1e-9);
        assert!((dv - 227.31).abs() < 0.01);
        // 推薬 0 なら 0、少量なら Δv ≈ v_e · m_p / m₀
        assert_eq!(delta_v(ve, 10.0, 0.0), Some(0.0));
        let small = delta_v(ve, 10.0, 1e-6).unwrap();
        assert!((small - ve * 1e-7).abs() < 1e-9);
    }

    #[test]
    fn impossible_masses_are_none() {
        assert_eq!(delta_v(2000.0, 10.0, 10.0), None);
        assert_eq!(delta_v(2000.0, 0.0, 0.0), None);
        assert_eq!(delta_v(2000.0, 10.0, -1.0), None);
    }
}
