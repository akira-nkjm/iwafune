//! Op → 同じ操作をする CLI のコマンド文字列（GUI の「CLI としてコピー」）。
//!
//! 1 つの Op が複数のコマンドになることがある（モード表の一括編集・状態ごとの値の丸ごとの書き込み・図の配置）。
//!
//! | モジュール | 中身 |
//! |---|---|
//! | `text` | 引用・数値と単位の書き方・コマンドの組み立て |
//! | `commands` | 部品・定義・モード・ネット・図・設定の Op のコマンド |
//! | `records` | 取付点・解析ケース・出典・ハーネス・マヌーバの Op のコマンド |
//! | `phases` | フェーズの Op のコマンド（旗は `fune_service` の `PhaseText` と 1 対 1。軌道と姿勢則は葉ごとの `set-field`）・ケースのフェーズ |
//! | `configurations` | 形態の Op のコマンド（旗は `fune_service` の `ConfigurationText` と 1 対 1。形態の中の配置は `part place --configuration`） |
//! | `renames` | 名前の変更・通信の回線・プロジェクト・ローカルのカタログの Op のコマンド |
//! | `requirements` | 要求の Op のコマンド（旗は `fune_service` の `RequirementText` と 1 対 1） |
//! | `catalog` | カタログ品の Op のコマンド |
//! | `defs` | 定義の Op のコマンド（`attr add`・`kind add`。フィールドと旗は 1 対 1） |

mod catalog;
mod commands;
mod configurations;
mod defs;
mod phases;
mod records;
mod renames;
mod requirements;
mod text;

pub use text::quote;

use crate::op::Op;

/// Op → コマンド（1 行以上）。
pub fn to_cli(op: &Op) -> Vec<String> {
    commands::part_op(op)
        .or_else(|| records::record_op(op))
        .or_else(|| phases::phase_op(op))
        .or_else(|| configurations::configuration_op(op))
        .or_else(|| renames::rename_op(op))
        .or_else(|| requirements::requirement_op(op))
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
                at: Some("p.3 表 2".into()),
                note: Some("EM 実測".into()),
            }),
        };
        assert_eq!(
            to_cli(&op),
            vec![r#"fune set RW_X.mass "620 g" --source em_test --at "p.3 表 2" --note "EM 実測""#]
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
