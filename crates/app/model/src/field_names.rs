//! 部品・カタログ品・ネットのフィールド名の正本（ファイルのキー・パス記法の名前・CLI の旗とフィールドの名前）。
//!
//! 宣言の順がファイルの見出しのキーの正規の順（`fune fmt` と新しいファイルの書き出し）。型付きのフィールド
//! （`design::{PartField, CatalogField, NetField}`）の名前はここから引き（`key()`）、store の読み書き・ops の CLI の文字列・
//! service の打った文字列の読み方も、この一覧を回すか、この列挙で match する。
//! 属性 ID はフィールド名と重ねてはいけない（パスの記法で区別できないため。検査の `def.reserved`）。

use crate::enum_text::enum_text;
use serde::{Deserialize, Serialize};

enum_text! {
    /// 部品のフィールド名（`RW_X.label`。正規の順: label → description → catalog → kind → candidates → count → maturity）。
    #[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
    pub enum PartFieldName: "部品のフィールド" {
        Label = "label",
        Description = "description",
        Catalog = "catalog",
        Kind = "kind",
        Candidates = "candidates",
        Count = "count",
        Maturity = "maturity",
    }
}

enum_text! {
    /// カタログ品のフィールド名（`fune catalog set <品> <フィールド>`）。
    #[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
    pub enum CatalogFieldName: "カタログ品のフィールド" {
        Kind = "kind",
        Label = "label",
        LabelEn = "label_en",
        Manufacturer = "manufacturer",
        PartNumber = "part_number",
        Maturity = "maturity",
        Datasheet = "datasheet",
        Source = "source",
    }
}

enum_text! {
    /// ネットのフィールド名（`nets/<ID>.toml` のキー・`fune net set --<フィールド>`・`NetView.field_paths` のキー）。
    #[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
    pub enum NetFieldName: "ネットのフィールド" {
        Kind = "kind",
        Label = "label",
        Source = "source",
        Rail = "rail",
        Limit = "limit",
        Protocol = "protocol",
        Bitrate = "bitrate",
        Master = "master",
    }
}
