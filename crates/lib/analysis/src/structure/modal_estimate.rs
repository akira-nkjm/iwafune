//! 簡易固有振動数（`structure.modal_estimate`）。FEM はせず、教科書の式で一次固有振動数を見積もって要求と比べる。
//!
//! 前提（概念設計の精度）:
//! - パネル: 四辺単純支持の等方性平板（Kirchhoff 理論）。搭載部品の質量は面内に一様に塗り広げる。
//!   f₁₁ = (π/2)(1/a² + 1/b²)·√(D/ρₛ)、D = E t³ / (12 (1 − ν²))、ρₛ = ρ t + m_ns / (a b)。
//! - 機体全体: 根元固定の一様な片持ち梁（Euler–Bernoulli、せん断変形と回転慣性は無視）に先端質量を付けたもの。
//!   横: 1 + cos β cosh β + (M/m) β (cos β sinh β − sin β cosh β) = 0 の最小根 β から f = β²/(2π)·√(EI/(m L³))。
//!   機軸（EA を与えたとき）: β tan β = m/M の最小根から f = β/(2π)·√(EA/(m L))。M = 0 なら β = π/2（f = ¼√(EA/(mL))）。

use crate::budget::{BudgetItem, counted_total};
use crate::spec::quantity_text;
use crate::spec::{
    Analysis, AnalysisArea, AnalysisError, AnalysisKind, Check, System, Verdict, ratio_margin,
    ratio_margin_verdict,
};
use sat_astro::math::bisect;
use sat_units::typed::{
    Area, Force, Fraction, Frequency, Length, Mass, MassDensity, Pressure, Qty,
};
use serde::{Deserialize, Serialize};
use std::f64::consts::{FRAC_PI_2, PI};

/// 曲げ剛性 EI の次元（N·m²）。
pub type FlexuralRigidity = <Force as std::ops::Mul<Area>>::Output;

/// 固有振動数の余裕の警告の幅。0 なので、要求以上なら ok、下回れば違反の二段階（[`crate::spec::ratio_margin_verdict`]）。
pub const FREQUENCY_MARGIN_WARNING_BAND: Fraction = Fraction::new(0.0);

/// 四辺単純支持のパネル。
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct ModalEstimatePanel {
    pub id: String,
    /// 辺の長さ a
    pub length_a: Qty<Length>,
    /// 辺の長さ b
    pub length_b: Qty<Length>,
    pub thickness: Qty<Length>,
    pub youngs_modulus: Qty<Pressure>,
    pub poisson_ratio: f64,
    pub density: Qty<MassDensity>,
    /// パネルに載る部品（質量 × 員数を面内に一様に塗り広げる）
    #[serde(default)]
    pub mounted: Vec<ModalEstimateMounted>,
}

/// パネルに載る部品。
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct ModalEstimateMounted {
    pub id: String,
    /// 1 台あたりの質量
    pub mass: Qty<Mass>,
    /// 員数（省略で 1、0 なら数えない）
    #[serde(default = "crate::spec::default_count")]
    pub count: u32,
}

/// 根元固定の片持ち梁（機体全体など）。
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct ModalEstimateBeam {
    pub id: String,
    pub length: Qty<Length>,
    /// 曲げ剛性 EI [N·m²]
    pub bending_stiffness: Qty<FlexuralRigidity>,
    /// 軸剛性 EA [N]（与えると機軸方向も求める）
    #[serde(default)]
    pub axial_stiffness: Option<Qty<Force>>,
    /// 梁に一様に分布する質量
    pub mass: Qty<Mass>,
    /// 先端の集中質量（既定 0）
    #[serde(default)]
    pub tip_mass: Option<Qty<Mass>>,
}

/// 要求する一次固有振動数の下限（無いものは判定しない）。
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct ModalEstimateRequirements {
    #[serde(default)]
    pub lateral: Option<Qty<Frequency>>,
    #[serde(default)]
    pub axial: Option<Qty<Frequency>>,
    #[serde(default)]
    pub panel: Option<Qty<Frequency>>,
}

/// 簡易固有振動数の入力。
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct ModalEstimateInput {
    #[serde(default)]
    pub panels: Vec<ModalEstimatePanel>,
    #[serde(default)]
    pub beams: Vec<ModalEstimateBeam>,
    #[serde(default)]
    pub requirements: ModalEstimateRequirements,
}

/// 見積もりの種類。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(rename_all = "snake_case")]
pub enum ModalEstimateKind {
    /// パネルの面外一次
    Panel,
    /// 梁の横（曲げ）一次
    BeamLateral,
    /// 梁の機軸（縦）一次
    BeamAxial,
}

/// 1 要素の一次固有振動数。
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct ModalEstimateResult {
    pub id: String,
    pub kind: ModalEstimateKind,
    pub frequency: Qty<Frequency>,
    pub required: Option<Qty<Frequency>>,
    /// 余裕 = f / f_req − 1（[`crate::spec::ratio_margin`]。要求が無いか 0 なら `None`）
    pub margin: Option<Fraction>,
    pub verdict: Verdict,
}

/// 簡易固有振動数の出力。
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct ModalEstimateOutput {
    /// パネル → 梁（横・機軸）の順
    pub results: Vec<ModalEstimateResult>,
    /// 要求のある要素ごとの判定（key は `<kind>:<id>`）
    pub checks: Vec<Check>,
}

/// 簡易固有振動数。
pub struct ModalEstimate;

impl Analysis for ModalEstimate {
    const ID: &'static str = "structure.modal_estimate";
    const TITLE: &'static str = "簡易固有振動数";
    const AREA: AnalysisArea = AnalysisArea::System(System::Structure);
    const KIND: AnalysisKind = AnalysisKind::Steady;
    const DESCRIPTION: &'static str = "簡易モデルでの一次固有振動数と要求との比較";
    const TITLE_EN: &'static str = "Natural frequency estimate";
    const DESCRIPTION_EN: &'static str =
        "First natural frequency from simplified models compared with the requirement";
    type Input = ModalEstimateInput;
    type Output = ModalEstimateOutput;

    fn run(input: &Self::Input) -> Result<Self::Output, AnalysisError> {
        let req = &input.requirements;
        let mut results = Vec::new();
        for p in &input.panels {
            results.push(result(
                &p.id,
                ModalEstimateKind::Panel,
                panel_frequency_hz(p)?,
                req.panel,
            ));
        }
        for b in &input.beams {
            let (lat, ax) = beam_frequencies_hz(b)?;
            results.push(result(
                &b.id,
                ModalEstimateKind::BeamLateral,
                lat,
                req.lateral,
            ));
            if let Some(ax) = ax {
                results.push(result(&b.id, ModalEstimateKind::BeamAxial, ax, req.axial));
            }
        }
        if results.is_empty() {
            return Err(AnalysisError::InvalidInput("No panels or beams".to_owned()));
        }
        let checks = results.iter().filter_map(check).collect();
        Ok(ModalEstimateOutput { results, checks })
    }
}

fn result(
    id: &str,
    kind: ModalEstimateKind,
    f_hz: f64,
    required: Option<Qty<Frequency>>,
) -> ModalEstimateResult {
    let margin = required.and_then(|r| ratio_margin(f_hz, r.value()));
    ModalEstimateResult {
        id: id.to_owned(),
        kind,
        frequency: Qty::from_raw(f_hz),
        required,
        margin,
        verdict: ratio_margin_verdict(margin, FREQUENCY_MARGIN_WARNING_BAND),
    }
}

fn check(r: &ModalEstimateResult) -> Option<Check> {
    let req = r.required?;
    let kind = match r.kind {
        ModalEstimateKind::Panel => (sat_checkkey::family::PANEL, "panel"),
        ModalEstimateKind::BeamLateral => (sat_checkkey::family::LATERAL, "lateral"),
        ModalEstimateKind::BeamAxial => (sat_checkkey::family::AXIAL, "axial"),
    };
    Some(
        Check::new(
            &sat_checkkey::modal_frequency(kind.0, &r.id),
            &format!("{} first natural frequency ({})", r.id, kind.1),
            r.verdict,
            format!(
                "{} (required at least {})",
                quantity_text::<Frequency>(r.frequency.value()),
                quantity_text::<Frequency>(req.value())
            ),
        )
        .with_ratio_margin(r.margin),
    )
}

fn invalid(id: &str) -> AnalysisError {
    AnalysisError::InvalidInput(format!(
        "{id}: dimensions, stiffness and mass must be positive (Poisson's ratio 0 to 0.5)"
    ))
}

/// パネルに載る部品の質量の合計 Σ 質量 × 員数 [kg]（`budget::counted_total` で数える）。
fn mounted_mass(mounted: &[ModalEstimateMounted]) -> Result<f64, AnalysisError> {
    let items: Vec<BudgetItem> = mounted
        .iter()
        .map(|m| BudgetItem::counted(m.id.clone(), m.mass.value(), m.count))
        .collect();
    counted_total(&items)
}

/// 四辺単純支持の平板の一次固有振動数 [Hz]。
pub fn panel_frequency_hz(p: &ModalEstimatePanel) -> Result<f64, AnalysisError> {
    let (a, b, t) = (p.length_a.value(), p.length_b.value(), p.thickness.value());
    let (e, nu, rho) = (p.youngs_modulus.value(), p.poisson_ratio, p.density.value());
    let m_ns = mounted_mass(&p.mounted)?;
    if !(a > 0.0
        && b > 0.0
        && t > 0.0
        && e > 0.0
        && rho > 0.0
        && m_ns >= 0.0
        && (0.0..0.5).contains(&nu))
    {
        return Err(invalid(&p.id));
    }
    let d = e * t.powi(3) / (12.0 * (1.0 - nu * nu));
    let rho_s = rho * t + m_ns / (a * b);
    Ok(FRAC_PI_2 * (1.0 / (a * a) + 1.0 / (b * b)) * (d / rho_s).sqrt())
}

/// 片持ち梁の (横, 機軸) の一次固有振動数 [Hz]。機軸は EA があるときだけ。
pub fn beam_frequencies_hz(b: &ModalEstimateBeam) -> Result<(f64, Option<f64>), AnalysisError> {
    let (l, ei, m) = (
        b.length.value(),
        b.bending_stiffness.value(),
        b.mass.value(),
    );
    let tip = b.tip_mass.map_or(0.0, Qty::value);
    let ea = b.axial_stiffness.map(Qty::value);
    if !(l > 0.0
        && ei > 0.0
        && m >= 0.0
        && tip >= 0.0
        && m + tip > 0.0
        && ea.is_none_or(|x| x > 0.0))
    {
        return Err(invalid(&b.id));
    }
    let two_pi = 2.0 * PI;
    let lateral = if m == 0.0 {
        (3.0 * ei / (l.powi(3) * tip)).sqrt() / two_pi
    } else {
        let f = |x: f64| {
            1.0 + x.cos() * x.cosh() + tip / m * x * (x.cos() * x.sinh() - x.sin() * x.cosh())
        };
        let beta = root(f, 1e-9, 1.9, &b.id)?;
        beta * beta / two_pi * (ei / (m * l.powi(3))).sqrt()
    };
    let axial = ea
        .map(|ea| -> Result<f64, AnalysisError> {
            Ok(if m == 0.0 {
                (ea / (l * tip)).sqrt() / two_pi
            } else if tip == 0.0 {
                0.25 * (ea / (m * l)).sqrt()
            } else {
                // β tan β = m/M を β sin β − (m/M) cos β = 0 として解く。
                let beta = root(|x| x * x.sin() - m / tip * x.cos(), 1e-9, FRAC_PI_2, &b.id)?;
                beta / two_pi * (ea / (m * l)).sqrt()
            })
        })
        .transpose()?;
    Ok((lateral, axial))
}

/// 振動数方程式の根（`sat_astro::math::bisect`）。見つからなければ入力の誤り。
fn root(f: impl Fn(f64) -> f64, lo: f64, hi: f64, id: &str) -> Result<f64, AnalysisError> {
    bisect(f, lo, hi, 1e-14, 200).ok_or_else(|| invalid(id))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn aluminium_panel(m_ns: Option<f64>) -> ModalEstimatePanel {
        ModalEstimatePanel {
            id: "PZ".to_owned(),
            length_a: Qty::from_raw(0.2),
            length_b: Qty::from_raw(0.2),
            thickness: Qty::from_raw(0.002),
            youngs_modulus: Qty::from_raw(70.0e9),
            poisson_ratio: 0.33,
            density: Qty::from_raw(2700.0),
            mounted: m_ns
                .map(|m| {
                    vec![ModalEstimateMounted {
                        id: "CAM".to_owned(),
                        mass: Qty::from_raw(m / 2.0),
                        count: 2,
                    }]
                })
                .unwrap_or_default(),
        }
    }

    /// 手計算: D = 70e9·8e-9 / (12·0.8911) = 52.37 N·m、ρₛ = 5.4 kg/m²、f = (π/2)(50)√(9.698) = 244.6 Hz。
    /// 部品の質量を ρₛ·a·b（0.216 kg）足すと 1/√2 倍。
    #[test]
    fn simply_supported_plate() {
        let f = panel_frequency_hz(&aluminium_panel(None)).unwrap();
        assert!((f - 244.6).abs() < 0.2, "{f}");
        let f2 = panel_frequency_hz(&aluminium_panel(Some(0.216))).unwrap();
        assert!((f2 - f / 2.0_f64.sqrt()).abs() < 1e-9);
    }

    fn beam(m: f64, tip: f64) -> ModalEstimateBeam {
        ModalEstimateBeam {
            id: "SC".to_owned(),
            length: Qty::from_raw(0.3),
            bending_stiffness: Qty::from_raw(1.0e5),
            axial_stiffness: Some(Qty::from_raw(1.0e8)),
            mass: Qty::from_raw(m),
            tip_mass: Some(Qty::from_raw(tip)),
        }
    }

    /// 先端質量なし: β = 1.87510、f = β²/(2π)·√(EI/(mL³))。機軸は ¼√(EA/(mL))。
    #[test]
    fn uniform_cantilever() {
        let (lat, ax) = beam_frequencies_hz(&beam(10.0, 0.0)).unwrap();
        let expected = 1.875_104_f64.powi(2) / (2.0 * PI) * (1.0e5 / (10.0 * 0.027_f64)).sqrt();
        assert!((lat / expected - 1.0).abs() < 1e-5, "{lat} vs {expected}");
        assert!((ax.unwrap() - 0.25 * (1.0e8 / 3.0_f64).sqrt()).abs() < 1e-9);
    }

    /// 梁の質量が先端質量に比べて小さければ、点質量の式 √(3EI/(L³M))/(2π) に近づく（Rayleigh: M + 33m/140）。
    #[test]
    fn tip_mass_limit() {
        let (lat, ax) = beam_frequencies_hz(&beam(1.0e-3, 10.0)).unwrap();
        let point =
            (3.0_f64 * 1.0e5 / (0.027 * (10.0 + 33.0 / 140.0 * 1.0e-3))).sqrt() / (2.0 * PI);
        assert!((lat / point - 1.0).abs() < 1e-5, "{lat} vs {point}");
        let axial_point = (1.0e8_f64 / (0.3 * (10.0 + 1.0e-3 / 3.0))).sqrt() / (2.0 * PI);
        assert!((ax.unwrap() / axial_point - 1.0).abs() < 1e-5);
        let (lat0, _) = beam_frequencies_hz(&beam(0.0, 10.0)).unwrap();
        assert!((lat0 / lat - 1.0).abs() < 1e-4);
    }

    #[test]
    fn requirement_check() {
        let out = ModalEstimate::run(&ModalEstimateInput {
            panels: vec![aluminium_panel(None)],
            beams: vec![],
            requirements: ModalEstimateRequirements {
                panel: Some(Qty::from_raw(300.0)),
                ..Default::default()
            },
        })
        .unwrap();
        assert_eq!(out.checks[0].verdict, Verdict::Violation);
        assert!(out.results[0].margin.unwrap().value() < 0.0);
    }
}
