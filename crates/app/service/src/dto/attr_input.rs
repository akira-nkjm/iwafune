//! 属性の値を欄で直すときの入力の形（`AttributeDefView.input`）。**何で入力させるかは service が決めて返し**、
//! Web は種類から欄の部品を選ぶだけで、属性の名前では分けない（決める関数は `fixes::attr_input` ただ 1 つ。不足入力の直し方の候補と共有する）。

use fune_model::role::Role;
use fune_projection::{Choice, RefKind};
use serde::{Deserialize, Serialize};
use ts_rs::TS;

/// 欄で直せない理由（値の型。`fune set` か TOML で直す）。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "snake_case")]
pub enum AttrReadOnlyReason {
    /// 3 成分のベクトル
    Vector,
    /// クォータニオン
    Quaternion,
    /// 3x3 のテンソル（文字列では打てない）
    Tensor,
    /// 範囲
    Range,
    /// 曲線（表）
    Curve,
    /// 表（`tables/` の表の ID）
    Table,
}

/// 属性の値の入力の形。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum AttrInput {
    /// 量（数と単位。状態ごとの値は `by_state`）
    Quantity {
        /// 次元の名前（無ければ null）
        dimension: Option<String>,
        by_state: bool,
    },
    /// 決まった語彙から選ぶ（列挙・真偽）
    Choice { choices: Vec<Choice> },
    /// 設計にあるものを選ぶ（値はその ID）。候補は設計から絞って返す
    Reference {
        what: RefKind,
        /// 候補を絞った部品の役割（空なら絞っていない）
        roles: Vec<Role>,
        choices: Vec<Choice>,
        /// 候補が無いときに足す CLI のコマンド
        add_command: Option<String>,
    },
    /// 自由に文字列を打つ（語彙が決まっていない列挙・文字列）
    Text,
    /// 欄では直せない（理由は値の型）
    ReadOnly { reason: AttrReadOnlyReason },
}
