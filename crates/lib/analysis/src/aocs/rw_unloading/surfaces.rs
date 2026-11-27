//! 空力・太陽輻射圧の面（受圧面積と腕）の、出どころと時系列の中での範囲の報告。

use super::RwUnloadingSurface;
use crate::aocs::disturbance::{DisturbanceKind, DisturbanceSpacecraft, DisturbanceTorques};
use sat_units::typed::Qty;

/// 面を持つ種類（空力・太陽輻射圧）の報告。時系列で 1 度も評価しなかった面（蝕の中だけ・大気のモデルが無い天体）は出さない。
pub(super) fn summary(
    sc: &DisturbanceSpacecraft,
    torques: &[DisturbanceTorques],
) -> Vec<RwUnloadingSurface> {
    let kinds = [
        (
            DisturbanceKind::Aero,
            sc.aero.as_ref(),
            (|d: &DisturbanceTorques| d.aero_surface) as fn(&DisturbanceTorques) -> _,
        ),
        (DisturbanceKind::Srp, sc.srp.as_ref(), |d| d.srp_surface),
    ];
    kinds
        .into_iter()
        .filter_map(|(key, surface, pick)| {
            let surface = surface?;
            let samples: Vec<_> = torques.iter().filter_map(pick).collect();
            let first = samples.first()?;
            Some(RwUnloadingSurface {
                key,
                coefficient: surface.coefficient,
                area_source: first.area_source,
                cp_source: first.cp_source,
                area_min: Qty::from_raw(
                    samples.iter().map(|s| s.area).fold(f64::INFINITY, f64::min),
                ),
                area_max: Qty::from_raw(samples.iter().map(|s| s.area).fold(0.0, f64::max)),
                arm_max: Qty::from_raw(
                    samples
                        .iter()
                        .map(|s| s.cp_offset.norm())
                        .fold(0.0, f64::max),
                ),
            })
        })
        .collect()
}
