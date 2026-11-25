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
//! | `history` | 編集の取り消し・やり直し（`fune undo`・`fune redo`・`POST /undo`・`POST /redo`。履歴は `.fune/history.json`、適用は fune-ops） |
//! | `preview` | 保存していない編集（Op の列）を書き込まずに当てた設計（下書きのまま解析・バジェットを見る） |
//! | `units` | 単位の一覧・入力文字列の解析 |
//! | `stale_acknowledgements` | 古くなった承知（幾何の所見の承知済みの組が今のどの所見にも当たらない）の診断。解析を回して所見を知り、`Snapshot::of` が `fune check` の診断に足す（`analysis` に依存しない葉） |
//! | `session` | 実行の記憶（1 回の読み込みの設計を持ち、同じ解析・同じケースの結果を 1 回だけ計算して使い回す。要求適合表・設計書の書き出し・電源のモードの表示が使う。ほかの module に依存しない葉） |
//! | `analysis` | 解析の一覧・実行・KPI（代表値・判定・塗りの比） |
//! | `budget` | 質量・電力・データ量のバジェット |
//! | `requirements` | 要求適合表（要求ごとに、確かめる先の解析・バジェットの判定を量化したケースで集める） |
//! | `mode_systems` | モードの見方を持つ系の一覧（`DefsView.mode_systems` と `system_modes` の対応判定の共通の置き場。ほかの module に依存しない） |
//! | `attitude_view` | 姿勢則の設定と実効軸の表示（AOCS のモードの姿勢則とフェーズの上書きの姿勢則が同じ欄を使う。どちらからも呼ばれる葉の module） |
//! | `system_modes` | 系ごとのモードの見方（電源・C&DH・熱・AOCS の値・合計・判定） |
//! | `thermal_tables` | 熱の結合・表面の編集の表（解析が使った結合の値と出どころ・表面材・面の割合・節点の温度。下書きを当てて作れる） |
//! | `data` | 外部データ（DE440・宇宙天気）の取得・状態・読み込み（ユーザーごとのキャッシュ）、日付からの宇宙天気 |
//! | `shared_catalog` | 共有カタログ（`[[catalog]] git = …`）の取得・更新（git の CLI。プロジェクトの `.fune/catalogs/` のキャッシュと `catalog.lock`） |
//! | `fixes` | 不足入力の直し方の候補（操作の種類・絞り込み・CLI のコマンド。`fune analyze` と GUI の行の操作が同じ候補を使う） |
//! | `fetch` | 外部からの取得（`curl`。HTTPS だけ。宇宙天気と `self_update` が使う） |
//! | `self_update` | `fune self-update`（最新のリリースの確認と、実行中の `fune` の置き換え。CLI だけ） |
//! | `skill` | `fune skill`（AI エージェント向けの skill 本文 `skill/SKILL.md` をバイナリに埋め込み、`.claude/skills/fune/` などへ入れる。CLI だけ） |
//! | `report` | 設計書に入れる表と図の書き出し（`fune report export`・`POST /report/export`。解析の結果から表・図・値を組み立て、ファイルの内容を返す。書き出すのは呼び出し側） |
//! | `diff` | 変更前後の本文 → unified diff |
//! | `error` | サービスのエラー（HTTP の状態と CLI の終了コードに写せる分類） |

pub mod analysis;
pub(crate) mod attitude_view;
pub mod budget;
pub mod data;
pub mod diff;
pub mod dto;
pub mod edit;
pub mod error;
pub(crate) mod fetch;
pub mod history;
pub mod mode_systems;
pub mod preview;
pub mod query;
pub mod report;
pub mod requirements;
pub mod self_update;
pub mod session;
pub mod shared_catalog;
pub mod skill;
mod stale_acknowledgements;
pub mod system_modes;
pub mod thermal_tables;
pub mod units;
pub mod workspace;

pub use error::{ErrorKind, Result, ServiceError};
pub use workspace::Workspace;

pub mod export;
pub mod fixes;

pub mod format;
