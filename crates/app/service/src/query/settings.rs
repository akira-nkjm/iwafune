//! 設定（`mission/`・`analysis/`・`budget/`）→ キーと値の一覧（`analysis.power.dod_max = 0.3`）。
//! 葉の列挙は `Design::setting_entries`、ファイルの位置は `fune_store::layout::locate_setting`。ここは表示用の型にするのと、
//! 設定を見せる系の対応（[`system_of`]）だけ。

use crate::dto::{QuantityView, SettingView, ValueData};
use crate::query::values::{authored_quantity, display_of};
use crate::units::number_text;
use fune_model::design::{BudgetKind, Design};
use fune_model::path::{SettingPath, SettingScope};
use fune_model::value::SettingValue;
use fune_model::value::units::to_si;
use sat_analysis::spec::System;
use serde::Deserialize;
use serde::de::value::{Error as ValueError, StrDeserializer};

/// 設定の一覧（種類 → キーの順）。書かれていない任意の値は `unset` で出す（GUI が埋められるように）。
pub(crate) fn settings(design: &Design) -> Vec<SettingView> {
    design
        .setting_entries()
        .into_iter()
        .map(|e| {
            let value = e.value.as_ref().map_or(ValueData::Unset, value_data);
            let keys = fune_store::layout::setting_location(&e.path)
                .map(|(_, keys)| keys)
                .unwrap_or_default();
            let group = SettingPath {
                scope: e.path.scope,
                segments: e.path.segments[..e.path.segments.len().saturating_sub(keys.len())]
                    .to_vec(),
            };
            SettingView {
                scope: e.path.scope,
                system: system_of(&e.path),
                group: group.to_string(),
                field: keys.join("."),
                key: e.path.to_string(),
                display: display_of(&value),
                value,
                file: fune_store::layout::locate_setting(design, &e.path).map(|l| l.file),
            }
        })
        .collect()
}

/// 設定を見せる系。**設定と系の対応はここだけ**（Web の系のページはこの欄で選ぶ）。
/// `analysis/<系>.toml` はその系、配分（`budget/allocation.toml`）は質量 → 構造・電力 → 電源・データ量 → C&DH。
/// ミッション（軌道・打ち上げ・地上局）とマージン方針（`budget.policy`）はどの系にも属さない。
pub(crate) fn system_of(path: &SettingPath) -> Option<System> {
    let first = path.segments.first()?.as_str();
    match path.scope {
        // 解析の設定のファイル名は系の名前（serde の綴り）そのもの
        SettingScope::Analysis => {
            System::deserialize(StrDeserializer::<ValueError>::new(first)).ok()
        }
        SettingScope::Budget => Some(match first.parse::<BudgetKind>().ok()? {
            BudgetKind::Mass => System::Structure,
            BudgetKind::Power => System::Power,
            BudgetKind::Data => System::Cdh,
        }),
        SettingScope::Mission => None,
    }
}

/// 無次元の数値（線形の比）。
fn number(x: f64) -> QuantityView {
    QuantityView {
        value: x,
        unit: String::new(),
        si: x,
        dimension: "Dimensionless".to_owned(),
        display: number_text(x),
    }
}

fn value_data(v: &SettingValue) -> ValueData {
    let scalar = |nominal| ValueData::Scalar {
        nominal,
        min: None,
        max: None,
    };
    match v {
        SettingValue::Number(x) => scalar(number(*x)),
        SettingValue::Integer(i) => scalar(number(*i as f64)),
        SettingValue::Quantity(q) => scalar(authored_quantity(q, None)),
        SettingValue::Bool(b) => ValueData::Bool { value: *b },
        SettingValue::Text(s) => ValueData::Text { value: s.clone() },
        SettingValue::List(items) => ValueData::Text {
            value: items.join(", "),
        },
        SettingValue::Vector(vec) => {
            let unit = vec.unit.clone().unwrap_or_default();
            let si = |x: f64| to_si(x, &unit).unwrap_or(x);
            ValueData::Vector {
                x: vec.x,
                y: vec.y,
                z: vec.z,
                si: [si(vec.x), si(vec.y), si(vec.z)],
                unit,
                frame: vec.frame.clone(),
            }
        }
    }
}
