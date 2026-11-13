//! Op → 同じ操作をする CLI のコマンド文字列（GUI の「CLI としてコピー」）。
//!
//! 1 つの Op が複数のコマンドになることがある（モード表の一括編集・状態ごとの値の丸ごとの書き込み・図の配置）。
//!
//! | モジュール | 中身 |
//! |---|---|
//! | `text` | 引用・数値と単位の書き方・コマンドの組み立て |
//! | `commands` | 部品・定義・モード・ネット・図・設定の Op のコマンド |
//! | `records` | 取付点・解析ケース・出典・ハーネスの Op のコマンド |
//! | `catalog` | カタログ品の Op のコマンド |
//! | `defs` | 定義の Op のコマンド（`attr add`・`kind add`。フィールドと旗は 1 対 1） |

mod catalog;
mod commands;
mod defs;
mod records;
mod text;

pub use text::quote;

use crate::op::Op;

/// Op → コマンド（1 行以上）。
pub fn to_cli(op: &Op) -> Vec<String> {
    commands::part_op(op)
        .or_else(|| records::record_op(op))
        .or_else(|| catalog::catalog_op(op))
        .unwrap_or_else(|| commands::other_op(op))
}

/// Op の列 → 1 行 1 コマンドのスクリプト。
pub fn to_cli_script(ops: &[Op]) -> String {
    ops.iter().flat_map(to_cli).collect::<Vec<_>>().join("\n")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::op::ModeCell;
    use fune_model::value::{Provenance, Scalar, SettingValue, Value};
    use std::collections::BTreeMap;

    #[test]
    fn commands_match_the_plan() {
        let op = Op::SetValue {
            part: "RW_X".into(),
            attr: "mass".into(),
            state: None,
            value: Value::Scalar(Scalar::new(620.0, "g")),
            from: Some(Provenance {
                source: Some("em_test".into()),
                note: Some("EM 実測".into()),
            }),
        };
        assert_eq!(
            to_cli(&op),
            vec![r#"fune set RW_X.mass "620 g" --source em_test --note "EM 実測""#]
        );
        let op = Op::SetModeStates {
            cells: vec![ModeCell {
                mode: "NOMINAL".into(),
                part: "TRX".into(),
                state: Some("rx".into()),
                duty: BTreeMap::from([("tx".to_string(), 0.1)]),
                active: None,
                spare_state: None,
                eclipse: false,
            }],
        };
        assert_eq!(
            to_cli(&op),
            vec!["fune mode set NOMINAL TRX rx --duty tx=0.1"]
        );
        let op = Op::SetSetting {
            path: "analysis.power.dod_max".into(),
            value: Some(SettingValue::Number(0.3)),
        };
        assert_eq!(
            to_cli(&op),
            vec!["fune setting set analysis.power.dod_max 0.3"]
        );
        assert_eq!(quote("mN*m*s"), "\"mN*m*s\"");
    }
}
