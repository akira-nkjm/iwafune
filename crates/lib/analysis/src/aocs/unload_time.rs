//! アンロードにかかる時間（`aocs.unload_time`）。
//!
//! - **捨てる量**: `momentum` を与えなければ、ホイールの最大角運動量の最大 × 使ってよい割合（既定 0.8）。
//! - **MTQ**: 出せるトルクは `τ = η · m_eff · |B|`。`m_eff` は MTQ の組が機体のどの軸の向きにも出せる双極子の最小
//!   （各機体軸 j について `Σ_i m_max,i |a_i·e_j|` を求め、その最小）。`|B|` は軌道に沿った磁場の大きさの時間平均
//!   （`sat_astro::geomag::field_about_eci_t`。標本は `sat_astro::timeline::OrbitSampling`）。磁場のモデルは地球だけで、ほかの中心天体の
//!   軌道では MTQ を評価せず（`mtq` は `None`）、理由と RCS で足りるかを `mtq_field_gap` に返す（[`MtqFieldGap`]。RCS の評価は続ける）。
//!   η は **双極子が B と直交する成分しかトルクにならず、B の向きが軌道上で変わることを見込んだ平均の有効率**で、
//!   入力で与える（省略時 0.5。概算）。
//! - **RCS**: スラスタの取付位置・推力の向きと重心から各スラスタのトルクを求め、捨てる向き（−ĥ）の成分が正の
//!   スラスタを同時に噴射する（[`rcs`]）。時間 `t = H / T`、推薬 `m = Σ F_i t / (Isp_i g₀)`。

use super::thrusters::Thruster;
use super::wheels::{RwWheel, usable_momentum_fraction};
use crate::spec::quantity_text;
use crate::spec::{
    Analysis, AnalysisArea, AnalysisError, AnalysisKind, Check, System, Verdict, ensure_fraction,
    ensure_non_negative, ensure_positive,
};
use sat_astro::geomag::field_about_eci_t;
use sat_astro::math::Vec3;
use sat_astro::timeline::{OrbitSampling, time_mean};
use sat_units::typed::{
    AngularMomentum, Fraction, MagneticDipoleMoment, MagneticFluxDensity, Qty, Time, Torque,
};
use serde::{Deserialize, Serialize};

/// MTQ の有効率の既定値。**既定値はここだけ。**
pub const DEFAULT_MTQ_EFFICIENCY: Fraction = Fraction::new(0.5);

/// アンロードにかかる時間の入力。
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct UnloadTimeInput {
    /// 捨てる角運動量（大きさ）。省略時は `wheels` の最大角運動量の最大 × `usable_momentum_fraction`
    #[serde(default)]
    pub momentum: Option<Qty<AngularMomentum>>,
    /// ホイール（`momentum` を省略したときに捨てる量を決める）
    #[serde(default)]
    pub wheels: Vec<RwWheel>,
    /// 最大角運動量のうち使ってよい割合（0〜1）。省略時 [`super::wheels::DEFAULT_USABLE_MOMENTUM_FRACTION`]
    #[serde(default)]
    pub usable_momentum_fraction: Option<Fraction>,
    /// 捨てる角運動量の向き ĥ（機体座標系。RCS の評価に使う）。省略時は機体の ±x・±y・±z で評価して最悪を取る
    #[serde(default)]
    pub momentum_direction: Option<Vec3>,
    /// MTQ（空なら MTQ の評価をしない）
    #[serde(default)]
    pub magnetorquers: Vec<UnloadTimeMtq>,
    /// 地磁気の大きさの与え方（MTQ があるときに使う）
    #[serde(default)]
    pub field: Option<UnloadTimeField>,
    /// MTQ の平均の有効率（0〜1）。省略時 [`DEFAULT_MTQ_EFFICIENCY`]
    #[serde(default)]
    pub mtq_efficiency: Option<Fraction>,
    /// スラスタ（空なら RCS の評価をしない）
    #[serde(default)]
    pub thrusters: Vec<Thruster>,
    /// 重心 [m]（機体座標系）。スラスタがあるときは必須
    #[serde(default)]
    pub center_of_mass: Option<Vec3>,
    /// アンロード時間の要求上限（省略時は判定しない）
    #[serde(default)]
    pub required_time: Option<Qty<Time>>,
}

impl UnloadTimeInput {
    /// MTQ の平均の有効率（省略時 [`DEFAULT_MTQ_EFFICIENCY`]）。
    pub fn mtq_efficiency(&self) -> Fraction {
        self.mtq_efficiency.unwrap_or(DEFAULT_MTQ_EFFICIENCY)
    }
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
    /// 軌道に沿って平均する（期間・刻み・β は時系列と同じ標本の取り方）
    Orbit { sampling: OrbitSampling },
    /// 大きさを直接与える
    Constant { strength: Qty<MagneticFluxDensity> },
}

/// アンロードの評価の仮定。**文は持たない**（service の語彙が日本語と英語で引く）。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(rename_all = "snake_case")]
pub enum UnloadAssumption {
    /// MTQ: トルク = 有効率 × 最も弱い機体軸の双極子 × 軌道平均の |B|（B と直交する成分だけが使え、B の向きの変化を有効率で見込む）
    MtqMeanField,
    /// RCS: −H 向きのトルク成分が正のスラスタをすべて同時に連続噴射する（H と直交するトルクと正味の推力は残る）
    RcsSimultaneousFiring,
}

/// アンロードにかかる時間の出力。
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct UnloadTimeOutput {
    /// 捨てる角運動量（入力の値か、ホイールから求めた値）
    pub momentum: Qty<AngularMomentum>,
    /// 捨てる量をホイールから求めたときに使った「使ってよい割合」（`momentum` を与えたときは `None`）
    pub usable_momentum_fraction: Option<Fraction>,
    /// MTQ での結果（MTQ が無いか、中心天体の磁場で評価できなければ `None`）
    pub mtq: Option<UnloadTimeMtqResult>,
    /// MTQ を中心天体の磁場で評価できなかった理由と、RCS で足りるか（評価したか MTQ が無ければ `None`）
    #[serde(default)]
    pub mtq_field_gap: Option<MtqFieldGap>,
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
    /// 使った有効率（入力が無ければ既定値）
    pub efficiency: Fraction,
    /// 平均のアンロードトルク（η · m_eff · |B| の平均）
    #[serde(serialize_with = "sat_units::typed::TorqueKind::serialize")]
    pub torque: Qty<Torque>,
    pub time: Qty<Time>,
    /// 時間を軌道周期で割ったもの（地磁気を軌道から求めたときだけ）
    pub orbits: Option<f64>,
    /// 仮定（文は service が語彙で引く。有効率の値は `efficiency`）
    pub assumption: UnloadAssumption,
}

/// アンロードにかかる時間。
pub struct UnloadTime;

impl Analysis for UnloadTime {
    const ID: &'static str = "aocs.unload_time";
    const TITLE: &'static str = "アンロードにかかる時間";
    const AREA: AnalysisArea = AnalysisArea::System(System::Aocs);
    const KIND: AnalysisKind = AnalysisKind::Steady;
    const DESCRIPTION: &'static str = "MTQ・RCS で所定の角運動量を捨てるのにかかる時間";
    const TITLE_EN: &'static str = "Momentum unloading time";
    const DESCRIPTION_EN: &'static str =
        "Time needed to dump a given momentum with magnetorquers or RCS";
    type Input = UnloadTimeInput;
    type Output = UnloadTimeOutput;

    fn run(input: &Self::Input) -> Result<Self::Output, AnalysisError> {
        let (h, usable) = dump_momentum(input)?;
        let mtq_field_reason = if input.magnetorquers.is_empty() {
            None
        } else {
            field_reason(input.field.as_ref())
        };
        let mtq = if input.magnetorquers.is_empty() || mtq_field_reason.is_some() {
            None
        } else {
            Some(mtq_result(input, h)?)
        };
        let rcs = if input.thrusters.is_empty() {
            None
        } else {
            let com = input.center_of_mass.ok_or_else(|| {
                AnalysisError::InvalidInput("RCS evaluation requires center_of_mass".to_owned())
            })?;
            Some(rcs_unload(
                h,
                input.momentum_direction,
                &input.thrusters,
                com,
            )?)
        };
        let checks = verdicts(input, mtq.as_ref(), rcs.as_ref());
        // RCS がすべての向きを捨てられるなら（最悪の向きの時間がある）、MTQ を使えなくても問題ではない
        let covered_by_rcs = rcs.as_ref().is_some_and(|r| r.worst.time.is_some());
        let mtq_field_gap = mtq_field_reason.map(|reason| MtqFieldGap {
            reason,
            covered_by_rcs,
        });
        Ok(UnloadTimeOutput {
            momentum: Qty::from_raw(h),
            usable_momentum_fraction: usable,
            mtq,
            mtq_field_gap,
            rcs,
            checks,
        })
    }
}

/// 捨てる角運動量 [N·m·s] と、ホイールから求めたときに使った割合。入力に無ければ、ホイールの最大角運動量の最大 × 使ってよい割合。
fn dump_momentum(input: &UnloadTimeInput) -> Result<(f64, Option<Fraction>), AnalysisError> {
    if let Some(h) = input.momentum {
        ensure_non_negative("Momentum to dump", h.value())?;
        return Ok((h.value(), None));
    }
    let usable = usable_momentum_fraction(input.usable_momentum_fraction)?;
    let max = input
        .wheels
        .iter()
        .map(|w| w.max_momentum.value())
        .fold(f64::NAN, f64::max);
    if !max.is_finite() {
        return Err(AnalysisError::InvalidInput(
            "Either momentum to dump (momentum) or wheels is required".to_owned(),
        ));
    }
    ensure_non_negative("Wheel max angular momentum", max)?;
    Ok((max * usable.value(), Some(usable)))
}

/// どの機体軸の向きにも出せる双極子の最小 [A·m²]。
pub fn effective_dipole(mtqs: &[UnloadTimeMtq]) -> Result<f64, AnalysisError> {
    let mut axes = Vec::with_capacity(mtqs.len());
    for m in mtqs {
        ensure_non_negative(&format!("MTQ {} max dipole", m.id), m.max_dipole.value())?;
        let a = m.axis.normalized().ok_or_else(|| {
            AnalysisError::InvalidInput(format!("MTQ {} axis is a zero vector", m.id))
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
        UnloadTimeField::Orbit { sampling } => {
            // 磁場は位置だけで決まるので、姿勢則の要らない軌道の標本で求める（β の指定は RAAN に当てる）
            let central = sampling.orbit.central;
            if MtqFieldReason::of(central).is_some() {
                // `run` が先に外すので通らない（磁場を 0 として計算しないための守り）
                return Err(AnalysisError::InvalidInput(format!(
                    "The magnetic field of this body is not supported (central body: {central:?})"
                )));
            }
            let points = sampling.states()?;
            let t: Vec<f64> = points.iter().map(|p| p.t_s).collect();
            let b: Vec<f64> = points
                .iter()
                .map(|p| field_about_eci_t(central, p.state.r_eci, p.epoch).norm())
                .collect();
            if b.is_empty() {
                return Err(AnalysisError::InvalidInput(
                    "The geomagnetic time series is empty".to_owned(),
                ));
            }
            let mean = time_mean(&t, &b);
            let min = b.iter().copied().fold(f64::INFINITY, f64::min);
            let max = b.iter().copied().fold(0.0, f64::max);
            Ok(((mean, min, max), Some(sampling.orbit.period_s())))
        }
    }
}

fn mtq_result(input: &UnloadTimeInput, h: f64) -> Result<UnloadTimeMtqResult, AnalysisError> {
    let eta = input.mtq_efficiency();
    ensure_fraction("MTQ effectiveness", eta.value())?;
    ensure_positive("MTQ effectiveness", eta.value())?;
    let field = input.field.as_ref().ok_or_else(|| {
        AnalysisError::InvalidInput("MTQ evaluation requires the magnetic field (field)".to_owned())
    })?;
    let ((mean, min, max), period) = field_stats(field)?;
    let m_eff = effective_dipole(&input.magnetorquers)?;
    let torque = eta.value() * m_eff * mean;
    if torque <= 0.0 {
        return Err(AnalysisError::InvalidInput("MTQ torque is 0".to_owned()));
    }
    let time = h / torque;
    Ok(UnloadTimeMtqResult {
        mean_field: Qty::from_raw(mean),
        min_field: Qty::from_raw(min),
        max_field: Qty::from_raw(max),
        effective_dipole: Qty::from_raw(m_eff),
        efficiency: eta,
        torque: Qty::from_raw(torque),
        time: Qty::from_raw(time),
        orbits: period.map(|p| time / p),
        assumption: UnloadAssumption::MtqMeanField,
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
            "Directions RCS can unload",
            Verdict::Violation,
            format!(
                "No thruster can produce torque to dump momentum along {}",
                r.worst.label
            ),
        ));
    }
    let Some(req) = input.required_time else {
        return checks;
    };
    let mut check = |key: &str, label: &str, time: Option<Qty<Time>>| {
        let v = Verdict::from_ok(time.is_some_and(|t| t <= req));
        let shown = time.map_or("cannot unload".to_owned(), |t| {
            quantity_text::<Time>(t.value())
        });
        checks.push(Check::new(
            key,
            label,
            v,
            format!(
                "{shown} (required within {})",
                quantity_text::<Time>(req.value())
            ),
        ));
    };
    if let Some(m) = mtq {
        check("mtq_time", "Unload time with MTQ", Some(m.time));
    }
    if let Some(r) = rcs {
        check(
            "rcs_time",
            "Unload time with RCS (worst direction)",
            r.worst.time,
        );
    }
    checks
}

mod field_gap;
use field_gap::field_reason;
pub use field_gap::{MTQ_FIELD_MODEL_CHECK, MtqFieldGap, MtqFieldReason};
pub mod rcs;
pub use rcs::{UnloadTimeRcs, UnloadTimeRcsResult, rcs_unload};

#[cfg(test)]
mod tests;
