//! モードの時刻の指定（`modes/<ID>.toml` の `when`・`start`）と、モードの並びの先頭の位置（`analysis/power.toml` の
//! `[sequences.<名前>]` の `anchor`）。日照・食に対する位置の指定の綴りと形はここだけ
//! （解析の側の `sat_analysis::power::energy_balance` の同名の型とは、fune-projection のテストで綴りの一致を確かめる）。

use crate::enum_text::enum_text;
use crate::value::Quantity;
use serde::{Deserialize, Serialize};
use ts_rs::TS;

enum_text! {
    /// モードを使う時間帯。
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
    pub enum ModeWhen: "mode time window" {
        /// 時間帯の指定なし
        Any = "any",
        /// 日照中だけ
        Sunlit = "sunlit",
        /// 日陰中（食）だけ
        Eclipse = "eclipse",
    }
}

enum_text! {
    /// 開始の基準にする食の出入り。
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
    pub enum StartFrom: "start reference" {
        /// 食明け（日陰から日照に戻る時刻）
        EclipseExit = "eclipse_exit",
        /// 食入り（日照から日陰に入る時刻）
        EclipseEntry = "eclipse_entry",
    }
}

/// 食の出入りからの開始の指定（基準の時刻 + `offset`。負なら前。省略は 0）。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[serde(deny_unknown_fields)]
pub struct StartRef {
    pub from: StartFrom,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub offset: Option<Quantity>,
}
