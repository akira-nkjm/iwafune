//! 値を入れる候補（`Fix::SetValue`）の組み立て。書く先の次元・入力の例・選べる値（列挙・真偽）・カタログ品の側の書き先。
//! 次元の名前と入力の例は `sat_units`、列挙の語彙は属性定義（`AttributeDef::symbols`・`value_type`）から引き、表をここに持たない。

use super::{Design, Fix, MissingInput, ParamPath, quote};
use fune_model::path::{SettingPath, SettingScope};
use fune_model::value::ValueType;
use fune_model::value::units::unit_dimension;
use fune_projection::{CatalogFix, Choice};

/// 置き換え欄（値）。コマンドの中では `"<value>"` と引用して書く。
pub(super) const VALUE: &str = "\"<value>\"";

/// 値を書く先（部品の属性・フィールド・設定）に値を入れる候補。
pub(super) fn set_value(m: &MissingInput, target: ParamPath, dimension: Option<String>) -> Fix {
    Fix::SetValue {
        path: m.path.clone(),
        command: format!("fune set {} {VALUE}", quote(&m.path)),
        target,
        example: example_of(dimension.as_deref()),
        dimension,
        catalog: None,
        choices: Vec::new(),
    }
}

/// 次元名 → 入力の例（`1 kg`。`sat_units::format::input_example`）。次元が無ければ None。
pub(super) fn example_of(dimension: Option<&str>) -> Option<String> {
    sat_units::format::input_example(dimension?)
}

/// 量の入力の例（`1 km`）→ ベクトルの入力の例（`1 0 0 km`。単位は共通）。読めなければ None。
pub(super) fn vector_example(quantity: &str) -> Option<String> {
    let (value, unit) = quantity.split_once(' ')?;
    Some(format!("{value} 0 0 {unit}"))
}

/// 属性の選べる値: 列挙で語彙（`symbols`）が決まっていればその値、真偽なら `true`・`false`。自由入力なら空。
pub(super) fn choices_of_attr(design: &Design, attr: &str) -> Vec<Choice> {
    let Some(def) = design.attribute(attr) else {
        return Vec::new();
    };
    let words: Vec<&str> = match def.value_type {
        ValueType::Enum => def.symbols.iter().map(String::as_str).collect(),
        ValueType::Bool => vec!["true", "false"],
        _ => Vec::new(),
    };
    words
        .into_iter()
        .map(|w| Choice {
            value: w.to_owned(),
            label: w.to_owned(),
            label_en: None,
        })
        .collect()
}

/// `conditions.launch.<キー>` の欄で、キーが列挙の属性（拘束の種類・面）と同じ名前のもの。選べる値は属性定義の語彙
/// （カタログの品に書く値と同じ。語彙の表を持たない）。そうでなければ None。
pub(super) fn launch_enum(design: &Design, m: &MissingInput, s: &SettingPath) -> Option<Fix> {
    let [file, key] = s.segments.as_slice() else {
        return None;
    };
    if s.scope != SettingScope::Conditions || file != "launch" {
        return None;
    }
    let choices = choices_of_attr(design, key);
    if choices.is_empty() {
        return None;
    }
    let mut fix = set_value(m, ParamPath::Setting(s.clone()), None);
    if let Fix::SetValue { choices: c, .. } = &mut fix {
        *c = choices;
    }
    Some(fix)
}

/// 属性の次元名（定義の `dimension`、無ければ `units` の先頭の単位の次元）。
pub(super) fn attr_dimension(design: &Design, attr: &str) -> Option<String> {
    let def = design.attribute(attr)?;
    def.dimension
        .clone()
        .or_else(|| def.units.first().and_then(|u| unit_dimension(u).ok()))
}

/// 部品の属性に値を入れる候補。部品がカタログ品を選んでいれば、品の側へ書く代わりの書き先も付ける。
pub(super) fn set_attr(
    design: &Design,
    m: &MissingInput,
    part: &str,
    attr: &str,
    state: Option<&str>,
) -> Fix {
    let path = ParamPath::Attr {
        part: part.to_owned(),
        attr: attr.to_owned(),
        state: state.map(str::to_owned),
    };
    let mut fix = set_value(m, path, attr_dimension(design, attr));
    if let Fix::SetValue { choices, .. } = &mut fix {
        *choices = choices_of_attr(design, attr);
    }
    let item = design.parts.get(part).and_then(|p| p.catalog.as_ref());
    if let (Fix::SetValue { catalog, .. }, Some(item)) = (&mut fix, item) {
        let inside = state.map_or_else(|| attr.to_owned(), |s| format!("{attr}.{s}"));
        let item = item.to_string();
        *catalog = Some(CatalogFix {
            command: format!(
                "fune catalog set {} {} {VALUE}",
                quote(&item),
                quote(&inside)
            ),
            item,
            path: inside,
        });
    }
    fix
}
