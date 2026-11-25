//! 名前のついたレコードの欄 1 つ（`fune set <集まり>.<ID>.<欄> <値>`・`SetRecordField` の Op）。解析ケース・マヌーバ・要求・
//! ハーネス・出典・フェーズの見出しの欄を、レコードを丸ごと置き換えずに 1 つだけ書く（書かなかった欄は今のまま残る）。
//!
//! **集まりの名前（[`RecordKind`]）・欄の名前と値の型（[`RecordKind::fields`]）・打った文字列と値の往復（[`RecordValue`]）・
//! 欄を書き換える処理（[`patched`]）はここだけ**。値の型は欄の宣言（`FieldType`）で決め、書いた結果が型として正しいかは
//! レコードの serde が最後に確かめる（列挙の綴り・必須の欄・未知の欄はそこで拒まれる）。
//! 入れ子の表の欄は `constraint.value` のようにドットでつなぐ（表の中身が空になったら表ごと外す）。

use super::conditions::CentralBodyKind;
use super::defs::BudgetKind;
use super::maneuver::{DepartureBy, ManeuverCategory, ManeuverMethod, ManeuverSteering};
use super::requirement::{ConstraintKind, RequirementInput};
use super::source::{Distribution, SourceKind};
use super::system::UncertaintySide;
use crate::enum_text::enum_text;
use crate::path::PathError;
use crate::value::{
    Quantity, ValueError, list_from_text, list_to_text, quantity_from_text, quantity_to_text,
};
use serde::{Deserialize, Serialize, de::DeserializeOwned};
use serde_json::Value as Json;
use ts_rs::TS;

enum_text! {
    /// 欄を 1 つずつ書けるレコードの集まり（パスの先頭の区切り。ファイルを置くディレクトリの名前と同じ）。
    #[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize, TS)]
    pub enum RecordKind: "record collection" {
        Case = "cases",
        Maneuver = "maneuvers",
        Requirement = "requirements",
        Harness = "harness",
        Source = "sources",
        Phase = "phases",
    }
}

/// 欄の値の型（打った文字列をどう読むか）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FieldType {
    /// 文字列（ID・自由文）
    Text,
    /// 決まった綴りの 1 つ（列挙）
    Choice(&'static [&'static str]),
    /// 整数
    Integer,
    /// 0 以上の整数
    Unsigned,
    /// 小数
    Number,
    /// 真偽（`true`・`false`）
    Bool,
    /// 量（`"60 deg"`）
    Quantity,
    /// カンマ区切りの一覧
    List,
}

/// 欄の宣言（名前と型）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FieldSpec {
    /// 欄の名前（ファイルのキー。入れ子の表の中は `constraint.value`）
    pub name: &'static str,
    pub ty: FieldType,
}

const fn f(name: &'static str, ty: FieldType) -> FieldSpec {
    FieldSpec { name, ty }
}

use FieldType::{Bool, Choice, Integer, List, Number, Quantity as Q, Text, Unsigned};

/// 解析ケースの欄（`Case` の宣言の順）。
const CASE_FIELDS: &[FieldSpec] = &[
    f("label", Text),
    f("description", Text),
    f("mode", Text),
    f("phase", Text),
    f("side", Choice(UncertaintySide::NAMES)),
    f("thermal_case", Text),
    f("beta", Q),
    f("orbits", Unsigned),
    f("sun_distance", Q),
    f("epoch", Text),
];

/// マヌーバの欄（`[target]`・`[constraints]`・`[transfer]` の中は表の名前を付ける。`[[transfer.flybys]]` は含めない）。
const MANEUVER_FIELDS: &[FieldSpec] = &[
    f("label", Text),
    f("category", Choice(ManeuverCategory::NAMES)),
    f("method", Choice(ManeuverMethod::NAMES)),
    f("thruster", Text),
    f("after", Text),
    f("via_altitude", Q),
    f("delta_v", Q),
    f("target.body", Choice(CentralBodyKind::NAMES)),
    f("target.altitude", Q),
    f("target.perigee_altitude", Q),
    f("target.apogee_altitude", Q),
    f("target.inclination", Q),
    f("constraints.sunlit_only", Bool),
    f("constraints.mode", Text),
    f("constraints.duty", Number),
    f("constraints.steering", Choice(ManeuverSteering::NAMES)),
    f("transfer.departure", Text),
    f("transfer.arrival", Text),
    f("transfer.departure_by", Choice(DepartureBy::NAMES)),
];

/// 要求の欄。
const REQUIREMENT_FIELDS: &[FieldSpec] = &[
    f("label", Text),
    f("statement", Text),
    f("statement_en", Text),
    f("quantity", Text),
    f("source", Text),
    f("input", Choice(RequirementInput::NAMES)),
    f("constraint.kind", Choice(ConstraintKind::NAMES)),
    f("constraint.value", Q),
    f("constraint.tolerance", Q),
    f("constraint.min", Q),
    f("constraint.max", Q),
    f("quantify.all_cases", Bool),
    f("quantify.cases", List),
    f("verify.analysis", Text),
    f("verify.check", Text),
    f("verify.budget", Choice(BudgetKind::NAMES)),
];

/// ハーネスの欄。
const HARNESS_FIELDS: &[FieldSpec] = &[
    f("label", Text),
    f("from", Text),
    f("to", Text),
    f("nets", List),
    f("wire", Text),
    f("cores", Unsigned),
    f("length", Q),
];

/// 出典の欄（参照先は `ref`）。
const SOURCE_FIELDS: &[FieldSpec] = &[
    f("kind", Choice(SourceKind::NAMES)),
    f("label", Text),
    f("ref", Text),
    f("revision", Text),
    f("retrieved", Text),
    f("sha256", Text),
    f("distribution", Choice(Distribution::NAMES)),
    f("date", Text),
    f("by", Text),
    f("note", Text),
];

/// フェーズの見出しの欄（軌道 `[orbit]` と姿勢則 `[law.*]` は `fune phase set-field` が書く）。
const PHASE_FIELDS: &[FieldSpec] = &[
    f("label", Text),
    f("order", Integer),
    f("start", Text),
    f("end", Text),
    f("duration", Q),
    f("modes", List),
    f("configuration", Text),
];

/// 欄の名前・値・パスの誤り。
#[derive(Debug, Clone, PartialEq, thiserror::Error)]
pub enum RecordFieldError {
    #[error("`{field}` is not a field of {what} (fields: {choices})")]
    UnknownField {
        field: String,
        what: &'static str,
        choices: String,
    },
    #[error("Cannot read `{text}` as {expected} for `{field}`")]
    Value {
        field: String,
        text: String,
        expected: String,
    },
    #[error("{0}")]
    Quantity(#[from] ValueError),
    #[error("The record would no longer be valid with `{field}` {action}: {reason}")]
    Invalid {
        field: String,
        action: &'static str,
        reason: String,
    },
}

impl RecordKind {
    /// 欄の宣言の一覧（ファイルのキーの正規の順）。
    pub fn fields(self) -> &'static [FieldSpec] {
        match self {
            RecordKind::Case => CASE_FIELDS,
            RecordKind::Maneuver => MANEUVER_FIELDS,
            RecordKind::Requirement => REQUIREMENT_FIELDS,
            RecordKind::Harness => HARNESS_FIELDS,
            RecordKind::Source => SOURCE_FIELDS,
            RecordKind::Phase => PHASE_FIELDS,
        }
    }

    /// 誤りの文で使うレコードの呼び名。
    fn what(self) -> &'static str {
        match self {
            RecordKind::Case => "an analysis case",
            RecordKind::Maneuver => "a maneuver",
            RecordKind::Requirement => "a requirement",
            RecordKind::Harness => "a harness record",
            RecordKind::Source => "a source",
            RecordKind::Phase => "a phase heading",
        }
    }

    /// 欄の名前 → 型（書けない欄なら、書ける欄を並べた誤り）。
    pub fn field(self, name: &str) -> Result<FieldType, RecordFieldError> {
        match self.fields().iter().find(|s| s.name == name) {
            Some(s) => Ok(s.ty),
            None => Err(RecordFieldError::UnknownField {
                field: name.to_owned(),
                what: self.what(),
                choices: self
                    .fields()
                    .iter()
                    .map(|s| s.name)
                    .collect::<Vec<_>>()
                    .join(", "),
            }),
        }
    }
}

/// 欄の型付きの値（`SetRecordField` の Op が持つ。`None` はその欄を外す）。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[serde(tag = "type", content = "value", rename_all = "snake_case")]
pub enum RecordValue {
    Text(String),
    Integer(i64),
    Number(f64),
    Bool(bool),
    Quantity(Quantity),
    List(Vec<String>),
}

impl RecordValue {
    /// 打った文字列 → 欄の型の値（前後の空白は除く。**欄の型ごとの読み方はここだけ**）。
    pub fn from_text(field: &str, ty: FieldType, text: &str) -> Result<Self, RecordFieldError> {
        let t = text.trim();
        let bad = |expected: &str| RecordFieldError::Value {
            field: field.to_owned(),
            text: t.to_owned(),
            expected: expected.to_owned(),
        };
        Ok(match ty {
            Text => RecordValue::Text(t.to_owned()),
            Choice(names) => {
                if !names.contains(&t) {
                    return Err(bad(&names.join(" / ")));
                }
                RecordValue::Text(t.to_owned())
            }
            Integer => RecordValue::Integer(t.parse().map_err(|_| bad("an integer"))?),
            Unsigned => RecordValue::Integer(
                t.parse::<u32>()
                    .map(i64::from)
                    .map_err(|_| bad("a non-negative integer"))?,
            ),
            Number => {
                let n: f64 = t.parse().map_err(|_| bad("a number"))?;
                if !n.is_finite() {
                    return Err(bad("a finite number"));
                }
                RecordValue::Number(n)
            }
            Bool => RecordValue::Bool(t.parse().map_err(|_| bad("true or false"))?),
            Q => RecordValue::Quantity(quantity_from_text(t)?),
            List => RecordValue::List(list_from_text(t)),
        })
    }

    /// 値 → 打つ文字列（`fune set` の引数。[`RecordValue::from_text`] の逆）。
    pub fn to_text(&self) -> String {
        match self {
            RecordValue::Text(s) => s.clone(),
            RecordValue::Integer(n) => n.to_string(),
            RecordValue::Number(n) => n.to_string(),
            RecordValue::Bool(b) => b.to_string(),
            RecordValue::Quantity(q) => quantity_to_text(q),
            RecordValue::List(v) => list_to_text(v),
        }
    }

    /// レコードの serde が読む形（JSON）。
    fn to_json(&self) -> Result<Json, RecordFieldError> {
        let bad = |e: String| RecordFieldError::Invalid {
            field: String::new(),
            action: "set",
            reason: e,
        };
        Ok(match self {
            RecordValue::Text(s) => Json::String(s.clone()),
            RecordValue::Integer(n) => Json::from(*n),
            RecordValue::Number(n) => {
                Json::Number(serde_json::Number::from_f64(*n).ok_or_else(|| bad(n.to_string()))?)
            }
            RecordValue::Bool(b) => Json::Bool(*b),
            RecordValue::Quantity(q) => serde_json::to_value(q).map_err(|e| bad(e.to_string()))?,
            RecordValue::List(v) => Json::from(v.clone()),
        })
    }
}

/// 欄 `field`（`constraint.value` のようなドット区切りの入れ子も可）だけを書き換えたレコード（ほかの欄はそのまま）。
/// `value` が None ならその欄を外す（入れ子の表が空になれば表ごと外す）。書き換えた結果がレコードの型として読めなければ
/// （必須の欄を外した・向きと合わない値など）誤りにする。
pub fn patched<T: Serialize + DeserializeOwned>(
    current: &T,
    field: &str,
    value: Option<&RecordValue>,
) -> Result<T, RecordFieldError> {
    let invalid = |reason: String| RecordFieldError::Invalid {
        field: field.to_owned(),
        action: if value.is_some() { "set" } else { "removed" },
        reason,
    };
    let mut json = serde_json::to_value(current).map_err(|e| invalid(e.to_string()))?;
    let keys: Vec<&str> = field.split('.').collect();
    let new = value.map(RecordValue::to_json).transpose()?;
    write_key(&mut json, &keys, new);
    serde_json::from_value(json).map_err(|e| invalid(e.to_string()))
}

/// JSON の入れ子の表の `keys` の位置に値を書く（None なら外し、空になった表は親から外す）。
fn write_key(node: &mut Json, keys: &[&str], value: Option<Json>) {
    let Some(map) = node.as_object_mut() else {
        return;
    };
    let [key, rest @ ..] = keys else { return };
    if rest.is_empty() {
        match value {
            Some(v) => {
                map.insert((*key).to_owned(), v);
            }
            None => {
                map.remove(*key);
            }
        }
        return;
    }
    if value.is_none() && !map.contains_key(*key) {
        return;
    }
    let child = map
        .entry((*key).to_owned())
        .or_insert_with(|| Json::Object(Default::default()));
    if !child.is_object() {
        *child = Json::Object(Default::default());
    }
    write_key(child, rest, value);
    if child.as_object().is_some_and(|m| m.is_empty()) {
        map.remove(*key);
    }
}

/// 欄のパス（`<集まり>.<ID>.<欄>`。欄は入れ子の表なら `constraint.value`）→ (集まり, ID, 欄)。
/// 先頭が集まりの名前でない・区切りが 3 つ未満（`harness.mass` は部品の属性のパス）なら None（部品や設定のパスは
/// [`crate::path::parse`]）。集まりの名前と同じ ID の部品があるかどうかはここでは見ない（service が設計を見て決める）。
/// 欄の名前の検査は [`RecordKind::field`]。
pub fn parse_record_field(path: &str) -> Option<Result<(RecordKind, String, String), PathError>> {
    let segments: Vec<&str> = path.trim().split('.').collect();
    let kind: RecordKind = segments.first()?.parse().ok()?;
    if segments.len() < 3 {
        return None;
    }
    Some(match segments[..] {
        [_, id, ref field @ ..]
            if !id.is_empty() && !field.is_empty() && field.iter().all(|s| !s.is_empty()) =>
        {
            Ok((kind, id.to_owned(), field.join(".")))
        }
        _ => Err(PathError::Record(path.to_owned())),
    })
}

#[cfg(test)]
mod tests;
