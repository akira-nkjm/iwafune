//! 出典の台帳の表示用の型（`GET /design` の `sources`、`fune source ls`）。

use fune_model::design::Source;
use serde::{Deserialize, Serialize};
use ts_rs::TS;

/// 出典 1 つ（書かれた欄のまま）と、画面が使う派生の欄。
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
pub struct SourceView {
    #[serde(flatten)]
    #[ts(flatten)]
    pub source: Source,
    /// 参照先が Web の URL なら、その URL（GUI のリンク。判定は `fune_model::design::Source::web_link`）
    pub link: Option<String>,
    /// 台帳のファイル（`sources/<ID>.toml`。共有カタログの出典はキャッシュの中のファイル）
    pub file: String,
    /// 置かれているディレクトリ（`sources/` からの名前の列。直下なら空。共有カタログの出典はキャッシュの `sources/` から。
    /// 置き場所だけで、ID・参照には入らない）
    pub dir: Vec<String>,
    /// このプロジェクトの出典なら true。共有カタログの出典（`<カタログ名>/<ID>`）は読み取り専用で false
    /// （判定は `Design::shared_catalog_of`）
    pub editable: bool,
}
