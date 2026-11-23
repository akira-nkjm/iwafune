//! 入力の検査（値の範囲・ID の重複）。

use super::input::{ModalFemInput, ModalFemPanel};
use crate::spec::AnalysisError;
use std::collections::HashSet;

fn invalid(msg: String) -> AnalysisError {
    AnalysisError::InvalidInput(msg)
}

fn positive(x: f64) -> bool {
    x.is_finite() && x > 0.0
}

fn check_panel(p: &ModalFemPanel) -> Result<(), AnalysisError> {
    let ok = p.center.iter().all(|c| c.value().is_finite())
        && p.size.iter().all(|s| positive(s.value()))
        && positive(p.thickness.value())
        && positive(p.youngs_modulus.value())
        && (0.0..0.5).contains(&p.poisson_ratio)
        && (positive(p.density.value()) || p.mass.is_some_and(|m| positive(m.value())))
        && p.mass.is_none_or(|m| positive(m.value()));
    if ok {
        Ok(())
    } else {
        Err(invalid(format!(
            "{}: size, thickness, Young's modulus and mass or density must be positive (Poisson's ratio 0 to 0.5)",
            p.id
        )))
    }
}

/// 入力の検査。
pub fn check(input: &ModalFemInput) -> Result<(), AnalysisError> {
    if input.panels.is_empty() {
        return Err(invalid("No panels".to_owned()));
    }
    let mut ids = HashSet::new();
    for p in &input.panels {
        check_panel(p)?;
        if !ids.insert(p.id.as_str()) {
            return Err(invalid(format!("Duplicate panel id {}", p.id)));
        }
    }
    let mut lumped_ids = HashSet::new();
    for l in &input.lumped {
        let size_ok = l
            .footprint
            .is_none_or(|f| f.iter().all(|x| x.value().is_finite() && x.value() >= 0.0));
        if !(l.mass.value().is_finite() && l.mass.value() >= 0.0)
            || !l.position.iter().all(|x| x.value().is_finite())
            || !size_ok
        {
            return Err(invalid(format!(
                "{}: the mass and the footprint must not be negative",
                l.id
            )));
        }
        if !lumped_ids.insert(l.id.as_str()) {
            return Err(invalid(format!("Duplicate lumped mass id {}", l.id)));
        }
    }
    let settings_ok = input
        .distributed_mass
        .is_none_or(|m| m.value().is_finite() && m.value() >= 0.0)
        && input.mesh_size.is_none_or(|m| positive(m.value()))
        && input
            .merge_tolerance
            .is_none_or(|t| t.value().is_finite() && t.value() >= 0.0)
        && input
            .drilling_factor
            .is_none_or(|f| f.is_finite() && f >= 0.0);
    if settings_ok {
        Ok(())
    } else {
        Err(invalid(
            "The mesh size must be positive, and the tolerance, distributed mass and drilling factor must not be negative"
                .to_owned(),
        ))
    }
}
