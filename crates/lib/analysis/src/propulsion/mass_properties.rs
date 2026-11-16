//! 推薬を積んだとき（寿命初期）と使ったあと（寿命末期）の質量と重心（`propulsion.mass_properties`）。
//!
//! - 乾燥の部品（タンクの部品の質量を含む）は `structure.mass_properties` と同じ項目。質量特性の計算は
//!   [`crate::structure::mass_properties`]（質量特性はそこだけ）に任せ、ここは推薬の点質量を足すだけ。
//! - 寿命初期の推薬は `propulsion.propellant` の推薬ごとの積む量（マージン込みで使う分 + 残る分）。同じ推薬の
//!   タンクには容積の比で分け、タンクの部品の重心に点質量として置く（液面・ブラダの形は扱わない）。
//! - 寿命末期は残る推薬だけ（積む量に「残る / 積む」の比を掛けて、同じタンクに置く）。
//! - タンクの部品が質量特性の項目に無ければ、乾燥の重心に置いて警告する。

use super::input::PropulsionInput;
use super::propellant::Propellant;
use crate::spec::{Analysis, AnalysisArea, AnalysisError, AnalysisKind, Check, System, Verdict};
use crate::structure::{
    MassPropertiesInput, MassPropertiesItem, MassPropertiesSummary, mass_properties,
};
use sat_astro::math::Vec3;
use sat_units::typed::{Length, Mass, Qty};
use serde::{Deserialize, Serialize};

/// 質量特性の入力（乾燥の部品）と推進系の入力。
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct PropellantMassInput {
    pub structure: MassPropertiesInput,
    pub propulsion: PropulsionInput,
}

/// タンク 1 つの推薬。
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct TankLoad {
    pub id: String,
    /// 寿命初期（積む推薬）
    pub bol: Qty<Mass>,
    /// 寿命末期（残る推薬）
    pub eol: Qty<Mass>,
    /// 推薬を置いた位置（タンクの部品の重心。機体座標系）[m]
    pub position: Vec3,
}

/// `propulsion.mass_properties` の出力。
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct PropellantMassOutput {
    /// 乾燥（推薬なし）
    pub dry: MassPropertiesSummary,
    /// 寿命初期（積む推薬を入れた）
    pub bol: MassPropertiesSummary,
    /// 寿命末期（残る推薬だけ）
    pub eol: MassPropertiesSummary,
    pub tanks: Vec<TankLoad>,
    /// 寿命初期から末期への重心の移動の大きさ
    pub com_shift: Qty<Length>,
    pub checks: Vec<Check>,
}

/// 寿命初期と末期の質量と重心。
pub struct PropellantMassProperties;

/// 推薬の点質量の項目（員数 1、外形なし）。
fn point(id: &str, mass: f64, position: Vec3) -> MassPropertiesItem {
    MassPropertiesItem {
        id: format!("{id}:propellant"),
        mass: Qty::from_raw(mass),
        count: 1,
        position,
        size: None,
        orientation: None,
        inertia: None,
    }
}

/// 推薬ごとの積む量を、同じ推薬のタンクに容積の比で分ける（タンクの ID → 積む量 [kg]）。
fn split(
    input: &PropulsionInput,
    groups: &[super::propellant::PropellantGroup],
) -> Vec<(String, f64)> {
    let mut out = Vec::new();
    for g in groups {
        let total: f64 = g.tank_volume.value();
        for id in &g.tanks {
            let v = input
                .tanks
                .iter()
                .find(|t| &t.id == id)
                .map_or(0.0, |t| t.volume.value());
            let share = if total > 0.0 { v / total } else { 0.0 };
            out.push((id.clone(), g.required.value() * share));
        }
    }
    out
}

impl Analysis for PropellantMassProperties {
    const ID: &'static str = "propulsion.mass_properties";
    const USES: &'static [&'static str] = &[crate::propulsion::Propellant::ID];
    const TITLE: &'static str = "推薬による質量特性の変化";
    const TITLE_EN: &'static str = "Mass properties with propellant";
    const AREA: AnalysisArea = AnalysisArea::System(System::Propulsion);
    const KIND: AnalysisKind = AnalysisKind::Steady;
    const DESCRIPTION: &'static str = "推薬を積んだとき（寿命初期）と使ったあと（寿命末期）の質量・重心・慣性（推薬はタンクの重心の点質量）";
    const DESCRIPTION_EN: &'static str = "Mass, center of mass and inertia with propellant loaded (beginning of life) and used (end of life); propellant as point masses at the tanks";

    type Input = PropellantMassInput;
    type Output = PropellantMassOutput;

    fn run(input: &Self::Input) -> Result<Self::Output, AnalysisError> {
        let items = &input.structure.items;
        let dry = mass_properties(items)?;
        let prop = Propellant::run(&input.propulsion)?;
        let loaded = prop.loaded_propellant.value();
        let eol_ratio = if loaded > 0.0 {
            prop.residual_propellant.value() / loaded
        } else {
            0.0
        };
        let mut checks = Vec::new();
        let tanks: Vec<TankLoad> = split(&input.propulsion, &prop.groups)
            .into_iter()
            .map(|(id, bol)| {
                let position = items.iter().find(|it| it.id == id).map_or_else(
                    || {
                        checks.push(Check::new(
                            "tank_position",
                            "Tank position",
                            Verdict::Warning,
                            format!("Tank {id} part is not in the mass properties, so the propellant was placed at the dry centre of mass"),
                        ));
                        dry.center_of_mass
                    },
                    |it| it.position,
                );
                TankLoad { bol: Qty::from_raw(bol), eol: Qty::from_raw(bol * eol_ratio), position, id }
            })
            .collect();
        let with = |pick: fn(&TankLoad) -> f64| -> Result<MassPropertiesSummary, AnalysisError> {
            let mut all = items.clone();
            all.extend(
                tanks
                    .iter()
                    .filter(|t| pick(t) > 0.0)
                    .map(|t| point(&t.id, pick(t), t.position)),
            );
            mass_properties(&all)
        };
        let bol = with(|t| t.bol.value())?;
        let eol = with(|t| t.eol.value())?;
        let com_shift = Qty::from_raw((bol.center_of_mass - eol.center_of_mass).norm());
        Ok(PropellantMassOutput {
            dry,
            bol,
            eol,
            tanks,
            com_shift,
            checks,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::propulsion::tests::{input, raise};

    fn item(id: &str, mass: f64, z: f64) -> MassPropertiesItem {
        MassPropertiesItem {
            id: id.to_owned(),
            mass: Qty::from_raw(mass),
            count: 1,
            position: Vec3::new(0.0, 0.0, z),
            size: None,
            orientation: None,
            inertia: None,
        }
    }

    /// 乾燥 9 kg が原点・タンク 1 kg が z = 0.1 m、推薬 p をタンクに置く: 重心 z = 0.1 (1 + p) / (10 + p)（手計算）。
    #[test]
    fn bol_and_eol_center_of_mass_by_hand() {
        let structure = MassPropertiesInput {
            items: vec![item("BUS", 9.0, 0.0), item("TANK", 1.0, 0.1)],
            max_mass: None,
            com_box: None,
        };
        let propulsion = input(vec![raise()]);
        let out = PropellantMassProperties::run(&PropellantMassInput {
            structure,
            propulsion,
        })
        .unwrap();
        let p = out.tanks[0].bol.value();
        assert!(p > 0.0);
        assert!((out.bol.total_mass.value() - (10.0 + p)).abs() < 1e-12);
        assert!((out.bol.center_of_mass.z - 0.1 * (1.0 + p) / (10.0 + p)).abs() < 1e-12);
        // 残る推薬は積む量の 3 % / 103 %（既定の残る割合）
        let r = out.tanks[0].eol.value();
        assert!((r / p - 0.03 / 1.03).abs() < 1e-9, "{}", r / p);
        assert!((out.eol.center_of_mass.z - 0.1 * (1.0 + r) / (10.0 + r)).abs() < 1e-12);
        assert!(
            (out.com_shift.value() - (out.bol.center_of_mass.z - out.eol.center_of_mass.z)).abs()
                < 1e-12
        );
        assert!(out.checks.is_empty());
    }
}
