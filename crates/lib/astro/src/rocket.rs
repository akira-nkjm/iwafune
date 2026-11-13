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

/// 噴射前の質量 `initial_mass_kg` から ΔV `delta_v_m_s` を出すのに使う推薬 [kg]: `m₀ (1 − e^(−Δv / v_e))`（[`delta_v`] の逆）。
/// 排気速度・質量が正でない・ΔV が負・有限でない値なら `None`。
pub fn propellant_for(
    exhaust_velocity_m_s: f64,
    initial_mass_kg: f64,
    delta_v_m_s: f64,
) -> Option<f64> {
    let valid = [exhaust_velocity_m_s, initial_mass_kg, delta_v_m_s]
        .iter()
        .all(|v| v.is_finite())
        && exhaust_velocity_m_s > 0.0
        && initial_mass_kg > 0.0
        && delta_v_m_s >= 0.0;
    valid.then(|| initial_mass_kg * -(-delta_v_m_s / exhaust_velocity_m_s).exp_m1())
}

/// 一定の推力 `thrust_n` [N] で推薬 `propellant_kg` を使い切る噴射時間 [s]: `t = m_p v_e / F`（流量 `F / v_e`）。
/// 推力・排気速度が正でない・推薬が負・有限でない値なら `None`。
pub fn burn_time(thrust_n: f64, exhaust_velocity_m_s: f64, propellant_kg: f64) -> Option<f64> {
    let valid = [thrust_n, exhaust_velocity_m_s, propellant_kg]
        .iter()
        .all(|v| v.is_finite())
        && thrust_n > 0.0
        && exhaust_velocity_m_s > 0.0
        && propellant_kg >= 0.0;
    valid.then(|| propellant_kg * exhaust_velocity_m_s / thrust_n)
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

#[cfg(test)]
mod inverse_tests {
    use super::*;

    #[test]
    fn propellant_for_inverts_delta_v() {
        // Isp 220 s、10 kg で 227.31 m/s → 1 kg（`delta_v_by_hand` の逆）
        let ve = exhaust_velocity(220.0);
        let dv = delta_v(ve, 10.0, 1.0).unwrap();
        let mp = propellant_for(ve, 10.0, dv).unwrap();
        assert!((mp - 1.0).abs() < 1e-12);
        assert_eq!(propellant_for(ve, 10.0, 0.0), Some(0.0));
        assert_eq!(propellant_for(ve, 10.0, -1.0), None);
        assert_eq!(propellant_for(0.0, 10.0, 1.0), None);
    }

    #[test]
    fn burn_time_by_hand() {
        // 1 kg を v_e 2157.463 m/s・推力 1 N で使う → 2157.463 s（流量 1 / 2157.463 kg/s）
        let t = burn_time(1.0, 2157.463, 1.0).unwrap();
        assert!((t - 2157.463).abs() < 1e-9);
        assert_eq!(burn_time(0.0, 2000.0, 1.0), None);
    }
}
