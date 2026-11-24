//! 打ち上げ適合（`structure.launch_fit`）。衛星が、選んだ打ち上げ機・ディスペンサーの条件に収まるかの判定。
//!
//! - 質量: 打ち上げ時の質量（部品の質量 × 員数に成長余裕を掛けたものの合計 + 積む推薬）≤ 搭載質量の上限。
//!   成長余裕の掛け方は質量バジェットと同じ（[`budget_rollup`]。推薬は成長余裕 0 の 1 行）で、バジェットの推薬込みの合計と同じ質量を判定する。
//!   余裕を掛けない質量（素の質量）も出力に残す。
//! - 外形: 収納状態の外形を **回転せずに軸ごとに** 許容の箱と比べる（機体座標の x は x と、y は y と、z は z と。発射軸は z）。
//!   辺の入れ替えは許さない（置き方は設計が決めるので、解析は軸の入れ替えで合わせにいかない）。
//! - 固有振動数: 簡易固有振動数（`structure.modal_estimate`）の機体全体の梁の一次（横・機軸）の最小値を、下限と比べる。
//!   パネルの面外モードは局所のモードで、機体全体の剛性の条件とは別なので比べない。機体全体の見積もりが無ければ、判定せず
//!   警告にする（「確かめていない」を適合と読まれないため）。
//!
//! 限度の値は呼び出し側（projection）が **候補の並び**で渡す。1 つ目が打ち上げ機・ディスペンサーの値（基本の限度）で、
//! 続くものが要求の値。**限度の候補から効く 1 つを選ぶ規則はここだけ**（[`tightest_max_mass`]・[`tightest_min_frequency`]。
//! 厳しいほうが効く）で、どこから来た値かは判定の文に書く。

mod capability;

pub use capability::{CapabilityPoint, LaunchFitCapability, mass_at_c3};

use super::modal_estimate::ModalEstimateKind;
use crate::budget::{BudgetItem, budget_rollup};
use crate::spec::quantity_text;
use crate::spec::{
    Analysis, AnalysisArea, AnalysisError, AnalysisKind, Check, System, Verdict, ratio_margin,
    ratio_margin_verdict,
};
use sat_astro::math::Vec3;
use sat_units::typed::{Fraction, Frequency, Length, Mass, Qty, SpecificEnergy};
use serde::{Deserialize, Serialize};

/// 適合の余裕の警告の幅。0 なので、収まれば ok、収まらなければ違反の二段階（適合か不適合か）。
pub const FIT_MARGIN_WARNING_BAND: Fraction = Fraction::new(0.0);

/// 軸の名前（機体座標。外形の並びと同じ順）。
const AXES: [&str; 3] = ["x", "y", "z"];

/// 質量の上限の候補 1 つ。
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct LaunchFitMassLimit {
    /// 値の出所（`launcher local/lv_alpha`・`requirement R_MASS` など。判定の文に書く）
    pub origin: String,
    pub value: Qty<Mass>,
}

/// 固有振動数の下限の候補 1 つ。
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct LaunchFitFrequencyLimit {
    /// 値の出所
    pub origin: String,
    pub value: Qty<Frequency>,
}

/// 許容の外形（機体座標の軸ごとの長さ [m]）。
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct LaunchFitEnvelope {
    /// 値の出所
    pub origin: String,
    /// x, y, z 方向の許容の長さ [m]（z が発射軸）
    pub size: Vec3,
}

/// 機体全体の一次固有振動数の見積もり 1 つ（`structure.modal_estimate` の結果）。
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct LaunchFitMode {
    pub id: String,
    pub kind: ModalEstimateKind,
    pub frequency: Qty<Frequency>,
}

/// 打ち上げ適合の入力。
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct LaunchFitInput {
    /// 部品の行（1 個あたりの質量・員数・成長余裕。質量バジェットと同じ行）
    pub parts: Vec<BudgetItem>,
    /// 積む推薬（推進系が無ければ `None`）
    #[serde(default)]
    pub propellant_mass: Option<Qty<Mass>>,
    /// 収納状態の外形（機体座標の x, y, z の長さ [m]。無ければ外形は判定しない）
    #[serde(default)]
    pub size: Option<Vec3>,
    /// 質量の上限の候補（無ければ質量は判定しない）
    #[serde(default)]
    pub max_mass: Vec<LaunchFitMassLimit>,
    /// 打ち上げ機が出発を受け持つ遷移の C3 と、能力の曲線（あれば、曲線を C3 で引いた質量が上限の候補に加わる）
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub capability: Option<LaunchFitCapability>,
    /// 許容の外形（無ければ外形は判定しない）
    #[serde(default)]
    pub envelope: Option<LaunchFitEnvelope>,
    /// 機軸に直交する方向の最低固有振動数の候補
    #[serde(default)]
    pub min_lateral: Vec<LaunchFitFrequencyLimit>,
    /// 機軸方向の最低固有振動数の候補
    #[serde(default)]
    pub min_axial: Vec<LaunchFitFrequencyLimit>,
    /// 簡易固有振動数の結果（梁の一次だけを使う）
    #[serde(default)]
    pub modes: Vec<LaunchFitMode>,
}

/// 質量の判定。
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct LaunchFitMass {
    /// 判定した質量 = 成長余裕込みの打ち上げ時の質量（部品の成長余裕込みの合計 + 推薬。質量バジェットの推薬込みの合計と同じ）
    pub launch_mass: Qty<Mass>,
    /// 成長余裕を掛けない質量（部品の質量 × 員数の合計 + 推薬）
    pub bare_mass: Qty<Mass>,
    /// 効いた上限
    pub limit: Qty<Mass>,
    /// 効いた上限の出所
    pub origin: String,
    /// 余裕 = 上限 / 成長余裕込みの質量 − 1
    pub margin: Option<Fraction>,
    /// 能力の曲線を引いた C3（曲線を使ったときだけ。上限がほかの候補のものでも、引いたなら入る）
    pub c3: Option<Qty<SpecificEnergy>>,
    /// 曲線の終わりの C3（遷移の C3 がこれより大きく、送れないときだけ。上限は 0 kg になり違反）
    pub beyond_curve_end: Option<Qty<SpecificEnergy>>,
    pub verdict: Verdict,
}

/// 外形の 1 軸の判定。
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct LaunchFitAxis {
    /// `x`・`y`・`z`
    pub axis: String,
    pub size: Qty<Length>,
    pub limit: Qty<Length>,
    /// 余裕 = 許容 / 外形 − 1
    pub margin: Option<Fraction>,
    pub verdict: Verdict,
}

/// 固有振動数の向き。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(rename_all = "snake_case")]
pub enum LaunchFitDirection {
    /// 機軸に直交する方向
    Lateral,
    /// 機軸方向
    Axial,
}

impl LaunchFitDirection {
    fn name(self) -> &'static str {
        match self {
            Self::Lateral => "lateral",
            Self::Axial => "axial",
        }
    }
}

/// 固有振動数の判定。
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct LaunchFitFrequency {
    pub direction: LaunchFitDirection,
    /// 効いた下限
    pub limit: Qty<Frequency>,
    /// 効いた下限の出所
    pub origin: String,
    /// 機体全体の一次の見積もり（無ければ `None`。判定は警告）
    pub estimated: Option<Qty<Frequency>>,
    /// 見積もりの要素の ID
    pub estimated_by: Option<String>,
    /// 余裕 = 見積もり / 下限 − 1
    pub margin: Option<Fraction>,
    pub verdict: Verdict,
}

/// 打ち上げ適合の出力。
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct LaunchFitOutput {
    pub mass: Option<LaunchFitMass>,
    /// 許容の外形の出所（外形を判定したとき）
    pub envelope_origin: Option<String>,
    /// x → y → z の順
    pub envelope: Vec<LaunchFitAxis>,
    /// 横 → 機軸の順（下限のあるものだけ）
    pub frequencies: Vec<LaunchFitFrequency>,
    /// 判定ごとの結果（key は `mass`・`envelope:<軸>`・`frequency:<向き>`）
    pub checks: Vec<Check>,
}

/// 打ち上げ適合。
pub struct LaunchFit;

/// 質量の上限の候補のうち効くもの（最小。厳しいほうが効く）。値が数でないものは数えない。
pub fn tightest_max_mass(limits: &[LaunchFitMassLimit]) -> Option<&LaunchFitMassLimit> {
    limits
        .iter()
        .filter(|l| l.value.value().is_finite())
        .min_by(|a, b| a.value.value().total_cmp(&b.value.value()))
}

/// 固有振動数の下限の候補のうち効くもの（最大。厳しいほうが効く）。値が数でないものは数えない。
pub fn tightest_min_frequency(
    limits: &[LaunchFitFrequencyLimit],
) -> Option<&LaunchFitFrequencyLimit> {
    limits
        .iter()
        .filter(|l| l.value.value().is_finite())
        .max_by(|a, b| a.value.value().total_cmp(&b.value.value()))
}

impl Analysis for LaunchFit {
    const ID: &'static str = "structure.launch_fit";
    const TITLE: &'static str = "打ち上げ適合";
    const AREA: AnalysisArea = AnalysisArea::System(System::Structure);
    const KIND: AnalysisKind = AnalysisKind::Steady;
    const DESCRIPTION: &'static str =
        "質量・収納外形・固有振動数が、打ち上げ機とディスペンサーの条件に収まるか";
    const TITLE_EN: &'static str = "Launch vehicle fit";
    const DESCRIPTION_EN: &'static str = "Whether mass, stowed envelope and natural frequencies fit the launcher and dispenser conditions";
    type Input = LaunchFitInput;
    type Output = LaunchFitOutput;

    fn run(input: &Self::Input) -> Result<Self::Output, AnalysisError> {
        let mass = judge_mass(input)?;
        let (envelope_origin, envelope) = judge_envelope(input)?;
        let frequencies = judge_frequencies(input)?;
        if mass.is_none() && envelope.is_empty() && frequencies.is_empty() {
            return Err(AnalysisError::InvalidInput(
                "No limit to compare against (the launcher, dispenser and requirements give no mass limit, envelope or minimum natural frequency)".to_owned(),
            ));
        }
        let mut checks = Vec::new();
        checks.extend(mass.as_ref().map(mass_check));
        if let Some(origin) = &envelope_origin {
            checks.extend(envelope.iter().map(|a| axis_check(a, origin)));
        }
        checks.extend(frequencies.iter().map(frequency_check));
        Ok(LaunchFitOutput {
            mass,
            envelope_origin,
            envelope,
            frequencies,
            checks,
        })
    }
}

fn positive(name: &str, v: f64) -> Result<(), AnalysisError> {
    if v.is_finite() && v > 0.0 {
        Ok(())
    } else {
        Err(AnalysisError::InvalidInput(format!(
            "{name} must be positive ({v})"
        )))
    }
}

fn judge_mass(input: &LaunchFitInput) -> Result<Option<LaunchFitMass>, AnalysisError> {
    // 質量の上限の候補は、基本の限度・要求と、能力の曲線を遷移の C3 で引いた質量（あれば）。効く 1 つはここだけで選ぶ
    let mut candidates = input.max_mass.clone();
    let mut beyond_end = None;
    if let Some(cap) = &input.capability {
        match capability::evaluate(cap)? {
            capability::Evaluated::Limit(l) => candidates.push(l),
            capability::Evaluated::BeyondEnd(end) => beyond_end = Some(end),
        }
    }
    // 曲線の終わりより大きい C3 は送れない: 上限 0 kg の違反（ほかの候補があっても、送れないことが先）
    let limit = match (&input.capability, beyond_end) {
        (Some(cap), Some(_)) => Some(LaunchFitMassLimit {
            origin: cap.origin.clone(),
            value: Qty::from_raw(0.0),
        }),
        _ => tightest_max_mass(&candidates).cloned(),
    };
    let Some(limit) = limit else {
        return Ok(None);
    };
    let propellant = input.propellant_mass.map_or(0.0, Qty::value);
    if beyond_end.is_none() {
        positive("Mass limit", limit.value.value())?;
    }
    if !(propellant.is_finite() && propellant >= 0.0) {
        return Err(AnalysisError::InvalidInput(format!(
            "Propellant mass must not be negative ({propellant})"
        )));
    }
    // 質量バジェットと同じ集計（推薬は成長余裕 0 の 1 行）
    let mut items = input.parts.clone();
    items.push(BudgetItem::counted("propellant", propellant, 1));
    let rollup = budget_rollup(&items, None, 0.0)?;
    positive("Dry mass", rollup.total - propellant)?;
    let launch = rollup.total_with_growth;
    let margin = ratio_margin(limit.value.value(), launch);
    Ok(Some(LaunchFitMass {
        launch_mass: Qty::from_raw(launch),
        bare_mass: Qty::from_raw(rollup.total),
        limit: limit.value,
        origin: limit.origin,
        margin,
        c3: input.capability.as_ref().map(|c| c.c3),
        beyond_curve_end: beyond_end,
        verdict: ratio_margin_verdict(margin, FIT_MARGIN_WARNING_BAND),
    }))
}

fn judge_envelope(
    input: &LaunchFitInput,
) -> Result<(Option<String>, Vec<LaunchFitAxis>), AnalysisError> {
    let (Some(envelope), Some(size)) = (&input.envelope, input.size) else {
        return Ok((None, Vec::new()));
    };
    let (sizes, limits) = (
        [size.x, size.y, size.z],
        [envelope.size.x, envelope.size.y, envelope.size.z],
    );
    let mut axes = Vec::new();
    for ((name, s), l) in AXES.iter().zip(sizes).zip(limits) {
        positive(&format!("Outer dimension {name}"), s)?;
        positive(&format!("Envelope {name}"), l)?;
        let margin = ratio_margin(l, s);
        axes.push(LaunchFitAxis {
            axis: (*name).to_owned(),
            size: Qty::from_raw(s),
            limit: Qty::from_raw(l),
            margin,
            verdict: ratio_margin_verdict(margin, FIT_MARGIN_WARNING_BAND),
        });
    }
    Ok((Some(envelope.origin.clone()), axes))
}

/// 機体全体の梁の一次の最小値（向きの梁の結果。無ければ `None`）。
fn whole_body(input: &LaunchFitInput, direction: LaunchFitDirection) -> Option<&LaunchFitMode> {
    let kind = match direction {
        LaunchFitDirection::Lateral => ModalEstimateKind::BeamLateral,
        LaunchFitDirection::Axial => ModalEstimateKind::BeamAxial,
    };
    input
        .modes
        .iter()
        .filter(|m| m.kind == kind && m.frequency.value().is_finite())
        .min_by(|a, b| a.frequency.value().total_cmp(&b.frequency.value()))
}

fn judge_frequencies(input: &LaunchFitInput) -> Result<Vec<LaunchFitFrequency>, AnalysisError> {
    let mut out = Vec::new();
    for (direction, limits) in [
        (LaunchFitDirection::Lateral, &input.min_lateral),
        (LaunchFitDirection::Axial, &input.min_axial),
    ] {
        let Some(limit) = tightest_min_frequency(limits) else {
            continue;
        };
        positive("Minimum natural frequency", limit.value.value())?;
        let estimate = whole_body(input, direction);
        let margin = estimate.and_then(|e| ratio_margin(e.frequency.value(), limit.value.value()));
        let verdict = match estimate {
            Some(_) => ratio_margin_verdict(margin, FIT_MARGIN_WARNING_BAND),
            // 見積もりが無いのは「確かめていない」。適合と読まれないよう警告にする
            None => Verdict::Warning,
        };
        out.push(LaunchFitFrequency {
            direction,
            limit: limit.value,
            origin: limit.origin.clone(),
            estimated: estimate.map(|e| e.frequency),
            estimated_by: estimate.map(|e| e.id.clone()),
            margin,
            verdict,
        });
    }
    Ok(out)
}

fn mass_check(m: &LaunchFitMass) -> Check {
    if let (Some(end), Some(c3)) = (m.beyond_curve_end, m.c3) {
        return Check::new(
            "mass",
            "Launch mass",
            m.verdict,
            format!(
                "Not capable: the departure needs C3 {}, above the last point ({}) of the capability curve of {} (not extrapolated); launch mass with growth allowance {} (bare {})",
                quantity_text::<SpecificEnergy>(c3.value()),
                quantity_text::<SpecificEnergy>(end.value()),
                m.origin,
                quantity_text::<Mass>(m.launch_mass.value()),
                quantity_text::<Mass>(m.bare_mass.value()),
            ),
        );
    }
    Check::new(
        "mass",
        "Launch mass",
        m.verdict,
        format!(
            "Launch mass with growth allowance {} (bare {}) (limit {} from {})",
            quantity_text::<Mass>(m.launch_mass.value()),
            quantity_text::<Mass>(m.bare_mass.value()),
            quantity_text::<Mass>(m.limit.value()),
            m.origin
        ),
    )
}

fn axis_check(a: &LaunchFitAxis, origin: &str) -> Check {
    Check::new(
        &format!("envelope:{}", a.axis),
        &format!("Envelope {}", a.axis),
        a.verdict,
        format!(
            "{} (allowed {} from {origin})",
            quantity_text::<Length>(a.size.value()),
            quantity_text::<Length>(a.limit.value())
        ),
    )
}

fn frequency_check(f: &LaunchFitFrequency) -> Check {
    let name = f.direction.name();
    let required = format!(
        "required at least {} from {}",
        quantity_text::<Frequency>(f.limit.value()),
        f.origin
    );
    let message = match f.estimated {
        Some(e) => format!("{} ({required})", quantity_text::<Frequency>(e.value())),
        None => format!(
            "Not verified: there is no whole-body estimate of the {name} first natural frequency (structure.modal_estimate has no beam result) ({required})"
        ),
    };
    Check::new(
        &format!("frequency:{name}"),
        &format!("First natural frequency ({name})"),
        f.verdict,
        message,
    )
}

#[cfg(test)]
mod tests;
