//! 人が打つ文字列（`fune set RW_X.mass "620 g"`）→ Op。属性定義・部品のフィールド・設定の現在の型を見て値の型を決める。
//! 文字列の書式そのもの（読む向きと書く向き）は `fune_model::value`（`value_from_text`・`setting_from_text`・
//! `quantity_from_text`・`list_from_text` など）に任せ、ここは型を決めて誤りをサービスのエラーにする。
//! 入口は `edit::{set_op, unset_op}`（カタログ品は `super::catalog`、部品と設定はここ）。
//! 名詞のコマンドの引数（`super::command`・`super::defs`）も、ここの読み方（量・ベクトル・クォータニオン・一覧・`名前=値`・
//! X,Y・ネットの種類）を使う。

use crate::error::{Result, ServiceError};
use crate::preview::snapshot_with;
use crate::workspace::Workspace;
use fune_model::design::{Maturity, NetKind, PartField, PartFieldName};
use fune_model::path::{ParamPath, SettingPath, parse};
use fune_model::value::{
    Provenance, Quantity, Quaternion, Scalar, SettingValue, Value, ValueError, ValueType, Vector3,
    parse_number_unit, setting_from_text, value_from_text,
};
use fune_ops::Op;

pub use crate::dto::{SetText, UnsetText};
/// カンマ区切りの一覧の読み方（定義は `fune_model::value::list_from_text`。CLI と名詞のコマンドはここから呼ぶ）。
pub use fune_model::value::list_from_text;

impl SetText {
    /// 打った出典・位置・メモ → 値の出典（どれも無ければ付けない）。部品の値とカタログ品の値の両方がこれを通す。
    pub fn provenance(&self) -> Option<Provenance> {
        Provenance::from_parts(self.source.clone(), self.at.clone(), self.note.clone())
    }
}

/// `fune set` → `SetValue`・`SetField`・`SetSetting` のどれか（`t.path` は部品か設定のパス）。
pub(super) fn param_set_op(ws: &Workspace, t: &SetText) -> Result<Op> {
    match parse(&t.path).map_err(|e| ServiceError::InvalidInput(e.to_string()))? {
        ParamPath::Attr { part, attr, state } => {
            let value = attr_value(ws, &attr, AttrSlot::of(state.as_deref()), t)?;
            let from = t.provenance();
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

/// 打った値を書く先（属性の値そのものか、状態ごとの値の 1 セルか）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum AttrSlot {
    /// 属性の値そのもの（型は属性定義の `type`）
    Whole,
    /// 状態ごとの値の 1 セル（`RW_X.power.on`。スカラー）
    StateCell,
}

impl AttrSlot {
    /// パスの状態（あればそのセル）から。
    pub(super) fn of(state: Option<&str>) -> Self {
        match state {
            Some(_) => AttrSlot::StateCell,
            None => AttrSlot::Whole,
        }
    }
}

/// 属性の値（`slot` が状態のセルならスカラー）。型は属性定義から決める（`t.ops` があれば、それを当てた設計の定義）。
pub(super) fn attr_value(ws: &Workspace, attr: &str, slot: AttrSlot, t: &SetText) -> Result<Value> {
    let snapshot = snapshot_with(ws, &t.ops)?;
    let def = snapshot
        .design
        .attribute(attr)
        .ok_or_else(|| ServiceError::not_found("attribute definition", attr))?;
    let value_type = match slot {
        AttrSlot::StateCell => ValueType::Scalar,
        AttrSlot::Whole => def.value_type,
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
                "The unit of --min / --max must be the same as the value: {}",
                s.unit
            )));
        }
        Ok(Some(v))
    };
    s.min = bound(&t.min)?;
    s.max = bound(&t.max)?;
    Ok(s)
}

/// フィールド名（`PartFieldName`）と打った文字列 → 型付きのフィールド（`text` が None なら消す）。
fn field_value(field: &str, text: Option<&str>) -> Result<PartField> {
    let owned = text.map(str::to_owned);
    Ok(match field.parse::<PartFieldName>().map_err(invalid)? {
        PartFieldName::Label => PartField::Label(owned),
        PartFieldName::Description => PartField::Description(owned),
        PartFieldName::Catalog => PartField::Catalog(owned),
        PartFieldName::Kind => PartField::Kind(owned),
        PartFieldName::Count => PartField::Count(match text {
            Some(t) => t
                .trim()
                .parse()
                .map_err(|_| invalid(format!("Count must be an integer: {t}")))?,
            None => 1,
        }),
        PartFieldName::Redundancy => {
            PartField::Redundancy(text.unwrap_or("none").parse().map_err(invalid)?)
        }
        PartFieldName::Maturity => PartField::Maturity(text.map(maturity).transpose()?),
        PartFieldName::Candidates => {
            PartField::Candidates(text.map(list_from_text).unwrap_or_default())
        }
    })
}

/// 成熟度の文字列（`off_the_shelf` / `modified` / `new`）。部品とカタログ品のフィールド、`fune catalog add --maturity` で共通。
pub fn maturity(text: &str) -> Result<Maturity> {
    text.parse().map_err(|_| {
        invalid(format!(
            "Maturity must be off_the_shelf / modified / new: {text}"
        ))
    })
}

/// 設定の値。今の値（`Design::setting`）の型に合わせ、書かれていなければ文字列から推し量る（`setting_from_text`）。
fn setting_value(ws: &Workspace, path: &SettingPath, text: &SetText) -> Result<SettingValue> {
    let snapshot = snapshot_with(ws, &text.ops)?;
    let like = snapshot
        .design
        .setting(path)
        .or_else(|| attitude_field_type(path));
    setting_from_text(&text.value, like.as_ref()).map_err(|e| match like {
        Some(SettingValue::Quantity(_)) => quantity_error(text.value.trim(), e),
        _ => invalid(e),
    })
}

// 未設定でも、AOCS の姿勢則の軸と回転は文字列ではなく構造の値として読む（欄の形は model が持つ）。
fn attitude_field_type(path: &SettingPath) -> Option<SettingValue> {
    use fune_model::design::{AnalysisSystem, AttitudeLawSetting};
    use fune_model::path::SettingScope;
    let [system, table, _mode, field] = path.segments.as_slice() else {
        return None;
    };
    if path.scope != SettingScope::Analysis
        || system != AnalysisSystem::Aocs.as_str()
        || table != "law"
    {
        return None;
    }
    AttitudeLawSetting::field_shape(field)
}

/// 文字列 → ベクトル（`"0 0 12 mm"`）。`fune part place --position` 用。
pub fn vector_from_text(text: &str) -> Result<Vector3> {
    match value_from_text(text, ValueType::Vec3).map_err(invalid)? {
        Value::Vec3(v) => Ok(v),
        _ => Err(invalid(format!("Cannot read as a vector: {text}"))),
    }
}

/// 文字列 → クォータニオン（`"1 0 0 0"`、w x y z）。
pub fn quaternion_from_text(text: &str) -> Result<Quaternion> {
    match value_from_text(text, ValueType::Quat).map_err(invalid)? {
        Value::Quat(q) => Ok(q),
        _ => Err(invalid(format!("Cannot read as a quaternion: {text}"))),
    }
}

/// 文字列 → 量（`"12 V"`）。単位が読めなければエラー（読み方は `fune_model::value::quantity_from_text`）。
pub fn quantity_from_text(text: &str) -> Result<Quantity> {
    fune_model::value::quantity_from_text(text).map_err(|e| quantity_error(text, e))
}

/// 量として読めない誤り → `ServiceError::Unit`（単位が読めなければその単位、数が読めなければ打った文字列を添える）。
fn quantity_error(text: &str, e: ValueError) -> ServiceError {
    match e {
        ValueError::Unit { unit, reason } => ServiceError::Unit {
            input: unit,
            reason,
        },
        e => ServiceError::Unit {
            input: text.to_owned(),
            reason: e.to_string(),
        },
    }
}

/// `名前=値`（`"tx=0.1"`・`"PWR=power"`）→ (名前, 値)。前後の空白を除き、どちらかが空なら None。
/// **`名前=値` の読み方はここだけ**（`--duty` の割合・`--port` の雛形）。
pub fn pair_from_text(text: &str) -> Option<(String, String)> {
    let (name, value) = text.split_once('=')?;
    let (name, value) = (name.trim(), value.trim());
    (!name.is_empty() && !value.is_empty()).then(|| (name.to_owned(), value.to_owned()))
}

/// 図の上の位置・大きさ（`"420,120"`、整数の X,Y）。`fune view set --pos|--size`・`fune view edge-set --points` の 1 点。
pub fn xy_from_text(text: &str) -> Result<(i32, i32)> {
    let bad = || {
        invalid(format!(
            "Give a pair of integers X,Y (for example 420,120): {text}"
        ))
    };
    let (x, y) = text.split_once(',').ok_or_else(bad)?;
    Ok((
        x.trim().parse().map_err(|_| bad())?,
        y.trim().parse().map_err(|_| bad())?,
    ))
}

/// 空白区切りの点の列（`"-20,0 20,0 20,80"`）→ 点の列（`fune view edge-set --points`）。
pub fn points_from_text(text: &str) -> Result<Vec<[i32; 2]>> {
    text.split_whitespace()
        .map(|p| xy_from_text(p).map(|(x, y)| [x, y]))
        .collect()
}

/// ネットの種類（`power` / `data` / `rf`）。`fune net add|set --kind`・`fune kind add --port` で共通。
pub fn net_kind(text: &str) -> Result<NetKind> {
    text.trim()
        .parse()
        .map_err(|_| invalid(format!("Net kind must be power / data / rf: {text}")))
}

/// 書いた旗・欄の文字列（前後の空白を除く）。省略か空なら None（GUI の空の欄は CLI で旗を書かないのと同じ）。
pub(super) fn given(text: &Option<String>) -> Option<&str> {
    text.as_deref().map(str::trim).filter(|t| !t.is_empty())
}

/// 真偽の打った文字列（`true` / `false`。書かなければ None。マヌーバの `--sunlit-only`・要求の `--all-cases`）。
pub(super) fn bool_from_text(text: &Option<String>) -> Result<Option<bool>> {
    given(text)
        .map(|t| match t {
            "true" => Ok(true),
            "false" => Ok(false),
            other => Err(invalid(format!("must be true or false ({other})"))),
        })
        .transpose()
}

/// 時間割合（`"0.1"`。モードの duty・マヌーバのデューティ）。範囲と合計は設計の検査（`fune_model::check`）が見る。
pub(super) fn fraction_from_text(text: &str) -> Result<f64> {
    text.trim()
        .parse::<f64>()
        .ok()
        .filter(|f| f.is_finite())
        .ok_or_else(|| {
            invalid(format!(
                "Duty fraction must be a number from 0 to 1: {text}"
            ))
        })
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
        assert!(e.contains("Count must be an integer"), "{e}");
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

    /// 部品のフィールドの名前（`PartFieldName::NAMES`）は、どれも読めて同じ名前の型付きのフィールドになる（消すときも）。
    #[test]
    fn every_part_field_name_reads_back_to_the_same_name() {
        for &name in PartFieldName::ALL {
            let text = match name {
                PartFieldName::Count => "3",
                PartFieldName::Redundancy => "cold",
                PartFieldName::Maturity => "new",
                PartFieldName::Candidates => "local/a,local/b",
                _ => "text",
            };
            assert_eq!(field_value(name.as_str(), Some(text)).unwrap().key(), name);
            assert_eq!(field_value(name.as_str(), None).unwrap().key(), name);
        }
        assert!(field_value("voltage", Some("5 V")).is_err());
    }
}
