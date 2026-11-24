//! `orbit.total_dose` の判定（部品ごとの余裕・評価できなかった部品・総線量から除いた区間）。

use super::{DoseLeg, DoseLegStatus, DoseNotEvaluated, DosePart};
use crate::spec::{Check, Verdict, quantity_text};
use sat_units::typed::{AbsorbedDoseKind, Dimensionless, Length};

/// 判定の一覧。部品ごとの `dose:<部品>`（余裕の判定、または評価しなかった警告）と、除いた区間の `phases_left_out`。
pub(super) fn checks(parts: &[DosePart], legs: &[DoseLeg], rdm: f64) -> Vec<Check> {
    let mut out = Vec::new();
    for p in parts {
        let key = sat_checkkey::total_dose(&p.part);
        let label = format!("Total dose of {}", p.part);
        match (p.not_evaluated, p.verdict, p.total, p.tolerance) {
            (Some(DoseNotEvaluated::NoShielding), ..) => out.push(Check::new(
                &key,
                &label,
                Verdict::Warning,
                "Not evaluated: the part has no shielding thickness and no default is set",
            )),
            (Some(DoseNotEvaluated::OutsideTable), ..) => out.push(Check::new(
                &key,
                &label,
                Verdict::Warning,
                format!(
                    "Not evaluated: the shielding of {} is outside the range of an environment table (no extrapolation)",
                    p.shielding.map_or_else(String::new, |s| quantity_text::<Length>(s.value()))
                ),
            )),
            (None, Some(v), Some(total), Some(tol)) => out.push(
                Check::new(
                    &key,
                    &label,
                    v,
                    format!(
                        "Total dose {} (x{} design margin = {}) against a tolerance of {}",
                        quantity_text::<AbsorbedDoseKind>(total.value()),
                        quantity_text::<Dimensionless>(rdm),
                        quantity_text::<AbsorbedDoseKind>(total.value() * rdm),
                        quantity_text::<AbsorbedDoseKind>(tol.value()),
                    ),
                )
                // 判定を導いた余裕（許容 / (総線量 × 設計余裕) − 1）。適合表が最も厳しい部品を選ぶのに使う
                .with_ratio_margin(p.margin),
            ),
            _ => {}
        }
    }
    let skipped: Vec<&str> = legs
        .iter()
        .filter(|l| l.status != DoseLegStatus::Evaluated)
        .map(|l| l.phase.as_deref().unwrap_or("(design orbit)"))
        .collect();
    if !skipped.is_empty() {
        out.push(Check::new(
            "phases_left_out",
            "Phases left out of the total",
            Verdict::Warning,
            format!(
                "The totals leave out phases with no duration or no environment table ({}), so they are lower bounds",
                skipped.join(", ")
            ),
        ));
    }
    out
}
