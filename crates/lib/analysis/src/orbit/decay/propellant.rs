//! 軌道維持に使う推薬と、それを使い切ったときの ΔV。
//!
//! 推薬の量は推進系の解析（`propulsion.propellant`）だけが決める。ここはその `usable_propellant`（マージン込みで使う推薬）を
//! 読むだけで、推薬を別に持たない。比推力は推進系のスラスタのうち最も低いもの（保守側）、噴射前の質量は推薬を積んだ質量
//! （`wet_mass`）。換算は `sat_astro::rocket`。

use crate::propulsion::{Propellant, PropulsionInput};
use crate::spec::{Analysis, AnalysisError, ensure_positive};
use sat_astro::rocket::{delta_v, exhaust_velocity};
use sat_units::typed::{Mass, Qty, Velocity};

/// 軌道維持に使う推進系の値（`DecayUsed` に残す）。
pub(super) struct Capacity {
    /// 比推力を取ったスラスタ
    pub thruster: String,
    /// 使える推薬
    pub propellant: Qty<Mass>,
    /// 使える推薬をすべて使ったときの ΔV
    pub delta_v: Qty<Velocity>,
}

/// 推進系の入力から、軌道維持に使える推薬と ΔV を求める（スラスタが無ければ `None`）。
pub(super) fn capacity(p: &PropulsionInput) -> Result<Option<Capacity>, AnalysisError> {
    let Some(thruster) = p
        .thrusters
        .iter()
        .min_by(|a, b| a.isp.value().total_cmp(&b.isp.value()))
    else {
        return Ok(None);
    };
    ensure_positive("比推力", thruster.isp.value())?;
    let out = Propellant::run(p)?;
    let usable = out.usable_propellant.value();
    let dv = delta_v(
        exhaust_velocity(thruster.isp.value()),
        out.wet_mass.value(),
        usable,
    )
    .ok_or_else(|| {
        AnalysisError::InvalidInput("推進系の推薬と質量から ΔV を求められません".into())
    })?;
    Ok(Some(Capacity {
        thruster: thruster.id.clone(),
        propellant: Qty::from_raw(usable),
        delta_v: Qty::from_raw(dv),
    }))
}
