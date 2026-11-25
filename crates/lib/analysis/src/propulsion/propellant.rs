//! 推薬の質量とタンク（`propulsion.propellant`）。
//!
//! - マヌーバの ΔV にマージンを掛けた設計値で、質量を減らしながら順に推薬を求める（`sequence`）。初期質量（ウェット）は
//!   最後に「乾燥質量 + 残る推薬」が残る質量。残る推薬は使う推薬 × 残る割合（既定 [`super::DEFAULT_RESIDUAL_FRACTION`]）。
//! - 推薬の名前（スラスタとタンクの `propellant`）ごとにまとめ、必要な容積 `m / ρ` をタンクの容積と比べる
//!   （比の余裕 `ratio_margin`、警告の幅 [`super::input::TANK_MARGIN_WARNING`]）。名前の無いスラスタ・タンクは、
//!   推薬が 1 種類だけならそれとみなす。
//! - RCS でのアンロードの推薬は含めない（`aocs.rw_unloading` が 1 日の推薬を出す）。

use super::input::{DvCategory, PropulsionInput, TANK_MARGIN_WARNING};
use super::sequence::{Options, Sequence, run};
use super::tank_load::{self, PropellantTankRow, key_of, only_name, thruster_used_kg};
use super::unloading;
use crate::spec::{
    Analysis, AnalysisArea, AnalysisError, AnalysisKind, Check, System, Verdict, margin_verdict,
    quantity_text, ratio_margin,
};
use sat_units::typed::{Fraction, Mass, MassDensity, Qty, Volume};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

/// 推薬 1 種類とそのタンク。
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct PropellantGroup {
    /// 推薬の名前（スラスタにもタンクにも名前が無ければ `None`）
    pub propellant: Option<String>,
    /// この推薬を使うスラスタ
    pub thrusters: Vec<String>,
    pub tanks: Vec<String>,
    /// 積む推薬（マージン込みで使う分 + 残る分）
    pub required: Qty<Mass>,
    pub density: Option<Qty<MassDensity>>,
    /// 積む推薬の容積（密度が無ければ `None`）
    pub required_volume: Option<Qty<Volume>>,
    pub tank_volume: Qty<Volume>,
    /// タンクの容積の余裕 = タンク / 必要 − 1（必要が 0 か判定できなければ `None`）
    pub volume_margin: Option<Fraction>,
    pub checks: Vec<Check>,
}

/// `propulsion.propellant` の出力。
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct PropellantOutput {
    pub dry_mass: Qty<Mass>,
    /// マージンを見込まない推薬（`propulsion.maneuvers` の合計）
    pub nominal_propellant: Qty<Mass>,
    /// マージン込みで使う推薬
    pub usable_propellant: Qty<Mass>,
    /// 残る推薬
    pub residual_propellant: Qty<Mass>,
    /// 積む推薬（使う分 + 残る分。RCS のスラスタがタンクを名指していれば、そのアンロードの推薬の積む分も含む）
    pub loaded_propellant: Qty<Mass>,
    /// RCS でのアンロードに積む推薬（使う分 + 残る分）のうち、RCS のスラスタが名指したタンクに入れた分
    /// （どのスラスタもタンクを名指さなければ 0。`usable_propellant`・`residual_propellant` はマヌーバの分だけ）
    pub unloading_propellant: Qty<Mass>,
    /// ウェット質量（乾燥質量 + 積む推薬。アンロードの推薬はマヌーバの列の質量には入れず、あとから足す）
    pub wet_mass: Qty<Mass>,
    pub dv_margin: Fraction,
    pub residual_fraction: Fraction,
    pub groups: Vec<PropellantGroup>,
    /// タンクごとの内訳（入力のタンクの順。合計は `groups` と同じ積む量）
    pub tanks: Vec<PropellantTankRow>,
    /// スラスタが無く推薬を求められないマヌーバ
    pub maneuvers_without_thruster: Vec<String>,
    pub checks: Vec<Check>,
}

/// 推薬の名前ごとに、使う推薬・スラスタ・タンクを集める。
fn groups(input: &PropulsionInput, seq: &Sequence<'_>, residual: f64) -> Vec<PropellantGroup> {
    let only = only_name(input);
    let mut by: BTreeMap<Option<String>, (Vec<String>, Vec<usize>, f64)> = BTreeMap::new();
    for t in &input.thrusters {
        let used = thruster_used_kg(seq, &t.id);
        let e = by.entry(key_of(t.propellant.as_ref(), only)).or_default();
        e.0.push(t.id.clone());
        e.2 += used * (1.0 + residual);
    }
    for (i, t) in input.tanks.iter().enumerate() {
        by.entry(key_of(t.propellant.as_ref(), only))
            .or_default()
            .1
            .push(i);
    }
    by.into_iter()
        .map(|(name, (thrusters, tanks, required))| group(input, name, thrusters, &tanks, required))
        .collect()
}

fn group(
    input: &PropulsionInput,
    propellant: Option<String>,
    thrusters: Vec<String>,
    tanks: &[usize],
    required: f64,
) -> PropellantGroup {
    let tanks: Vec<_> = tanks.iter().map(|&i| &input.tanks[i]).collect();
    let tank_volume: f64 = tanks.iter().map(|t| t.volume.value()).sum();
    let density = tanks.iter().find_map(|t| t.density);
    let required_volume = density.map(|rho| required / rho.value());
    let margin = required_volume.and_then(|v| ratio_margin(tank_volume, v));
    let name = propellant.as_deref().unwrap_or("propellant");
    let mut checks = Vec::new();
    if required > 0.0 && tanks.is_empty() {
        checks.push(Check::new(
            "tank",
            "Tank",
            Verdict::Violation,
            format!("No tank holds {name}"),
        ));
    } else if required > 0.0 && density.is_none() {
        checks.push(Check::new(
            "density",
            "Propellant density",
            Verdict::Warning,
            format!("No density for {name}, so tank volume is not checked"),
        ));
    } else if let (Some(v), false) = (required_volume, tanks.is_empty()) {
        let verdict = margin.map_or(Verdict::Ok, |m| {
            margin_verdict(m.value(), TANK_MARGIN_WARNING)
        });
        checks.push(
            Check::new(
                "tank_volume",
                "Tank volume",
                verdict,
                format!(
                    "{name}: needs {} / tank {}",
                    quantity_text::<Volume>(v),
                    quantity_text::<Volume>(tank_volume)
                ),
            )
            .with_ratio_margin(margin),
        );
    }
    PropellantGroup {
        propellant,
        thrusters,
        tanks: tanks.iter().map(|t| t.id.clone()).collect(),
        required: Qty::from_raw(required),
        density,
        required_volume: required_volume.map(Qty::from_raw),
        tank_volume: Qty::from_raw(tank_volume),
        volume_margin: margin,
        checks,
    }
}

/// 推薬の質量（ウェット・ドライ）とタンクの容積の余裕。
pub struct Propellant;

/// 区分ごとに、スラスタが使う推薬（[`Propellant::run_for_category`]）。
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct ThrusterPropellant {
    /// スラスタの ID
    pub thruster: String,
    /// そのスラスタのマヌーバが使う推薬（マージン込み）
    pub propellant: Qty<Mass>,
}

impl Propellant {
    /// [`Analysis::run`] と同じ出力に加えて、指定した区分のマヌーバがスラスタごとに使う推薬 [kg]
    /// （マージン込み。出力の `usable_propellant` と同じ設計値の列から取る。スラスタを名指すマヌーバだけ。
    /// 同じスラスタは 1 つにまとめ、並びは入力のスラスタの順）。
    pub(crate) fn run_for_category(
        input: &PropulsionInput,
        category: DvCategory,
    ) -> Result<(PropellantOutput, Vec<ThrusterPropellant>), AnalysisError> {
        evaluate(input, Some(category))
    }
}

impl Analysis for Propellant {
    const ID: &'static str = "propulsion.propellant";
    const TITLE: &'static str = "推薬とタンク";
    const TITLE_EN: &'static str = "Propellant and tanks";
    const AREA: AnalysisArea = AnalysisArea::System(System::Propulsion);
    const KIND: AnalysisKind = AnalysisKind::Steady;
    const DESCRIPTION: &'static str = "ロケット方程式をマヌーバの順に質量を減らしながら当て、マージン込みの推薬・ウェット質量とタンクの容積の余裕を出す";
    const DESCRIPTION_EN: &'static str = "Applies the rocket equation maneuver by maneuver with decreasing mass: propellant with margin, wet mass and tank volume margin";

    type Input = PropulsionInput;
    type Output = PropellantOutput;

    fn run(input: &Self::Input) -> Result<Self::Output, AnalysisError> {
        evaluate(input, None).map(|(out, _)| out)
    }
}

/// 区分 `category` のマヌーバがスラスタごとに使う推薬 [kg]（スラスタを名指すマヌーバだけ）。
fn used_by_thruster(
    input: &PropulsionInput,
    seq: &Sequence<'_>,
    category: DvCategory,
) -> Vec<ThrusterPropellant> {
    input
        .thrusters
        .iter()
        .filter_map(|t| {
            let legs: Vec<_> = seq
                .legs
                .iter()
                .filter(|l| {
                    l.maneuver.category == category && l.thruster.is_some_and(|x| x.id == t.id)
                })
                .collect();
            let kg = legs.iter().filter_map(|l| l.propellant_kg).sum();
            (!legs.is_empty()).then(|| ThrusterPropellant {
                thruster: t.id.clone(),
                propellant: Qty::from_raw(kg),
            })
        })
        .collect()
}

/// RCS が名指したタンクごとのアンロードの推薬 [kg] と、足せないときの警告。RCS のスラスタ（マヌーバのスラスタに数えない部品）が
/// タンクを名指していて、アンロードの入力が無ければ、その推薬はどのタンクにも入らないと警告する。
fn unloading_by_tank(
    input: &PropulsionInput,
) -> Result<(BTreeMap<String, f64>, Vec<Check>), AnalysisError> {
    if input.thruster_tanks.is_empty() {
        return Ok((BTreeMap::new(), Vec::new()));
    }
    if let Some(u) = &input.unloading {
        return Ok((
            unloading::propellant_by_tank(u, &input.thruster_tanks)?,
            Vec::new(),
        ));
    }
    let rcs: Vec<&str> = input
        .thruster_tanks
        .keys()
        .filter(|id| !input.thrusters.iter().any(|t| &&t.id == id))
        .map(String::as_str)
        .collect();
    let checks = (!rcs.is_empty())
        .then(|| {
            Check::new(
                "unloading_tank",
                "Unloading propellant tank",
                Verdict::Warning,
                format!(
                    "RCS thrusters ({}) name a tank, but the unloading propellant is not available, so it is not counted in the tank",
                    rcs.join(", ")
                ),
            )
        })
        .into_iter()
        .collect();
    Ok((BTreeMap::new(), checks))
}

/// 推薬の解析の本体。`category` があれば、その区分のマヌーバがスラスタごとに使う推薬も返す。
fn evaluate(
    input: &PropulsionInput,
    category: Option<DvCategory>,
) -> Result<(PropellantOutput, Vec<ThrusterPropellant>), AnalysisError> {
    let opts = Options::design(input)?;
    let nominal = super::maneuvers::nominal(input)?;
    let seq = run(input, opts)?;
    let usable = seq.used_propellant_kg();
    let residual = seq.residual_kg();
    let (unloading_kg, unloading_checks) = unloading_by_tank(input)?;
    let tanks = tank_load::rows(input, &seq, opts.residual_fraction, &unloading_kg)?;
    let unloading_loaded = tanks
        .iter()
        .fold(0.0, |a, t| a + t.unloading_propellant.value());
    let without: Vec<String> = seq
        .legs
        .iter()
        .filter(|l| l.thruster.is_none())
        .map(|l| l.maneuver.id.clone())
        .collect();
    let mut checks = unloading_checks;
    if !without.is_empty() {
        checks.push(Check::new(
            "thruster",
            "Maneuver thrusters",
            Verdict::Warning,
            format!(
                "Propellant is not computed for maneuvers without a thruster ({})",
                without.join(", ")
            ),
        ));
    }
    if tanks.iter().any(|t| t.named) {
        checks.extend(tanks.iter().filter(|t| t.thrusters.is_empty()).map(|t| {
            Check::new(
                "tank_unused",
                "Tank without thrusters",
                Verdict::Warning,
                format!(
                    "No thruster draws from tank {}, while other tanks are named by thrusters",
                    t.id
                ),
            )
        }));
    }
    let by_category = category.map_or_else(Vec::new, |c| used_by_thruster(input, &seq, c));
    let out = PropellantOutput {
        dry_mass: input.dry_mass,
        nominal_propellant: Qty::from_raw(nominal.used_propellant_kg()),
        usable_propellant: Qty::from_raw(usable),
        residual_propellant: Qty::from_raw(residual),
        loaded_propellant: Qty::from_raw(usable + residual + unloading_loaded),
        unloading_propellant: Qty::from_raw(unloading_loaded),
        wet_mass: Qty::from_raw(seq.initial_mass_kg + unloading_loaded),
        dv_margin: Fraction::new(opts.dv_scale - 1.0),
        residual_fraction: Fraction::new(opts.residual_fraction),
        groups: groups(input, &seq, opts.residual_fraction),
        tanks,
        maneuvers_without_thruster: without,
        checks,
    };
    Ok((out, by_category))
}
