//! 人が打つ文字列（`fune set RW_X.mass "620 g"`）→ Op。属性定義・部品のフィールド・設定の現在の型を見て値の型を決める。
//! 文字列の読み方そのものは `fune_model::value::{value_from_text, parse_number_unit}` に任せる。
//! 入口は `edit::{set_op, unset_op}`（カタログ品は `super::catalog`、部品と設定はここ）。

use crate::error::{Result, ServiceError};
use crate::preview::snapshot_with;
use crate::workspace::Workspace;
use fune_model::design::{Maturity, PartField};
use fune_model::path::{ParamPath, SettingPath, parse};
use fune_model::value::{
    Provenance, Quantity, Quaternion, Scalar, SettingValue, Value, ValueType, Vector3,
    parse_number_unit, value_from_text,
};
use fune_ops::Op;

pub use crate::dto::{SetText, UnsetText};

/// `fune set` → `SetValue`・`SetField`・`SetSetting` のどれか（`t.path` は部品か設定のパス）。
pub(super) fn param_set_op(ws: &Workspace, t: &SetText) -> Result<Op> {
    match parse(&t.path).map_err(|e| ServiceError::InvalidInput(e.to_string()))? {
        ParamPath::Attr { part, attr, state } => {
            let value = attr_value(ws, &attr, state.is_some(), t)?;
            let from = (t.source.is_some() || t.note.is_some()).then(|| Provenance {
                source: t.source.clone(),
                note: t.note.clone(),
            });
            Ok(Op::SetValue {
                part,
                attr,
                state,
                value,
                from,
            })
        }
        ParamPath::Field { part, field } => Ok(Op::SetField {
            part,
            field: field_value(&field, Some(&t.value))?,
        }),
        ParamPath::Setting(path) => Ok(Op::SetSetting {
            value: Some(setting_value(ws, &path, t)?),
            path: path.to_string(),
        }),
    }
}

/// `fune unset` → `UnsetValue`・`SetField`（空）・`SetSetting`（消す）のどれか（`path` は部品か設定のパス）。
pub(super) fn param_unset_op(path: &str) -> Result<Op> {
    Ok(
        match parse(path).map_err(|e| ServiceError::InvalidInput(e.to_string()))? {
            ParamPath::Attr { part, attr, state } => Op::UnsetValue { part, attr, state },
            ParamPath::Field { part, field } => Op::SetField {
                part,
                field: field_value(&field, None)?,
            },
            ParamPath::Setting(p) => Op::SetSetting {
                path: p.to_string(),
                value: None,
            },
        },
    )
}

pub(super) fn invalid(e: impl std::fmt::Display) -> ServiceError {
    ServiceError::InvalidInput(e.to_string())
}

/// 属性の値（`cell` なら状態ごとの 1 セル）。型は属性定義から決める（`t.ops` があれば、それを当てた設計の定義）。
pub(super) fn attr_value(ws: &Workspace, attr: &str, cell: bool, t: &SetText) -> Result<Value> {
    let snapshot = snapshot_with(ws, &t.ops)?;
    let def = snapshot
        .design
        .attribute(attr)
        .ok_or_else(|| ServiceError::not_found("属性定義", attr))?;
    let value_type = if cell {
        ValueType::Scalar
    } else {
        def.value_type
    };
    if t.value.trim_start().starts_with('{') {
        // `to_cli` はテンソルなどを `fune set <path> --json '{…}'` の形で出す。値の JSON（`{ type, data }`）として読む。
        return serde_json::from_str(&t.value).map_err(invalid);
    }
    let value = value_from_text(&t.value, value_type).map_err(|e| ServiceError::Unit {
        input: t.value.clone(),
        reason: e.to_string(),
    })?;
    match value {
        Value::Scalar(s) => Ok(Value::Scalar(with_bounds(s, t)?)),
        other => Ok(other),
    }
}

/// `--min` / `--max` を nominal の単位に合わせて付ける（違う単位なら拒む）。
fn with_bounds(mut s: Scalar, t: &SetText) -> Result<Scalar> {
    let bound = |text: &Option<String>| -> Result<Option<f64>> {
        let Some(text) = text else { return Ok(None) };
        let (v, unit) = parse_number_unit(text).map_err(invalid)?;
        if !unit.is_empty() && unit != s.unit {
            return Err(invalid(format!(
                "--min / --max の単位は値と同じ {} にしてください",
                s.unit
            )));
        }
        Ok(Some(v))
    };
    s.min = bound(&t.min)?;
    s.max = bound(&t.max)?;
    Ok(s)
}

fn field_value(field: &str, text: Option<&str>) -> Result<PartField> {
    let owned = text.map(str::to_owned);
    Ok(match field {
        "label" => PartField::Label(owned),
        "description" => PartField::Description(owned),
        "catalog" => PartField::Catalog(owned),
        "kind" => PartField::Kind(owned),
        "count" => PartField::Count(match text {
            Some(t) => t
                .trim()
                .parse()
                .map_err(|_| invalid(format!("員数は整数で書きます: {t}")))?,
            None => 1,
        }),
        "maturity" => PartField::Maturity(text.map(maturity).transpose()?),
        "candidates" => PartField::Candidates(text.map(list_from_text).unwrap_or_default()),
        other => return Err(invalid(format!("部品のフィールドではありません: {other}"))),
    })
}

/// 成熟度の文字列（`off_the_shelf` / `modified` / `new`）。部品とカタログ品のフィールド、`fune catalog add --maturity` で共通。
pub fn maturity(text: &str) -> Result<Maturity> {
    text.parse().map_err(|_| {
        invalid(format!(
            "成熟度は off_the_shelf / modified / new です: {text}"
        ))
    })
}

/// 設定の値。今の値の型（整数・小数・量）に合わせ、無ければ文字列から推し量る。
fn setting_value(ws: &Workspace, path: &SettingPath, text: &SetText) -> Result<SettingValue> {
    let snapshot = snapshot_with(ws, &text.ops)?;
    let t = text.value.trim();
    // 今の値の型（`Design::setting`）に合わせる。書かれていなければ文字列から推し量る。
    Ok(match snapshot.design.setting(path) {
        Some(SettingValue::Number(_)) => SettingValue::Number(t.parse().map_err(invalid)?),
        Some(SettingValue::Integer(_)) => SettingValue::Integer(t.parse().map_err(invalid)?),
        Some(SettingValue::Bool(_)) => SettingValue::Bool(t.parse().map_err(invalid)?),
        Some(SettingValue::Quantity(_)) => SettingValue::Quantity(quantity_from_text(t)?),
        Some(SettingValue::Vector(_)) => SettingValue::Vector(vector_from_text(t)?),
        Some(SettingValue::List(_)) => SettingValue::List(list_from_text(t)),
        Some(SettingValue::Text(_)) => SettingValue::Text(t.to_owned()),
        None => guess_setting(t),
    })
}

fn guess_setting(t: &str) -> SettingValue {
    if let Ok(i) = t.parse::<i64>() {
        return SettingValue::Integer(i);
    }
    if let Ok(x) = t.parse::<f64>() {
        return SettingValue::Number(x);
    }
    if let Ok(b) = t.parse::<bool>() {
        return SettingValue::Bool(b);
    }
    match quantity_from_text(t) {
        Ok(q) => SettingValue::Quantity(q),
        Err(_) => SettingValue::Text(t.to_owned()),
    }
}

/// 文字列 → ベクトル（`"0 0 12 mm"`）。`fune part place --position` 用。
pub fn vector_from_text(text: &str) -> Result<Vector3> {
    match value_from_text(text, ValueType::Vec3).map_err(invalid)? {
        Value::Vec3(v) => Ok(v),
        _ => Err(invalid(format!("ベクトルとして読めません: {text}"))),
    }
}

/// 文字列 → クォータニオン（`"1 0 0 0"`、w x y z）。
pub fn quaternion_from_text(text: &str) -> Result<Quaternion> {
    match value_from_text(text, ValueType::Quat).map_err(invalid)? {
        Value::Quat(q) => Ok(q),
        _ => Err(invalid(format!("クォータニオンとして読めません: {text}"))),
    }
}

/// 文字列 → 量（`"12 V"`）。単位が読めなければエラー。
pub fn quantity_from_text(text: &str) -> Result<Quantity> {
    let (value, unit) = parse_number_unit(text).map_err(|e| ServiceError::Unit {
        input: text.to_owned(),
        reason: e.to_string(),
    })?;
    crate::units::quantity_from_authored(value, &unit, None)?;
    Ok(Quantity::new(value, unit))
}

/// カンマ区切りの一覧（`"off,on"`・`"mass, power"`）→ 語の列（空白を除き、空の語は捨てる）。**一覧の読み方はここだけ**（CLI も使う）。
pub fn list_from_text(text: &str) -> Vec<String> {
    text.split(',')
        .map(|s| s.trim().to_owned())
        .filter(|s| !s.is_empty())
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 部品のフィールドの打った文字列（GUI の基本の欄もこれを通る）。員数は整数、一覧はカンマ区切り。
    #[test]
    fn part_fields_read_typed_text() {
        assert_eq!(
            field_value("count", Some(" 3 ")).unwrap(),
            PartField::Count(3)
        );
        assert_eq!(field_value("count", None).unwrap(), PartField::Count(1));
        let e = field_value("count", Some("1.5")).unwrap_err().to_string();
        assert!(e.contains("員数は整数"), "{e}");
        assert_eq!(
            field_value("candidates", Some(" local/a, local/b ,")).unwrap(),
            PartField::Candidates(vec!["local/a".into(), "local/b".into()])
        );
        assert_eq!(
            field_value("candidates", None).unwrap(),
            PartField::Candidates(vec![])
        );
        assert!(field_value("maturity", Some("brand_new")).is_err());
    }
}
