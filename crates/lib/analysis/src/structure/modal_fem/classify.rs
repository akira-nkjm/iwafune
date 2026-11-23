//! モードの分類。有効質量（モード質量のうち、機体を各軸に一様に揺らしたときに励起される分）の割合と、
//! 機軸に対する向き（機軸・横・局所・剛体）を決める。
//!
//! 有効質量 mₑ = (Σ φᵢ mᵢ)²（φ は質量で正規化、和は自由な並進の自由度）。割合は全質量（固定した節点の分も含む）に対する比。
//! 拘束した節点の質量は基礎と一緒に動くので、拘束のある系では 3 軸それぞれの割合の総和は 1 以下になる。

use super::assemble::System;
use super::input::ModalFemAxis;
use super::output::ModalFemModeClass;
use super::solve::Solved;
use std::f64::consts::TAU;

/// 振動数がこれ未満（Hz）のモードは剛体（拘束のないモード）とみなす。
/// 弾性のモードがここまで低くなる機体は無く（数 Hz 以上）、計算の丸めで出る 0 の誤差（1e-3 Hz 程度）よりは大きい。
pub const RIGID_BODY_MAX_HZ: f64 = 0.01;
/// 3 軸のどれかの有効質量の割合がこれ未満なら、全体のモードではなく局所のモードとする。
pub const GLOBAL_MODE_MIN_FRACTION: f64 = 0.05;

/// 1 モードの分類の結果。
#[derive(Debug, Clone)]
pub struct ModeData {
    pub frequency_hz: f64,
    /// 軸ごとの有効質量の割合
    pub fractions: [f64; 3],
    pub class: ModalFemModeClass,
    /// 最も効く軸（剛体・局所は `None`）
    pub dominant: Option<ModalFemAxis>,
}

/// 固有値 λ から振動数 [Hz]（負は 0）。
fn frequency_hz(lambda: f64) -> f64 {
    lambda.max(0.0).sqrt() / TAU
}

/// 全モードの有効質量の割合と分類。`total_mass` は並進の質量の合計 [kg]。
pub fn classify(
    sys: &System,
    solved: &Solved,
    total_mass: f64,
    launch_axis: ModalFemAxis,
) -> Vec<ModeData> {
    solved
        .values
        .iter()
        .zip(&solved.vectors)
        .map(|(lambda, phi)| {
            let mut gamma = [0.0; 3];
            for (r, g) in sys.global.iter().enumerate() {
                if g % 6 < 3 {
                    gamma[g % 6] += phi[r] * sys.mass[r];
                }
            }
            let fractions = gamma.map(|x| x * x / total_mass);
            let f = frequency_hz(*lambda);
            let (class, dominant) = mode_class(f, fractions, launch_axis);
            ModeData {
                frequency_hz: f,
                fractions,
                class,
                dominant,
            }
        })
        .collect()
}

/// 振動数と軸ごとの割合から、モードの種類と最も効く軸を決める。
fn mode_class(
    f: f64,
    fractions: [f64; 3],
    launch_axis: ModalFemAxis,
) -> (ModalFemModeClass, Option<ModalFemAxis>) {
    if f < RIGID_BODY_MAX_HZ {
        return (ModalFemModeClass::Rigid, None);
    }
    let a = launch_axis.index();
    let axial = fractions[a];
    let (lat_axis, lateral) = [(a + 1) % 3, (a + 2) % 3]
        .into_iter()
        .map(|i| (i, fractions[i]))
        .fold((a, 0.0), |best, c| if c.1 > best.1 { c } else { best });
    if axial.max(lateral) < GLOBAL_MODE_MIN_FRACTION {
        (ModalFemModeClass::Local, None)
    } else if axial >= lateral {
        (ModalFemModeClass::Axial, Some(launch_axis))
    } else {
        (
            ModalFemModeClass::Lateral,
            ModalFemAxis::from_index(lat_axis),
        )
    }
}
