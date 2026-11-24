//! 打ち上げ機の能力の曲線（C3 → 送れる質量）の引き方。質量の上限の候補を 1 つ増やすだけで、
//! 候補から効く 1 つを選ぶ規則は親の [`super::tightest_max_mass`] のまま。
//!
//! - 曲線は C3 が増える順の点で、点の間は C3 について区分線形で結ぶ（手で点を引く表と同じ読み方）。
//! - 最後の点より大きい C3 は **外挿せず、送れない**（質量の上限は 0 で違反。判定の文に曲線の終わりの C3 を書く）。
//! - 最初の点より小さい C3 は最初の点の質量で見る（C3 が小さいほど送れる質量は増えるので、最初の点の質量は保守側）。

use super::LaunchFitMassLimit;
use crate::spec::{AnalysisError, quantity_text};
use sat_units::typed::{Mass, Qty, SpecificEnergy};
use serde::{Deserialize, Serialize};

/// 曲線の点 1 つ。
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct CapabilityPoint {
    pub c3: Qty<SpecificEnergy>,
    pub mass: Qty<Mass>,
}

/// 打ち上げ機が出発を受け持つ遷移の C3 と、その打ち上げ機の能力の曲線。
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct LaunchFitCapability {
    /// 曲線の出所（`launcher local/lv_alpha`）
    pub origin: String,
    /// 出発を打ち上げ機が受け持つマヌーバの ID
    pub maneuver: String,
    /// そのマヌーバの出発の C3
    pub c3: Qty<SpecificEnergy>,
    /// 曲線（C3 が増える順）
    pub curve: Vec<CapabilityPoint>,
}

/// 曲線を C3 で引いた結果。
pub(super) enum Evaluated {
    /// 質量の上限の候補（出所に C3 を書く）
    Limit(LaunchFitMassLimit),
    /// 曲線の終わりより大きい C3 のため送れない（曲線の終わりの C3）
    BeyondEnd(Qty<SpecificEnergy>),
}

/// 点の並びの検査（1 点以上・有限・C3 が増える順・質量が正）。
fn validate(curve: &[CapabilityPoint]) -> Result<(), AnalysisError> {
    let bad = |why: &str| {
        Err(AnalysisError::InvalidInput(format!(
            "Capability curve: {why}"
        )))
    };
    if curve.is_empty() {
        return bad("it has no point");
    }
    let finite = curve
        .iter()
        .all(|p| p.c3.value().is_finite() && p.mass.value().is_finite() && p.mass.value() > 0.0);
    if !finite {
        return bad("a point is not a finite number or its mass is not positive");
    }
    if curve.windows(2).any(|w| w[1].c3.value() <= w[0].c3.value()) {
        return bad("C3 must be strictly increasing");
    }
    Ok(())
}

/// 曲線の質量 [kg] を C3 [m²/s²] で引く（最後の点より大きければ `None`）。
pub fn mass_at_c3(curve: &[CapabilityPoint], c3: f64) -> Result<Option<f64>, AnalysisError> {
    validate(curve)?;
    let (first, last) = (&curve[0], &curve[curve.len() - 1]);
    if !c3.is_finite() {
        return Err(AnalysisError::InvalidInput(format!(
            "C3 must be a finite number ({c3})"
        )));
    }
    if c3 > last.c3.value() {
        return Ok(None);
    }
    if c3 <= first.c3.value() {
        return Ok(Some(first.mass.value()));
    }
    let hi = curve.partition_point(|p| p.c3.value() < c3);
    let (a, b) = (&curve[hi - 1], &curve[hi]);
    if c3 == b.c3.value() {
        return Ok(Some(b.mass.value()));
    }
    let t = (c3 - a.c3.value()) / (b.c3.value() - a.c3.value());
    Ok(Some(a.mass.value() + t * (b.mass.value() - a.mass.value())))
}

/// 能力の曲線を、遷移の C3 で質量の上限の候補にする。
pub(super) fn evaluate(cap: &LaunchFitCapability) -> Result<Evaluated, AnalysisError> {
    let c3 = cap.c3.value();
    Ok(match mass_at_c3(&cap.curve, c3)? {
        Some(kg) => Evaluated::Limit(LaunchFitMassLimit {
            origin: format!(
                "{} at C3 {}",
                cap.origin,
                quantity_text::<SpecificEnergy>(c3)
            ),
            value: Qty::from_raw(kg),
        }),
        None => Evaluated::BeyondEnd(cap.curve[cap.curve.len() - 1].c3),
    })
}

#[cfg(test)]
mod tests;
