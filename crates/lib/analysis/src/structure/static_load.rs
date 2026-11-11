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
    ensure_positive, ratio_margin, ratio_margin_verdict,
};
use sat_astro::constants::G0;
use sat_units::typed::{Dimensionless, Force, Fraction, Length, Mass, Qty, Torque};
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

/// 安全余裕 MS の警告の幅。0 なので、MS ≥ 0 なら ok、負なら違反の二段階（[`crate::spec::ratio_margin_verdict`]）。
pub const MS_WARNING_BAND: Fraction = Fraction::new(0.0);

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
    #[serde(serialize_with = "sat_units::typed::TorqueKind::serialize")]
    pub moment: Qty<Torque>,
    /// 安全余裕 MS = 許容荷重 / (安全率 × 荷重) − 1（[`crate::spec::ratio_margin`]。許容値が無いか荷重が 0 なら `None`）
    pub margin_axial: Option<Fraction>,
    pub margin_lateral: Option<Fraction>,
    pub margin_moment: Option<Fraction>,
    /// 3 つの安全余裕のうち最小（許容値が 1 つも無ければ `None`）
    pub min_margin: Option<Fraction>,
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
            safety_factor: input.safety_factor(),
            checks,
        })
    }
}

impl StaticLoadInput {
    /// 安全率（省略時 [`DEFAULT_SAFETY_FACTOR`]）。
    pub fn safety_factor(&self) -> f64 {
        self.safety_factor.unwrap_or(DEFAULT_SAFETY_FACTOR)
    }
}

fn validate(input: &StaticLoadInput) -> Result<(), AnalysisError> {
    if input.cases.is_empty() {
        return Err(AnalysisError::InvalidInput(
            "荷重のケースがありません".to_owned(),
        ));
    }
    ensure_positive("安全率", input.safety_factor())?;
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
            let sf = input.safety_factor();
            let ms = |allow: Option<f64>, load: f64| allow.and_then(|a| ratio_margin(a, sf * load));
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
                .reduce(|a, b| if b < a { b } else { a });
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
                verdict: ratio_margin_verdict(min_margin, MS_WARNING_BAND),
            }
        })
        .collect()
}

fn interface_check(input: &StaticLoadInput, m: &StaticLoadMountResult) -> Option<Check> {
    input.interfaces.iter().find(|i| i.mount == m.mount)?;
    let text = if let Some(worst) = m.min_margin {
        format!(
            "最小の安全余裕 MS = {}（安全率 {}）",
            quantity_text::<Fraction>(worst.value()),
            quantity_text::<Dimensionless>(input.safety_factor())
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
mod tests;
