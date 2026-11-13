//! # fune-service — アプリ層
//!
//! **CLI と HTTP が呼ぶ関数はすべてここにある。** server と cli は整形だけをする。
//! 入出力の型は ts-rs で web/src/api/gen/ に TS として書き出す（`cargo test -p fune-service export_ts`）。
//!
//! | モジュール | 責務 |
//! |---|---|
//! | `workspace` | プロジェクトを開く・読み直す（Design と Resolved のスナップショット） |
//! | `export` / `format` | TS の型の生成・設計ファイルの整形操作 |
//! | `dto` | 画面と CLI 向けの表示用の型（部品の表示・量の表示など） |
//! | `query` | 読み取りの関数（設計全体・部品・カタログ品・定義・診断） |
//! | `edit` | 編集の関数（Op の適用と、人が打った文字列 → Op。CLI と GUI が同じ `set_op`・`unset_op`・`command_ops` を通す） |
//! | `preview` | 保存していない編集（Op の列）を書き込まずに当てた設計（下書きのまま解析・バジェットを見る） |
//! | `units` | 単位の一覧・入力文字列の解析 |
//! | `power_page` | 電源ページ全体の解析収集と表示用集約 |
//! | `analysis` | 解析の一覧・実行・KPI（代表値・判定・塗りの比） |
//! | `budget` | 質量・電力・データ量のバジェット |
//! | `system_modes` | 系ごとのモードの見方（電源・C&DH・熱・AOCS の値・合計・判定） |
//! | `data` | 外部データ（DE440）の取得・状態・読み込み（ユーザーごとのキャッシュ） |
//! | `diff` | 変更前後の本文 → unified diff |
//! | `error` | サービスのエラー（HTTP の状態と CLI の終了コードに写せる分類） |

pub mod analysis;
pub mod budget;
pub mod data;
pub mod diff;
pub mod dto;
pub mod edit;
pub mod error;
pub mod power_page;
pub mod preview;
pub mod query;
pub mod system_modes;
pub mod units;
pub mod workspace;

pub use error::{ErrorKind, Result, ServiceError};
pub use workspace::Workspace;

pub mod export;

pub mod format;
