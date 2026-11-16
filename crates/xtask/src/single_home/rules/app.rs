//! 単位・設計データ・service・store の「1 か所だけ」の規則（データだけ。照合は `single_home`）。台帳（registry.md §2）に行を足したら、ここにも足す。

use super::Rule;

/// (探す文字列, 許すパスの接尾辞の一覧, 台帳での機能名)
pub(in crate::single_home) const RULES: &[Rule] = &[
    // 単位
    ("fn parse_unit", &["crates/lib/units/src/"], "unit parsing"),
    // 解析の登録
    (
        "fn run_json",
        &["crates/lib/analysis/src/catalog.rs"],
        "analysis registration",
    ),
    // 列挙の綴り（serde_json を経由して綴りを得る関数を書かない。`enum_text!` の `as_str` を使う）
    (
        "fn enum_str",
        &["crates/app/model/src/enum_text.rs"],
        "enum spellings (`enum_text!`)",
    ),
    // 値の文字列の書式（読む向きと書く向き）
    (
        "fn value_to_text",
        &["crates/app/model/src/value/text"],
        "the text form of values",
    ),
    (
        "fn setting_to_text",
        &["crates/app/model/src/value/text"],
        "the text form of setting values",
    ),
    (
        "fn list_from_text",
        &["crates/app/model/src/value/text.rs"],
        "how lists are read",
    ),
    // 次元 → 次元名（同じ名前の別物を作らない。別名を選ぶのは `dimension_label_preferring`）
    (
        "fn dimension_name(",
        &["crates/lib/units/src/format.rs"],
        "dimension to dimension name",
    ),
    // 位置の既定の単位（`fune_model::design::DEFAULT_POSITION_UNIT`・`position_unit`）
    (
        "unwrap_or(\"m\")",
        &["crates/app/model/src/design/part.rs"],
        "the default unit of positions",
    ),
    // service の JSON Pointer の区間と、計画の変更をファイルの集合に当てる処理
    (
        "fn pointer_token",
        &["crates/app/service/src/analysis/dims.rs"],
        "how JSON Pointer segments are written",
    ),
    (
        "fn apply_changes",
        &["crates/app/store/src/write.rs"],
        "applying a list of changes to a set of files",
    ),
    // 配置の姿勢の角度の読み方（`--rotation`・GUI の X・Y・Z の欄）
    (
        "fn rotation_from_text",
        &["crates/app/service/src/edit/place.rs"],
        "how attitude angle text is read",
    ),
];
