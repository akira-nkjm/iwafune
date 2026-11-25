//! `fune set <集まり>.<ID>.<欄> <値>`・`fune unset …`（`POST /ops/from_text`・`/ops/unset_from_text`）→ レコードの欄を 1 つだけ変える Op。
//! 集まり（cases・maneuvers・requirements・harness・sources・phases）・欄の名前・値の型は fune-model の `RecordKind` と
//! `RecordValue` が決める（ここは打った文字列をその型に読むだけで、読み方を別に持たない）。
//! ケースのフェーズと時刻は専用の Op（`case_phase_op`・`case_epoch_op`）にする（時刻の読み方の検査はそちらが持つ）。

use super::phase::{case_epoch_op, case_phase_op};
use super::text::invalid;
use crate::error::Result;
use crate::preview::snapshot_with;
use crate::workspace::Workspace;
use fune_model::design::{RecordKind, RecordValue, parse_record_field};
use fune_ops::Op;

/// 打ったパスがレコードの欄（`<集まり>.<ID>.<欄>`）か。**部品が優先**: 集まりの名前と同じ ID の部品（またはグループ）が
/// 設計にあれば、`harness.power.on` は今までどおりその部品の属性のパスで、レコードの欄ではない（None）。
/// 2 区切り（`harness.mass`）は集まりの名前でも部品のパス（`parse_record_field` が None）。
/// `ops` があれば、それを当てた設計で部品の有無を見る（同じ編集で作る部品を読むため）。
pub(super) fn record_path(
    ws: &Workspace,
    path: &str,
    ops: &[Op],
) -> Result<Option<(RecordKind, String, String)>> {
    let Some(parsed) = parse_record_field(path) else {
        return Ok(None);
    };
    let (kind, id, field) = parsed.map_err(invalid)?;
    let design = snapshot_with(ws, ops)?.design;
    Ok((!design.node_exists(kind.as_str())).then_some((kind, id, field)))
}

/// `kind` のレコード `id` の欄 `field` を `text` に書く Op（`text` が None か空なら外す）。
pub(super) fn record_field_op(
    kind: RecordKind,
    id: &str,
    field: &str,
    text: Option<&str>,
) -> Result<Op> {
    let ty = kind.field(field).map_err(invalid)?;
    let text = text.map(str::trim).filter(|t| !t.is_empty());
    if kind == RecordKind::Case {
        match field {
            "phase" => return Ok(case_phase_op(id, text)),
            "epoch" => return case_epoch_op(id, text),
            _ => {}
        }
    }
    let value = text
        .map(|t| RecordValue::from_text(field, ty, t))
        .transpose()
        .map_err(invalid)?;
    Ok(Op::SetRecordField {
        record: kind,
        id: id.trim().to_owned(),
        field: field.to_owned(),
        value,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use fune_model::value::Quantity;

    fn value(kind: RecordKind, field: &str, text: Option<&str>) -> Option<RecordValue> {
        match record_field_op(kind, "X", field, text) {
            Ok(Op::SetRecordField { value, .. }) => value,
            other => panic!("{other:?}"),
        }
    }

    #[test]
    fn each_field_reads_its_own_type() {
        let case = |f, t| value(RecordKind::Case, f, t);
        assert_eq!(
            case("beta", Some(" 60 deg ")),
            Some(RecordValue::Quantity(Quantity::new(60.0, "deg")))
        );
        assert_eq!(case("beta", None), None);
        assert_eq!(case("orbits", Some("15")), Some(RecordValue::Integer(15)));
        assert_eq!(
            case("side", Some("max")),
            Some(RecordValue::Text("max".into()))
        );
        assert_eq!(case("label", Some("  ")), None);
        assert_eq!(
            value(RecordKind::Requirement, "quantify.cases", Some("a, b")),
            Some(RecordValue::List(vec!["a".into(), "b".into()]))
        );
        assert_eq!(
            value(RecordKind::Maneuver, "constraints.duty", Some("0.5")),
            Some(RecordValue::Number(0.5))
        );
        assert_eq!(
            value(RecordKind::Phase, "order", Some("-2")),
            Some(RecordValue::Integer(-2))
        );
        // ケースのフェーズと時刻は専用の Op
        assert!(matches!(
            record_field_op(RecordKind::Case, "hot", "phase", Some("P")),
            Ok(Op::SetCasePhase { .. })
        ));
        assert!(matches!(
            record_field_op(
                RecordKind::Case,
                "hot",
                "epoch",
                Some("2031-05-02T00:00:00Z")
            ),
            Ok(Op::SetCaseEpoch { .. })
        ));
        // 別の集まりの `phase`・`epoch` ではない（マヌーバの `transfer.departure` など）
        for (kind, name, text) in [
            (RecordKind::Case, "orbits", "1.5"),
            (RecordKind::Case, "side", "high"),
            (RecordKind::Case, "beta", "sixty"),
            (RecordKind::Case, "epoch", "tuesday"),
            (RecordKind::Case, "colour", "red"),
            (RecordKind::Harness, "cores", "-1"),
            (RecordKind::Source, "kind", "rumour"),
            (RecordKind::Requirement, "quantify.all_cases", "maybe"),
        ] {
            assert!(
                record_field_op(kind, "X", name, Some(text)).is_err(),
                "{name}"
            );
        }
    }

    #[test]
    fn a_part_named_like_a_collection_wins() {
        let ws = Workspace::open(
            std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../../fixtures/demo-6u"),
        )
        .unwrap();
        // 部品が無ければレコードの欄、2 区切りは部品のパス
        let some = record_path(&ws, "cases.eclipse_worst.beta", &[]).unwrap();
        assert!(matches!(some, Some((RecordKind::Case, _, _))));
        assert!(record_path(&ws, "cases.beta", &[]).unwrap().is_none());
        assert!(record_path(&ws, "RW_X.mass", &[]).unwrap().is_none());
        assert!(record_path(&ws, "cases..beta", &[]).is_err());
        // 同じ編集で作る部品も数える（`ops`）
        let add = Op::CreatePart {
            id: "cases".into(),
            group: None,
            group_under: Vec::new(),
            catalog: Some("local/gps_l1".into()),
            kind: None,
            count: None,
            label: None,
            mode_state: None,
        };
        assert!(
            record_path(&ws, "cases.eclipse_worst.beta", &[add])
                .unwrap()
                .is_none()
        );
    }
}
