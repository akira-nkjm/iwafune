//! 姿勢変更にかかる時間（`aocs.slew_time`）。
//!
//! 静止から静止までの、固定した軸 ê まわりの回転（eigen-axis、rest-to-rest）を、RW が出せるトルクと溜められる角運動量で制限して求める。
//!
//! - 軸まわりの慣性 `I = êᵀ I ê`（重心まわりの慣性テンソル。軸と直交する成分によるジャイロ効果・軸外のトルクは考えない）
//! - 出せるトルク `τ = WheelSet::capacity_along(ê, 最大トルク)`、使える角運動量 `h = capacity_along(ê, 最大角運動量 × 使ってよい割合)`
//!   （[`super::wheels`]。使ってよい割合の既定値もそこ）。ホイールは角運動量 0 から始める（擾乱で溜まった分は使ってよい割合の残りで見込む）
//! - `θ ≤ h² / (I τ)` なら三角形（bang-bang）: `t = 2 √(θ I / τ)`、最大角速度 `√(θ τ / I)`
//! - それより大きければ台形（最大角速度 `h / I` で等速の区間がある）: 加速 `t_a = h / τ`、等速 `t_c = θ I / h − h / τ`、`t = θ I / h + h / τ`
//! - 軸を与えなければ機体の x・y・z の 3 軸で評価し、最も時間のかかる軸を `worst` にする。整定の時間は含めない

use super::wheels::{RwWheel, WheelSet, usable_momentum_fraction};
use crate::spec::{
    Analysis, AnalysisArea, AnalysisError, AnalysisKind, Check, System, Verdict, ensure_positive,
    quantity_text,
};
use sat_astro::math::{Mat3, Vec3};
use sat_units::typed::{
    Angle, AngularMomentum, AngularVelocity, Fraction, MomentOfInertia, Qty, Time, Torque,
};
use serde::{Deserialize, Serialize};
use std::f64::consts::PI;

/// 角度が 180° を超えたとみなす許容差 [rad]（度からの換算の丸めを許す）。
const HALF_TURN_TOLERANCE: f64 = 1e-9;

/// 姿勢変更の時間の入力。
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct SlewTimeInput {
    /// 重心まわりの慣性テンソル [kg·m²]（機体座標系）
    pub inertia: Mat3,
    /// ホイール（台数・配置は任意。軸が 3 次元を張ること）
    pub wheels: Vec<RwWheel>,
    /// 最大角運動量のうち使ってよい割合（0〜1）。省略時 [`super::wheels::DEFAULT_USABLE_MOMENTUM_FRACTION`]
    #[serde(default)]
    pub usable_momentum_fraction: Option<Fraction>,
    /// 姿勢変更の角度（0 より大きく 180° 以下。固有軸まわりの回転は 180° を超えない）
    pub angle: Qty<Angle>,
    /// 回転軸（機体座標系。長さは問わない）。省略時は機体の x・y・z で評価して最悪を取る
    #[serde(default)]
    pub axis: Option<Vec3>,
    /// 姿勢変更の時間の要求上限（省略時は判定しない）
    #[serde(default)]
    pub required_time: Option<Qty<Time>>,
}

/// 角速度の形。**表示名は持たない**（service の語彙が引く）。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(rename_all = "snake_case")]
pub enum SlewProfile {
    /// 最大トルクで加速して、すぐ減速する（角運動量の上限に届かない）
    Triangular,
    /// 角運動量の上限で等速の区間がある
    Trapezoidal,
}

/// 姿勢変更の評価の仮定。**文は持たない**（service の語彙が引く）。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(rename_all = "snake_case")]
pub enum SlewAssumption {
    /// 静止から静止まで固定軸まわりに回る。ホイールは 0 から始め、ジャイロ効果・擾乱・整定の時間は含めない
    EigenAxisRestToRest,
}

/// 1 つの軸での結果。
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct SlewTimeAxis {
    /// 軸の名前（`"x"` など。与えた軸なら `"given"`）
    pub label: String,
    /// 回転軸（機体座標系の単位ベクトル）
    pub axis: Vec3,
    /// 軸まわりの慣性モーメント
    pub moment_of_inertia: Qty<MomentOfInertia>,
    /// 軸まわりに出せるトルク
    #[serde(serialize_with = "sat_units::typed::TorqueKind::serialize")]
    pub torque: Qty<Torque>,
    /// 軸まわりに使える角運動量（使ってよい割合込み）
    pub momentum: Qty<AngularMomentum>,
    /// 最大角速度
    pub peak_rate: Qty<AngularVelocity>,
    /// 加速の時間（減速も同じ）
    pub accel_time: Qty<Time>,
    /// 等速の時間（三角形なら 0）
    pub coast_time: Qty<Time>,
    /// 姿勢変更にかかる時間
    pub time: Qty<Time>,
    pub profile: SlewProfile,
}

/// 姿勢変更の時間の出力。
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct SlewTimeOutput {
    /// 姿勢変更の角度
    pub angle: Qty<Angle>,
    /// 使った「最大角運動量のうち使ってよい割合」（入力が無ければ既定値）
    pub usable_momentum_fraction: Fraction,
    /// 軸ごとの結果（`axis` を与えたらその 1 つ、無ければ機体の x・y・z）
    pub axes: Vec<SlewTimeAxis>,
    /// 最も時間のかかる軸
    pub worst: SlewTimeAxis,
    /// 仮定（文は service が語彙で引く）
    pub assumption: SlewAssumption,
    pub checks: Vec<Check>,
}

/// 姿勢変更にかかる時間。
pub struct SlewTime;

impl Analysis for SlewTime {
    const ID: &'static str = "aocs.slew_time";
    const TITLE: &'static str = "姿勢変更にかかる時間";
    const AREA: AnalysisArea = AnalysisArea::System(System::Aocs);
    const KIND: AnalysisKind = AnalysisKind::Steady;
    const DESCRIPTION: &'static str =
        "RW のトルクと角運動量の上限で、所定の角度の姿勢変更にかかる時間を求める";
    const TITLE_EN: &'static str = "Slew time";
    const DESCRIPTION_EN: &'static str =
        "Time to slew by a given angle within the RW torque and momentum limits";
    type Input = SlewTimeInput;
    type Output = SlewTimeOutput;

    fn run(input: &Self::Input) -> Result<Self::Output, AnalysisError> {
        let theta = input.angle.value();
        ensure_positive("姿勢変更の角度", theta)?;
        if theta > PI + HALF_TURN_TOLERANCE {
            return Err(AnalysisError::InvalidInput(
                "姿勢変更の角度は 180° 以下にしてください（固有軸まわりの回転は 180° を超えない）"
                    .to_owned(),
            ));
        }
        let usable = usable_momentum_fraction(input.usable_momentum_fraction)?;
        let set = WheelSet::new(&input.wheels)?;
        let torques: Vec<f64> = input.wheels.iter().map(|w| w.max_torque.value()).collect();
        let momenta: Vec<f64> = input
            .wheels
            .iter()
            .map(|w| w.max_momentum.value() * usable.value())
            .collect();
        let axes = axes(input.axis)?
            .into_iter()
            .map(|(label, e)| {
                let limits = AxisLimits {
                    inertia: axis_inertia(&input.inertia, e)?,
                    torque: set.capacity_along(e, &torques),
                    momentum: set.capacity_along(e, &momenta),
                };
                Ok(slew_about(label, e, theta, limits))
            })
            .collect::<Result<Vec<_>, AnalysisError>>()?;
        let worst = axes
            .iter()
            .max_by(|a, b| a.time.value().total_cmp(&b.time.value()))
            .cloned()
            .ok_or_else(|| AnalysisError::InvalidInput("評価する軸がありません".to_owned()))?;
        let checks = verdicts(input.required_time, &worst);
        Ok(SlewTimeOutput {
            angle: input.angle,
            usable_momentum_fraction: usable,
            axes,
            worst,
            assumption: SlewAssumption::EigenAxisRestToRest,
            checks,
        })
    }
}

/// 評価する軸（名前と単位ベクトル）。
fn axes(given: Option<Vec3>) -> Result<Vec<(String, Vec3)>, AnalysisError> {
    match given {
        Some(a) => {
            let e = a.normalized().ok_or_else(|| {
                AnalysisError::InvalidInput("回転軸がゼロベクトルです".to_owned())
            })?;
            Ok(vec![("given".to_owned(), e)])
        }
        None => Ok([("x", Vec3::X), ("y", Vec3::Y), ("z", Vec3::Z)]
            .iter()
            .map(|(l, e)| ((*l).to_owned(), *e))
            .collect()),
    }
}

/// 軸 `e`（単位ベクトル）まわりの慣性モーメント `eᵀ I e` [kg·m²]。正でなければ `InvalidInput`。
fn axis_inertia(inertia: &Mat3, e: Vec3) -> Result<f64, AnalysisError> {
    let i = e.dot(inertia.mul_vec(e));
    ensure_positive("軸まわりの慣性モーメント", i)?;
    Ok(i)
}

/// 1 つの軸まわりの値（SI）。
#[derive(Debug, Clone, Copy)]
struct AxisLimits {
    /// 慣性モーメント [kg·m²]
    inertia: f64,
    /// 出せるトルク [N·m]
    torque: f64,
    /// 使える角運動量 [N·m·s]
    momentum: f64,
}

/// 角度 `theta` [rad] を軸まわりの値 `l` で回る時間（三角形か台形）。
fn slew_about(label: String, axis: Vec3, theta: f64, l: AxisLimits) -> SlewTimeAxis {
    let AxisLimits {
        inertia,
        torque,
        momentum,
    } = l;
    let (profile, accel, coast, rate) = if theta * inertia * torque <= momentum * momentum {
        let accel = (theta * inertia / torque).sqrt();
        (
            SlewProfile::Triangular,
            accel,
            0.0,
            torque * accel / inertia,
        )
    } else {
        let accel = momentum / torque;
        let coast = theta * inertia / momentum - accel;
        (SlewProfile::Trapezoidal, accel, coast, momentum / inertia)
    };
    SlewTimeAxis {
        label,
        axis,
        moment_of_inertia: Qty::from_raw(inertia),
        torque: Qty::from_raw(torque),
        momentum: Qty::from_raw(momentum),
        peak_rate: Qty::from_raw(rate),
        accel_time: Qty::from_raw(accel),
        coast_time: Qty::from_raw(coast),
        time: Qty::from_raw(2.0 * accel + coast),
        profile,
    }
}

fn verdicts(required: Option<Qty<Time>>, worst: &SlewTimeAxis) -> Vec<Check> {
    let Some(req) = required else {
        return Vec::new();
    };
    vec![Check::new(
        "slew_time",
        "姿勢変更の時間（最悪の軸）",
        Verdict::from_ok(worst.time <= req),
        format!(
            "{}（要求 {} 以内）",
            quantity_text::<Time>(worst.time.value()),
            quantity_text::<Time>(req.value())
        ),
    )]
}

#[cfg(test)]
mod tests;
