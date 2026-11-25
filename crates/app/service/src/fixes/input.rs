//! 属性の値の入力の形（`AttrInput`）。**「この属性は何で入力させるか」を決めるのはここだけ**
//! （不足入力の直し方の候補（`Fix::SetValue` の選べる値・`Fix::PickRef`）も、部品の属性の欄（`GET /defs` の `input`）も、これを使う）。
//! 型・次元・語彙・参照先は属性定義（`AttributeDef`）から引き、属性の名前では分けない。

use super::parts_of_roles;
use super::refs::part_choices;
use crate::dto::{AttrInput, AttrReadOnlyReason};
use fune_model::design::{AttrReference, AttributeDef, Design};
use fune_model::value::ValueType;
use fune_model::value::units::unit_dimension;
use fune_projection::{Choice, RefKind};

/// 属性の入力の形（`GET /defs` の `input`）。定義が無い属性は None。
pub fn attr_input(design: &Design, attr: &str) -> Option<AttrInput> {
    Some(input_of(design, design.attribute(attr)?))
}

/// 定義 1 つの入力の形。
fn input_of(design: &Design, def: &AttributeDef) -> AttrInput {
    let read_only = |reason| AttrInput::ReadOnly { reason };
    match def.value_type {
        ValueType::Scalar => AttrInput::Quantity {
            dimension: dimension_of(def),
            by_state: def.by_state,
        },
        ValueType::Enum if !def.symbols.is_empty() => AttrInput::Choice {
            choices: words(&def.symbols),
        },
        ValueType::Bool => AttrInput::Choice {
            choices: words(&["true", "false"]),
        },
        ValueType::Text if def.references == Some(AttrReference::Part) => {
            let ids = parts_of_roles(design, &def.reference_roles);
            AttrInput::Reference {
                what: RefKind::Part,
                roles: def.reference_roles.clone(),
                choices: part_choices(design, ids),
                add_command: Some("fune part add <ID> --kind <kind>".to_owned()),
            }
        }
        ValueType::Enum | ValueType::Text => AttrInput::Text,
        ValueType::Vec3 => read_only(AttrReadOnlyReason::Vector),
        ValueType::Quat => read_only(AttrReadOnlyReason::Quaternion),
        ValueType::Tensor3 => read_only(AttrReadOnlyReason::Tensor),
        ValueType::Range => read_only(AttrReadOnlyReason::Range),
        ValueType::Curve => read_only(AttrReadOnlyReason::Curve),
        ValueType::Table => read_only(AttrReadOnlyReason::Table),
    }
}

/// 語彙の語 → 選べる値（表示名は綴りのまま）。
fn words<S: AsRef<str>>(words: &[S]) -> Vec<Choice> {
    words
        .iter()
        .map(|w| Choice {
            value: w.as_ref().to_owned(),
            label: w.as_ref().to_owned(),
            label_en: None,
        })
        .collect()
}

/// 定義の次元名（`dimension`、無ければ `units` の先頭の単位の次元）。
fn dimension_of(def: &AttributeDef) -> Option<String> {
    def.dimension
        .clone()
        .or_else(|| def.units.first().and_then(|u| unit_dimension(u).ok()))
}

/// 属性の次元名（定義が無ければ None）。
pub(super) fn attr_dimension(design: &Design, attr: &str) -> Option<String> {
    dimension_of(design.attribute(attr)?)
}

/// 属性の選べる値（列挙で語彙が決まっているもの・真偽）。自由入力・参照・量は空。
pub(super) fn choices_of_attr(design: &Design, attr: &str) -> Vec<Choice> {
    match attr_input(design, attr) {
        Some(AttrInput::Choice { choices }) => choices,
        _ => Vec::new(),
    }
}
