//! `fune set cases.<ID>.<欄>`・`fune unset cases.<ID>.<欄>`（`POST /ops/from_text`・`/ops/unset_from_text`）→ ケースの欄を 1 つだけ変える Op。
//! 欄の名前と値の型は fune-model の `CaseFieldName`・`CaseField` が決める（ここは打った文字列をその型に読むだけ）。
//! フェーズと時刻は専用の Op（`case_phase_op`・`case_epoch_op`）にする（時刻の読み方の検査はそちらが持つ）。

use super::phase::{case_epoch_op, case_phase_op};
use super::text::{invalid, quantity_from_text};
use crate::error::Result;
use fune_model::design::{CaseField, CaseFieldName, UncertaintySide};
use fune_ops::Op;

/// ケース `case` の欄 `field`（欄の名前）を `text` に書く Op（`text` が None なら外す）。
pub(super) fn case_field_op(case: &str, field: &str, text: Option<&str>) -> Result<Op> {
    let name: CaseFieldName = field.parse().map_err(invalid)?;
    let text = text.map(str::trim).filter(|t| !t.is_empty());
    let string = || text.map(str::to_owned);
    let typed = match name {
        CaseFieldName::Phase => return Ok(case_phase_op(case, text)),
        CaseFieldName::Epoch => return case_epoch_op(case, text),
        CaseFieldName::Label => CaseField::Label(string()),
        CaseFieldName::Description => CaseField::Description(string()),
        CaseFieldName::Mode => CaseField::Mode(string()),
        CaseFieldName::ThermalCase => CaseField::ThermalCase(string()),
        CaseFieldName::Side => CaseField::Side(
            text.map(|t| {
                t.parse::<UncertaintySide>()
                    .map_err(|_| invalid(format!("The side must be nominal / min / max: {t}")))
            })
            .transpose()?,
        ),
        CaseFieldName::Beta => CaseField::Beta(text.map(quantity_from_text).transpose()?),
        CaseFieldName::SunDistance => {
            CaseField::SunDistance(text.map(quantity_from_text).transpose()?)
        }
        CaseFieldName::Orbits => CaseField::Orbits(
            text.map(|t| {
                t.parse::<u32>()
                    .map_err(|_| invalid(format!("The number of orbits must be an integer: {t}")))
            })
            .transpose()?,
        ),
    };
    Ok(Op::SetCaseField {
        case: case.trim().to_owned(),
        field: typed,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use fune_model::value::Quantity;

    #[test]
    fn each_field_reads_its_own_type() {
        let field = |name: &str, text: Option<&str>| match case_field_op("hot", name, text) {
            Ok(Op::SetCaseField { field, .. }) => field,
            other => panic!("{other:?}"),
        };
        assert_eq!(
            field("beta", Some(" 60 deg ")),
            CaseField::Beta(Some(Quantity::new(60.0, "deg")))
        );
        assert_eq!(field("beta", None), CaseField::Beta(None));
        assert_eq!(field("orbits", Some("15")), CaseField::Orbits(Some(15)));
        assert_eq!(
            field("side", Some("max")),
            CaseField::Side(Some(UncertaintySide::Max))
        );
        assert_eq!(
            field("thermal_case", Some("cold")),
            CaseField::ThermalCase(Some("cold".into()))
        );
        assert_eq!(field("label", Some("  ")), CaseField::Label(None));
        // フェーズと時刻は専用の Op
        assert!(matches!(
            case_field_op("hot", "phase", Some("P")),
            Ok(Op::SetCasePhase { .. })
        ));
        assert!(matches!(
            case_field_op("hot", "epoch", Some("2031-05-02T00:00:00Z")),
            Ok(Op::SetCaseEpoch { .. })
        ));
        for (name, text) in [
            ("orbits", "1.5"),
            ("side", "high"),
            ("beta", "sixty"),
            ("epoch", "tuesday"),
            ("colour", "red"),
        ] {
            assert!(case_field_op("hot", name, Some(text)).is_err(), "{name}");
        }
    }
}
