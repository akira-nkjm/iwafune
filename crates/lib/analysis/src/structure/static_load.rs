//! 準静的荷重（`structure.static_load`）。打上げの準静的加速度（荷重倍数）による、部品と取付部の荷重と安全余裕。
//!
//! - 部品の慣性荷重 = 質量 × g₀ × 荷重倍数（機軸方向と横方向を別に、合成は二乗和の平方根）。
//! - 取付は木（部品 → `mount` の部品）。取付部 M が受ける荷重は、M に付く部品とその先に付く部品すべての質量による。
//! - 横荷重による取付部のモーメントは、M に直接付く部品ごとに「その部品より先の質量 × 横加速度 × `cg_height`」の和
//!   （先に付く部品の重心も `cg_height` にあるとみなす）。
//! - 安全余裕 MS = 許容荷重 / (安全率 × 荷重) − 1。0 以上なら可。

use crate::spec::quantity_text;
use crate::spec::{
    Analysis, AnalysisError, AnalysisKind, Check, System, Verdict, ensure_non_negative,
    ensure_positive,
};
use sat_astro::constants::G0;
use sat_units::typed::{Force, Length, Mass, Qty, Torque};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

/// 質量を持つ部品と取付先。
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct StaticLoadItem {
    pub id: String,
    /// 1 台あたりの質量
    pub mass: Qty<Mass>,
    /// 員数（省略で 1、0 なら数えない）
    #[serde(default = "crate::spec::default_count")]
    pub count: u32,
    /// 取付先の部品の ID（ロケットへの取付は `"LV"` など、部品でない ID でよい）
    pub mount: String,
    /// 取付面から部品の重心までの高さ（与えると取付部のモーメントを求める）
    #[serde(default)]
    pub cg_height: Option<Qty<Length>>,
}

/// 荷重のケース（荷重倍数は g₀ の倍数。符号は問わない）。
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct StaticLoadCase {
    pub id: String,
    pub label: String,
    /// 機軸方向の荷重倍数 [g₀]
    pub axial: f64,
    /// 横方向の荷重倍数 [g₀]
    pub lateral: f64,
}

/// 取付部の許容荷重。
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct StaticLoadInterface {
    /// 取付先の ID（[`StaticLoadItem::mount`] と同じ）
    pub mount: String,
    #[serde(default)]
    pub allowable_axial: Option<Qty<Force>>,
    #[serde(default)]
    pub allowable_lateral: Option<Qty<Force>>,
    #[serde(default)]
    pub allowable_moment: Option<Qty<Torque>>,
}

/// 準静的荷重の入力。
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct StaticLoadInput {
    pub items: Vec<StaticLoadItem>,
    pub cases: Vec<StaticLoadCase>,
    #[serde(default)]
    pub interfaces: Vec<StaticLoadInterface>,
    /// 安全率（既定 [`DEFAULT_SAFETY_FACTOR`]）
    #[serde(default)]
    pub safety_factor: Option<f64>,
}

/// 安全率の既定値。
pub const DEFAULT_SAFETY_FACTOR: f64 = 1.25;

impl StaticLoadItem {
    /// 質量 × 員数 [kg]。
    pub fn total_mass(&self) -> f64 {
        self.mass.value() * f64::from(self.count)
    }
}

/// 部品の慣性荷重（全ケースの包絡）。
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct StaticLoadItemResult {
    pub id: String,
    pub mount: String,
    pub axial: Qty<Force>,
    pub lateral: Qty<Force>,
    pub resultant: Qty<Force>,
}

/// 取付部の荷重（全ケースの包絡）と安全余裕。
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct StaticLoadMountResult {
    pub mount: String,
    /// この取付部より先の質量の合計
    pub carried_mass: Qty<Mass>,
    pub axial: Qty<Force>,
    pub lateral: Qty<Force>,
    pub moment: Qty<Torque>,
    /// 安全余裕（許容値が無ければ `None`）
    pub margin_axial: Option<f64>,
    pub margin_lateral: Option<f64>,
    pub margin_moment: Option<f64>,
    /// 3 つの安全余裕のうち最小（許容値が 1 つも無ければ `None`）
    pub min_margin: Option<f64>,
    pub verdict: Verdict,
}

/// 準静的荷重の出力。
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct StaticLoadOutput {
    pub items: Vec<StaticLoadItemResult>,
    /// 取付先の ID の順
    pub mounts: Vec<StaticLoadMountResult>,
    /// 余裕の計算に使った安全率（入力が無ければ既定値）
    pub safety_factor: f64,
    /// 許容値のある取付部ごとの判定（key は `interface:<mount>`）
    pub checks: Vec<Check>,
}

/// 準静的荷重。
pub struct StaticLoad;

impl Analysis for StaticLoad {
    const ID: &'static str = "structure.static_load";
    const TITLE: &'static str = "準静的荷重";
    const SYSTEM: System = System::Structure;
    const KIND: AnalysisKind = AnalysisKind::Steady;
    const DESCRIPTION: &'static str = "打上げの準静的加速度による取付部の荷重と余裕";
    const TITLE_EN: &'static str = "Quasi-static loads";
    const DESCRIPTION_EN: &'static str =
        "Interface loads and margins under launch quasi-static accelerations";
    type Input = StaticLoadInput;
    type Output = StaticLoadOutput;

    fn run(input: &Self::Input) -> Result<Self::Output, AnalysisError> {
        validate(input)?;
        let carried = carried_masses(&input.items)?;
        let n_ax = input
            .cases
            .iter()
            .fold(0.0_f64, |m, c| m.max(c.axial.abs()));
        let n_lat = input
            .cases
            .iter()
            .fold(0.0_f64, |m, c| m.max(c.lateral.abs()));
        let n_res = input
            .cases
            .iter()
            .fold(0.0_f64, |m, c| m.max(c.axial.hypot(c.lateral)));
        let items = input
            .items
            .iter()
            .map(|it| {
                let w = it.total_mass() * G0;
                StaticLoadItemResult {
                    id: it.id.clone(),
                    mount: it.mount.clone(),
                    axial: Qty::from_raw(w * n_ax),
                    lateral: Qty::from_raw(w * n_lat),
                    resultant: Qty::from_raw(w * n_res),
                }
            })
            .collect();
        let mounts = mount_results(input, &carried, n_ax, n_lat);
        let checks = mounts
            .iter()
            .filter_map(|m| interface_check(input, m))
            .collect();
        Ok(StaticLoadOutput {
            items,
            mounts,
            safety_factor: safety_factor(input),
            checks,
        })
    }
}

fn safety_factor(input: &StaticLoadInput) -> f64 {
    input.safety_factor.unwrap_or(DEFAULT_SAFETY_FACTOR)
}

fn validate(input: &StaticLoadInput) -> Result<(), AnalysisError> {
    if input.cases.is_empty() {
        return Err(AnalysisError::InvalidInput(
            "荷重のケースがありません".to_owned(),
        ));
    }
    ensure_positive("安全率", safety_factor(input))?;
    for it in &input.items {
        ensure_non_negative(&format!("部品 {} の質量", it.id), it.mass.value())?;
    }
    Ok(())
}

/// 部品ごとの「自分とその先に付くすべての部品」の質量 [kg]。取付が輪になっていたらエラー。
fn carried_masses(items: &[StaticLoadItem]) -> Result<BTreeMap<&str, f64>, AnalysisError> {
    let mut carried: BTreeMap<&str, f64> = BTreeMap::new();
    for it in items {
        // 自分から根に向かって、通る部品すべてに自分の質量を足す。
        let mut node = it.id.as_str();
        let mut hops = 0;
        loop {
            *carried.entry(node).or_insert(0.0) += it.total_mass();
            let Some(parent) = items
                .iter()
                .find(|p| p.id == node)
                .map(|p| p.mount.as_str())
            else {
                break;
            };
            if !items.iter().any(|p| p.id == parent) {
                break;
            }
            node = parent;
            hops += 1;
            if hops > items.len() {
                return Err(AnalysisError::InvalidInput(format!(
                    "部品 {} の取付が輪になっています",
                    it.id
                )));
            }
        }
    }
    Ok(carried)
}

fn mount_results(
    input: &StaticLoadInput,
    carried: &BTreeMap<&str, f64>,
    n_ax: f64,
    n_lat: f64,
) -> Vec<StaticLoadMountResult> {
    let mut mounts: Vec<&str> = input.items.iter().map(|it| it.mount.as_str()).collect();
    mounts.sort_unstable();
    mounts.dedup();
    mounts
        .into_iter()
        .map(|m| {
            let children = input.items.iter().filter(|it| it.mount == m);
            let (mut mass, mut moment) = (0.0, 0.0);
            for c in children {
                let cm = carried.get(c.id.as_str()).copied().unwrap_or(0.0);
                mass += cm;
                moment += cm * G0 * n_lat * c.cg_height.map_or(0.0, Qty::value);
            }
            let (axial, lateral) = (mass * G0 * n_ax, mass * G0 * n_lat);
            let iface = input.interfaces.iter().find(|i| i.mount == m);
            let sf = safety_factor(input);
            let ms = |allow: Option<f64>, load: f64| allow.map(|a| a / (sf * load) - 1.0);
            let margin_axial = ms(iface.and_then(|i| i.allowable_axial).map(Qty::value), axial);
            let margin_lateral = ms(
                iface.and_then(|i| i.allowable_lateral).map(Qty::value),
                lateral,
            );
            let margin_moment = ms(
                iface.and_then(|i| i.allowable_moment).map(Qty::value),
                moment,
            );
            let min_margin = [margin_axial, margin_lateral, margin_moment]
                .into_iter()
                .flatten()
                .reduce(f64::min);
            StaticLoadMountResult {
                mount: m.to_owned(),
                carried_mass: Qty::from_raw(mass),
                axial: Qty::from_raw(axial),
                lateral: Qty::from_raw(lateral),
                moment: Qty::from_raw(moment),
                margin_axial,
                margin_lateral,
                margin_moment,
                min_margin,
                verdict: Verdict::from_ok(min_margin.is_none_or(|m| m >= 0.0)),
            }
        })
        .collect()
}

fn interface_check(input: &StaticLoadInput, m: &StaticLoadMountResult) -> Option<Check> {
    input.interfaces.iter().find(|i| i.mount == m.mount)?;
    let text = if let Some(worst) = m.min_margin {
        format!(
            "最小の安全余裕 MS = {}（安全率 {}）",
            quantity_text(worst, sat_units::typed::Fraction::KIND),
            quantity_text(safety_factor(input), "Dimensionless")
        )
    } else {
        "許容値がありません".to_owned()
    };
    Some(Check::new(
        &format!("interface:{}", m.mount),
        &format!("{} の取付部", m.mount),
        m.verdict,
        text,
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn item(id: &str, mass: f64, mount: &str, h: Option<f64>) -> StaticLoadItem {
        StaticLoadItem {
            id: id.to_owned(),
            mass: Qty::from_raw(mass),
            count: 1,
            mount: mount.to_owned(),
            cg_height: h.map(Qty::from_raw),
        }
    }

    /// RW 0.6 kg を PANEL（0.4 kg）に付け、PANEL を FRAME に付ける。機軸 7 g、横 3 g。
    /// PANEL の取付部: 0.6 kg × 9.80665 × 7 = 41.19 N。FRAME の取付部: 1.0 kg → 68.65 N、
    /// 横 29.42 N、モーメント 1.0·9.80665·3·0.05 = 1.471 N·m。
    #[test]
    fn hand_calculated_tree() {
        let input = StaticLoadInput {
            items: vec![
                item("RW", 0.6, "PANEL", Some(0.02)),
                item("PANEL", 0.4, "FRAME", Some(0.05)),
            ],
            cases: vec![
                StaticLoadCase {
                    id: "a".to_owned(),
                    label: "機軸".to_owned(),
                    axial: 7.0,
                    lateral: 1.0,
                },
                StaticLoadCase {
                    id: "l".to_owned(),
                    label: "横".to_owned(),
                    axial: 2.0,
                    lateral: 3.0,
                },
            ],
            interfaces: vec![StaticLoadInterface {
                mount: "FRAME".to_owned(),
                allowable_axial: Some(Qty::from_raw(200.0)),
                allowable_lateral: None,
                allowable_moment: Some(Qty::from_raw(1.0)),
            }],
            safety_factor: None,
        };
        let out = StaticLoad::run(&input).unwrap();
        let frame = out.mounts.iter().find(|m| m.mount == "FRAME").unwrap();
        let panel = out.mounts.iter().find(|m| m.mount == "PANEL").unwrap();
        assert!((panel.axial.value() - 0.6 * G0 * 7.0).abs() < 1e-9);
        assert!((frame.carried_mass.value() - 1.0).abs() < 1e-12);
        assert!((frame.axial.value() - G0 * 7.0).abs() < 1e-9);
        assert!((frame.lateral.value() - G0 * 3.0).abs() < 1e-9);
        assert!((frame.moment.value() - G0 * 3.0 * 0.05).abs() < 1e-9);
        let ms_ax = 200.0 / (1.25 * G0 * 7.0) - 1.0;
        assert!((frame.margin_axial.unwrap() - ms_ax).abs() < 1e-9);
        // モーメントは 1.471 N·m × 1.25 > 1 N·m で負の余裕
        assert!(frame.margin_moment.unwrap() < 0.0);
        assert_eq!(frame.min_margin, frame.margin_moment);
        assert_eq!(out.safety_factor, DEFAULT_SAFETY_FACTOR);
        assert_eq!(out.checks.len(), 1);
        assert_eq!(out.checks[0].verdict, Verdict::Violation);
    }

    #[test]
    fn mount_cycle_is_rejected() {
        let input = StaticLoadInput {
            items: vec![item("A", 1.0, "B", None), item("B", 1.0, "A", None)],
            cases: vec![StaticLoadCase {
                id: "a".to_owned(),
                label: String::new(),
                axial: 1.0,
                lateral: 1.0,
            }],
            interfaces: vec![],
            safety_factor: None,
        };
        assert!(StaticLoad::run(&input).is_err());
    }
}
