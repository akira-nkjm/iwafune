//! # fune-ops — 編集の唯一の入口
//!
//! CLI のコマンドも GUI の操作も、すべて [`op::Op`] に変換してここを通す。
//!
//! | モジュール | 責務 |
//! |---|---|
//! | `op` | Op の型 |
//! | `plan` | Op の列 → ファイルの変更計画（適用後の検証込み）。書き込みはしない |
//! | `apply` | 変更計画を store で書き込む（revision の確認込み） |
//! | `history` | 履歴の取り消し・やり直し（控えた前後の中身を書き戻す適用。revision の確認込み） |
//! | `cli` | Op → 同じ操作をする CLI のコマンド文字列 |
//! | `error` | Op の誤り（競合・不在・不正・検証での拒否） |

pub mod apply;
pub mod cli;
pub mod error;
pub mod history;
pub mod op;
pub mod plan;

pub use apply::{ApplyOptions, ApplyOutcome, apply};
pub use cli::{to_cli, to_cli_script};
pub use error::OpError;
pub use history::{Direction, StepOutcome, redo, undo};
pub use op::{ModeCell, ModePosition, Op, ViewNodePos};
pub use plan::{Plan, PlanSummary, RemovedKeys, plan};
