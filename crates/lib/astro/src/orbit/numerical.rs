//! 数値伝播（Cowell 法）。[`crate::force::acceleration_eci`] を [`crate::ode::Dop853`] で積分し、
//! 要求された時刻列を 1 回の積分の密出力で返す。

use super::OrbitState;
use crate::AstroError;
use crate::ephem::CentralBody;
use crate::force::{ForceConfig, acceleration_eci_about};
use crate::math::Vec3;
use crate::ode::Dop853;
use crate::time::Epoch;

/// 数値伝播の積分器の設定（位置 [m]・速度 [m/s] の混在する状態に合わせた許容誤差）。
pub const NUMERICAL_INTEGRATOR: Dop853 = Dop853 {
    rtol: 1e-11,
    atol: 1e-6,
    max_step: 600.0,
    max_steps: 2_000_000,
};

/// 太陽中心の数値伝播の積分器: 精度は [`NUMERICAL_INTEGRATOR`] と同じで、刻みの上限だけを 1 日にする
/// （惑星間は 600 s の上限が要らない）。**太陽中心の積分の設定はここだけ**（巡航の確認 `transfer::cruise` と
/// 状態ベクトルの軌道 [`super::StateSpec`] が使う）。
pub const HELIOCENTRIC_INTEGRATOR: Dop853 = Dop853 {
    max_step: crate::constants::SECONDS_PER_DAY,
    ..NUMERICAL_INTEGRATOR
};

fn to_array(s: &OrbitState) -> [f64; 6] {
    [
        s.r_eci.x, s.r_eci.y, s.r_eci.z, s.v_eci.x, s.v_eci.y, s.v_eci.z,
    ]
}

fn from_array(y: &[f64; 6]) -> OrbitState {
    OrbitState {
        r_eci: Vec3::new(y[0], y[1], y[2]),
        v_eci: Vec3::new(y[3], y[4], y[5]),
    }
}

/// 元期 `epoch` の状態 `x0` から、元期からの秒 `times_s`（昇順・非負）での状態を求める（既定の積分器
/// [`NUMERICAL_INTEGRATOR`]）。
pub(super) fn states_at(
    central: CentralBody,
    forces: &ForceConfig,
    epoch: Epoch,
    x0: OrbitState,
    times_s: &[f64],
) -> Result<Vec<OrbitState>, AstroError> {
    propagate_states(central, forces, epoch, x0, times_s, &NUMERICAL_INTEGRATOR)
}

/// 任意の初期状態 `x0`（中心天体 `central` からの相対・ECI の向き）を力モデルで数値伝播し、元期からの秒 `times_s`
/// （昇順・非負）での状態を、1 回の積分の密出力で返す。**要素でなく状態から始める入口**で、中心天体は太陽でも惑星でもよい
/// （惑星間の巡航・ラグランジュ点まわりの軌道など、ケプラー要素で書けない軌道）。積分器は呼び出し側が選ぶ
/// （惑星間では刻みの上限を [`NUMERICAL_INTEGRATOR`] の 600 s より大きくできる）。
pub fn propagate_states(
    central: CentralBody,
    forces: &ForceConfig,
    epoch: Epoch,
    x0: OrbitState,
    times_s: &[f64],
    integrator: &Dop853,
) -> Result<Vec<OrbitState>, AstroError> {
    crate::force::validate_about(central, forces)?;
    let f = |t: f64, y: &[f64; 6]| {
        let state = from_array(y);
        let a = acceleration_eci_about(central, forces, epoch.plus_seconds(t), &state);
        [y[3], y[4], y[5], a.x, a.y, a.z]
    };
    let out = integrator.integrate_at(f, 0.0, to_array(&x0), times_s)?;
    Ok(out.iter().map(from_array).collect())
}
