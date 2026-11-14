//! 単位・設計データ・service・store の「1 か所だけ」の規則（データだけ。照合は `single_home`）。台帳（registry.md §2）に行を足したら、ここにも足す。

use super::Rule;

/// (探す文字列, 許すパスの接尾辞の一覧, 台帳での機能名)
pub(in crate::single_home) const RULES: &[Rule] = &[
    // 単位
    ("fn parse_unit", &["crates/lib/units/src/"], "単位の解析"),
    // 解析の登録
    (
        "fn run_json",
        &["crates/lib/analysis/src/catalog.rs"],
        "解析の登録",
    ),
    // 列挙の綴り（serde_json を経由して綴りを得る関数を書かない。`enum_text!` の `as_str` を使う）
    (
        "fn enum_str",
        &["crates/app/model/src/enum_text.rs"],
        "列挙の綴り（`enum_text!`）",
    ),
    // 値の文字列の書式（読む向きと書く向き）
    (
        "fn value_to_text",
        &["crates/app/model/src/value/text"],
        "値の文字列の書き方",
    ),
    (
        "fn setting_to_text",
        &["crates/app/model/src/value/text"],
        "設定の値の文字列の書き方",
    ),
    (
        "fn list_from_text",
        &["crates/app/model/src/value/text.rs"],
        "一覧の読み方",
    ),
    // 次元 → 次元名（同じ名前の別物を作らない。別名を選ぶのは `dimension_label_preferring`）
    (
        "fn dimension_name(",
        &["crates/lib/units/src/format.rs"],
        "次元 → 次元名",
    ),
    // 位置の既定の単位（`fune_model::design::DEFAULT_POSITION_UNIT`・`position_unit`）
    (
        "unwrap_or(\"m\")",
        &["crates/app/model/src/design/part.rs"],
        "位置の既定の単位",
    ),
    // service の JSON Pointer の区間と、計画の変更をファイルの集合に当てる処理
    (
        "fn pointer_token",
        &["crates/app/service/src/analysis/dims.rs"],
        "JSON Pointer の区間の書き方",
    ),
    (
        "fn apply_changes",
        &["crates/app/store/src/write.rs"],
        "変更の列をファイルの集合に当てる処理",
    ),
];
