//! アンロードにかかる時間（`attitude.unload_time`）。
//!
//! - **捨てる量**: `momentum` を与えなければ、ホイールの最大角運動量の最大 × 使ってよい割合（既定 0.8）。
//! - **MTQ**: 出せるトルクは `τ = η · m_eff · |B|`。`m_eff` は MTQ の組が機体のどの軸の向きにも出せる双極子の最小
//!   （各機体軸 j について `Σ_i m_max,i |a_i·e_j|` を求め、その最小）。`|B|` は軌道に沿った磁場の大きさの時間平均
//!   （`sat_astro::geomag::field_about_eci_t`、中心天体が地球以外なら 0。時系列は `sat_astro::timeline`）。η は **双極子が B と直交する成分しかトルクにならず、
//!   B の向きが軌道上で変わることを見込んだ平均の有効率**で、入力で与える（省略時 0.5。概算）。
//! - **RCS**: スラスタの取付位置・推力の向きと重心から各スラスタのトルクを求め、捨てる向き（−ĥ）の成分が正の
//!   スラスタを同時に噴射する（[`rcs`]）。時間 `t = H / T`、推薬 `m = Σ F_i t / (Isp_i g₀)`。

use super::wheels::{RwWheel, default_usable_fraction};
use crate::spec::quantity_text;
use crate::spec::{
    Analysis, AnalysisError, AnalysisKind, Check, System, Verdict, ensure_fraction,
    ensure_non_negative, ensure_positive,
};
use sat_astro::attitude::AttitudeLaw;
use sat_astro::geomag::field_about_eci_t;
use sat_astro::math::{Quat, Vec3};
use sat_astro::orbit::OrbitSpec;
use sat_astro::timeline::DEFAULT_STEP_S;
use sat_astro::timeline::{self, TimelineSpec, time_mean};
use sat_units::typed::{
    AngularMomentum, Fraction, MagneticDipoleMoment, MagneticFluxDensity, Qty, Time, Torque,
};
use serde::{Deserialize, Serialize};

/// MTQ の有効率の既定値。
pub const DEFAULT_MTQ_EFFICIENCY: f64 = 0.5;

/// アンロードにかかる時間の入力。
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct UnloadTimeInput {
    /// 捨てる角運動量（大きさ）。省略時は `wheels` の最大角運動量の最大 × `usable_fraction`
    #[serde(default)]
    pub momentum: Option<Qty<AngularMomentum>>,
    /// ホイール（`momentum` を省略したときに捨てる量を決める）
    #[serde(default)]
    pub wheels: Vec<RwWheel>,
    /// 最大角運動量のうち使ってよい割合（0〜1）。省略時 0.8
    #[serde(default = "default_usable_fraction")]
    pub usable_fraction: f64,
    /// 捨てる角運動量の向き ĥ（機体座標系。RCS の評価に使う）。省略時は機体の ±x・±y・±z で評価して最悪を取る
    #[serde(default)]
    pub momentum_direction: Option<Vec3>,
    /// MTQ（空なら MTQ の評価をしない）
    #[serde(default)]
    pub magnetorquers: Vec<UnloadTimeMtq>,
    /// 地磁気の大きさの与え方（MTQ があるときに使う）
    #[serde(default)]
    pub field: Option<UnloadTimeField>,
    /// MTQ の平均の有効率（0〜1）。省略時 0.5
    #[serde(default)]
    pub mtq_efficiency: Option<f64>,
    /// スラスタ（空なら RCS の評価をしない）
    #[serde(default)]
    pub thrusters: Vec<UnloadTimeThruster>,
    /// 重心 [m]（機体座標系）。スラスタがあるときは必須
    #[serde(default)]
    pub center_of_mass: Option<Vec3>,
    /// アンロード時間の要求上限（省略時は判定しない）
    #[serde(default)]
    pub required_time: Option<Qty<Time>>,
}

/// 磁気トルカ 1 台。
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct UnloadTimeMtq {
    pub id: String,
    /// 双極子の向き（機体座標系。長さは問わない）
    pub axis: Vec3,
    pub max_dipole: Qty<MagneticDipoleMoment>,
}

/// 地磁気の大きさの与え方。
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum UnloadTimeField {
    /// 軌道に沿って平均する。期間の省略時は 1 周、刻みの省略時は 30 s
    Orbit {
        orbit: OrbitSpec,
        #[serde(default)]
        span: Option<Qty<Time>>,
        #[serde(default)]
        step: Option<Qty<Time>>,
    },
    /// 大きさを直接与える
    Constant { strength: Qty<MagneticFluxDensity> },
}

/// アンロードにかかる時間の出力。
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct UnloadTimeOutput {
    /// 捨てる角運動量（入力の値か、ホイールから求めた値）
    pub momentum: Qty<AngularMomentum>,
    pub mtq: Option<UnloadTimeMtqResult>,
    pub rcs: Option<UnloadTimeRcs>,
    pub checks: Vec<Check>,
}

/// MTQ での結果。
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct UnloadTimeMtqResult {
    /// 地磁気の大きさの平均・最小・最大
    pub mean_field: Qty<MagneticFluxDensity>,
    pub min_field: Qty<MagneticFluxDensity>,
    pub max_field: Qty<MagneticFluxDensity>,
    /// どの機体軸の向きにも出せる双極子の最小
    pub effective_dipole: Qty<MagneticDipoleMoment>,
    /// 使った有効率
    pub efficiency: Fraction,
    /// 平均のアンロードトルク（η · m_eff · |B| の平均）
    pub torque: Qty<Torque>,
    pub time: Qty<Time>,
    /// 時間を軌道周期で割ったもの（地磁気を軌道から求めたときだけ）
    pub orbits: Option<f64>,
    /// 仮定の説明
    pub assumption: String,
}

/// アンロードにかかる時間。
pub struct UnloadTime;

impl Analysis for UnloadTime {
    const ID: &'static str = "attitude.unload_time";
    const TITLE: &'static str = "アンロードにかかる時間";
    const SYSTEM: System = System::Attitude;
    const KIND: AnalysisKind = AnalysisKind::Steady;
    const DESCRIPTION: &'static str = "MTQ・RCS で所定の角運動量を捨てるのにかかる時間";
    const TITLE_EN: &'static str = "Momentum unloading time";
    const DESCRIPTION_EN: &'static str =
        "Time needed to dump a given momentum with magnetorquers or RCS";
    type Input = UnloadTimeInput;
    type Output = UnloadTimeOutput;

    fn run(input: &Self::Input) -> Result<Self::Output, AnalysisError> {
        let h = dump_momentum(input)?;
        let mtq = if input.magnetorquers.is_empty() {
            None
        } else {
            Some(mtq_result(input, h)?)
        };
        let rcs = if input.thrusters.is_empty() {
            None
        } else {
            let com = input.center_of_mass.ok_or_else(|| {
                AnalysisError::InvalidInput(
                    "RCS の評価には重心（center_of_mass）が要ります".to_owned(),
                )
            })?;
            Some(rcs_unload(
                h,
                input.momentum_direction,
                &input.thrusters,
                com,
            )?)
        };
        let checks = verdicts(input, mtq.as_ref(), rcs.as_ref());
        Ok(UnloadTimeOutput {
            momentum: Qty::from_raw(h),
            mtq,
            rcs,
            checks,
        })
    }
}

/// 捨てる角運動量 [N·m·s]。入力に無ければ、ホイールの最大角運動量の最大 × 使ってよい割合。
fn dump_momentum(input: &UnloadTimeInput) -> Result<f64, AnalysisError> {
    if let Some(h) = input.momentum {
        ensure_non_negative("捨てる角運動量", h.value())?;
        return Ok(h.value());
    }
    ensure_fraction("使える割合", input.usable_fraction)?;
    let max = input
        .wheels
        .iter()
        .map(|w| w.max_momentum.value())
        .fold(f64::NAN, f64::max);
    if !max.is_finite() {
        return Err(AnalysisError::InvalidInput(
            "捨てる角運動量（momentum）かホイール（wheels）が要ります".to_owned(),
        ));
    }
    ensure_non_negative("ホイールの最大角運動量", max)?;
    Ok(max * input.usable_fraction)
}

/// どの機体軸の向きにも出せる双極子の最小 [A·m²]。
pub fn effective_dipole(mtqs: &[UnloadTimeMtq]) -> Result<f64, AnalysisError> {
    let mut axes = Vec::with_capacity(mtqs.len());
    for m in mtqs {
        ensure_non_negative(&format!("MTQ {} の最大双極子", m.id), m.max_dipole.value())?;
        let a = m.axis.normalized().ok_or_else(|| {
            AnalysisError::InvalidInput(format!("MTQ {} の軸がゼロベクトルです", m.id))
        })?;
        axes.push((a, m.max_dipole.value()));
    }
    Ok([Vec3::X, Vec3::Y, Vec3::Z]
        .iter()
        .map(|e| axes.iter().map(|(a, m)| m * a.dot(*e).abs()).sum::<f64>())
        .fold(f64::INFINITY, f64::min))
}

/// 地磁気の大きさの (平均, 最小, 最大) [T]。
type FieldStats = (f64, f64, f64);

/// 地磁気の大きさの統計と軌道周期 [s]（軌道から求めたとき）。
fn field_stats(field: &UnloadTimeField) -> Result<(FieldStats, Option<f64>), AnalysisError> {
    match field {
        UnloadTimeField::Constant { strength } => {
            let b = strength.value();
            Ok(((b, b, b), None))
        }
        UnloadTimeField::Orbit { orbit, span, step } => {
            let period = orbit.period_s();
            let spec = TimelineSpec::new(
                orbit.clone(),
                AttitudeLaw::Inertial {
                    q_body_to_eci: Quat::IDENTITY,
                },
                span.unwrap_or(Qty::from_raw(period)),
                step.unwrap_or(Qty::from_raw(DEFAULT_STEP_S)),
            );
            let samples = timeline::build(&spec)?;
            let t: Vec<f64> = samples.iter().map(|s| s.t_s).collect();
            let b: Vec<f64> = samples
                .iter()
                .map(|s| field_about_eci_t(orbit.central, s.r_eci, s.epoch).norm())
                .collect();
            if b.is_empty() {
                return Err(AnalysisError::InvalidInput(
                    "地磁気の時系列が空です".to_owned(),
                ));
            }
            let mean = time_mean(&t, &b);
            let min = b.iter().copied().fold(f64::INFINITY, f64::min);
            let max = b.iter().copied().fold(0.0, f64::max);
            Ok(((mean, min, max), Some(period)))
        }
    }
}

fn mtq_result(input: &UnloadTimeInput, h: f64) -> Result<UnloadTimeMtqResult, AnalysisError> {
    let eta = input.mtq_efficiency.unwrap_or(DEFAULT_MTQ_EFFICIENCY);
    ensure_fraction("MTQ の有効率", eta)?;
    ensure_positive("MTQ の有効率", eta)?;
    let field = input.field.as_ref().ok_or_else(|| {
        AnalysisError::InvalidInput("MTQ の評価には地磁気（field）が要ります".to_owned())
    })?;
    let ((mean, min, max), period) = field_stats(field)?;
    let m_eff = effective_dipole(&input.magnetorquers)?;
    let torque = eta * m_eff * mean;
    if torque <= 0.0 {
        return Err(AnalysisError::InvalidInput(
            "MTQ のトルクが 0 です".to_owned(),
        ));
    }
    let time = h / torque;
    Ok(UnloadTimeMtqResult {
        mean_field: Qty::from_raw(mean),
        min_field: Qty::from_raw(min),
        max_field: Qty::from_raw(max),
        effective_dipole: Qty::from_raw(m_eff),
        efficiency: Fraction::new(eta),
        torque: Qty::from_raw(torque),
        time: Qty::from_raw(time),
        orbits: period.map(|p| time / p),
        assumption: format!(
            "トルク = 有効率 {eta} × 最も弱い機体軸の双極子 × 軌道平均の |B|。B と直交する成分だけが使え、B の向きが軌道上で変わることを有効率で見込む"
        ),
    })
}

fn verdicts(
    input: &UnloadTimeInput,
    mtq: Option<&UnloadTimeMtqResult>,
    rcs: Option<&UnloadTimeRcs>,
) -> Vec<Check> {
    let mut checks = Vec::new();
    if let Some(r) = rcs
        && r.worst.time.is_none()
    {
        checks.push(Check::new(
            "rcs_direction",
            "RCS で捨てられる向き",
            Verdict::Violation,
            format!(
                "{} 向きの角運動量を捨てるトルクを出せるスラスタがありません",
                r.worst.label
            ),
        ));
    }
    let Some(req) = input.required_time else {
        return checks;
    };
    let mut check = |key: &str, label: &str, time: Option<Qty<Time>>| {
        let ok = time.is_some_and(|t| t <= req);
        let v = if ok { Verdict::Ok } else { Verdict::Violation };
        let shown = time.map_or("捨てられない".to_owned(), |t| {
            quantity_text(t.value(), "Time")
        });
        checks.push(Check::new(
            key,
            label,
            v,
            format!(
                "{shown}（要求 {} 以内）",
                quantity_text(req.value(), "Time")
            ),
        ));
    };
    if let Some(m) = mtq {
        check("mtq_time", "MTQ でのアンロード時間", Some(m.time));
    }
    if let Some(r) = rcs {
        check(
            "rcs_time",
            "RCS でのアンロード時間（最悪の向き）",
            r.worst.time,
        );
    }
    checks
}

pub mod rcs;
pub use rcs::{UnloadTimeRcs, UnloadTimeRcsResult, UnloadTimeThruster, rcs_unload};

#[cfg(test)]
mod tests;
