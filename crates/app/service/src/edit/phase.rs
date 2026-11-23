//! `fune phase add|set|set-field|unset-field` と `fune case set-phase|unset-phase` に打った引数 → `CreatePhase`・`SetPhase`・
//! `SetPhaseField`・`SetCasePhase`（`POST /ops/from_command` の `phase_add`・`phase_set`・`phase_field`・`case_phase`）。
//! **フェーズの旗と、軌道・姿勢則の欄の値の読み方はここだけ**（逆の向き = 編集の欄の初期値は `query::phases::phase_text`）。

use super::text::{given, invalid, list_from_text, quantity_from_text};
use crate::dto::PhaseText;
use crate::error::Result;
use fune_model::design::{AttitudeLawSetting, OrbitSettings, Phase};
use fune_model::value::{SettingValue, setting_from_text};
use fune_ops::Op;

/// 打った引数 → `CreatePhase`（`create`）か `SetPhase`（軌道と姿勢則は書かない。`SetPhase` は今のものを残す）。
pub fn phase_op(t: &PhaseText, create: bool) -> Result<Op> {
    let phase = phase_of(t)?;
    Ok(if create {
        Op::CreatePhase { phase }
    } else {
        Op::SetPhase { phase }
    })
}

fn phase_of(t: &PhaseText) -> Result<Phase> {
    Ok(Phase {
        id: t.id.trim().into(),
        label: given(&t.label).map(str::to_owned),
        order: given(&t.order)
            .map(|s| {
                s.trim()
                    .parse::<i64>()
                    .map_err(|_| invalid(format!("Order must be an integer: {s}")))
            })
            .transpose()?
            .unwrap_or(0),
        start: given(&t.start).map(str::to_owned),
        end: given(&t.end).map(str::to_owned),
        duration: given(&t.duration).map(quantity_from_text).transpose()?,
        modes: given(&t.modes)
            .map(list_from_text)
            .unwrap_or_default()
            .into_iter()
            .map(Into::into)
            .collect(),
        orbit: None,
        law: Default::default(),
    })
}

/// `fune phase set-field <ID> <パス> <値>`（`value` が None なら `unset-field`）→ `SetPhaseField`。
/// 値の型は欄の見本（`OrbitSettings::field_shape`・`AttitudeLawSetting::field_shape`）か、文字列からの推し量り
/// （`setting_from_text`。整数 → 小数 → 真偽 → 量 → 文字列）で決める。
pub fn phase_field_op(phase: &str, path: &str, value: Option<&str>) -> Result<Op> {
    let value = value
        .map(|text| {
            let like = field_shape(path);
            setting_from_text(text, like.as_ref()).map_err(invalid)
        })
        .transpose()?;
    Ok(Op::SetPhaseField {
        phase: phase.trim().to_owned(),
        path: path.trim().to_owned(),
        value,
    })
}

/// パス（`orbit.<キー>`・`law.<モード>.<キー>`）の欄の値の形の見本。
fn field_shape(path: &str) -> Option<SettingValue> {
    let keys: Vec<&str> = path.trim().split('.').collect();
    match keys.as_slice() {
        ["orbit", key] => OrbitSettings::field_shape(key),
        ["law", _mode, key] => AttitudeLawSetting::field_shape(key),
        _ => None,
    }
}

/// `fune case set-phase <ケース> <フェーズ>`（`phase` が None なら `unset-phase`）→ `SetCasePhase`。
pub fn case_phase_op(case: &str, phase: Option<&str>) -> Op {
    Op::SetCasePhase {
        case: case.trim().to_owned(),
        phase: phase.map(|p| p.trim().to_owned()).filter(|p| !p.is_empty()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::query::phases::phase_text;
    use fune_model::value::Quantity;

    fn text() -> PhaseText {
        PhaseText {
            id: "LEOP".into(),
            label: Some("Commissioning".into()),
            order: Some("1".into()),
            start: Some("Separation".into()),
            end: Some(String::new()),
            duration: Some(" 30 d".into()),
            modes: Some("SAFE, NOMINAL".into()),
        }
    }

    #[test]
    fn typed_text_becomes_a_phase_and_back() {
        let Op::SetPhase { phase } = phase_op(&text(), false).unwrap() else {
            panic!("not SetPhase");
        };
        assert_eq!(phase.order, 1);
        assert_eq!(phase.duration, Some(Quantity::new(30.0, "d")));
        assert_eq!(phase.modes.len(), 2);
        // 空の文字列は書かない
        assert_eq!(phase.end, None);
        let back = phase_op(&phase_text(&phase), true).unwrap();
        assert_eq!(back, Op::CreatePhase { phase });
    }

    #[test]
    fn bad_text_is_rejected() {
        let mut t = text();
        t.order = Some("first".into());
        assert!(phase_op(&t, false).is_err());
        let mut t = text();
        t.duration = Some("thirty days".into());
        assert!(phase_op(&t, false).is_err());
    }

    #[test]
    fn field_values_follow_the_shape_of_the_field() {
        let value = |path: &str, text: &str| match phase_field_op("P", path, Some(text)).unwrap() {
            Op::SetPhaseField { value, .. } => value.unwrap(),
            op => panic!("{op:?}"),
        };
        assert_eq!(
            value("orbit.altitude", "550 km"),
            SettingValue::Quantity(Quantity::new(550.0, "km"))
        );
        // 数値に見える文字列でも、文字列の欄は文字列
        assert_eq!(value("orbit.label", "1"), SettingValue::Text("1".into()));
        assert!(matches!(
            value("law.NOMINAL.axis", "0 0 1"),
            SettingValue::Vector(_)
        ));
        assert!(phase_field_op("P", "orbit.altitude", Some("")).is_ok());
    }
}
