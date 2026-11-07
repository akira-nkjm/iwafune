//! C&DH 系の解析。各解析は 1 ファイル 1 つ（入出力の型と実装）。系の中で共有する計算はサブモジュールに置き、台帳に登録する。
//!
//! | モジュール | 責務 |
//! |---|---|
//! | [`data_budget`] | `cdh.data_budget`（モードの状態と割合は `crate::modes` を使う） |

pub mod data_budget;

pub use data_budget::{DataBudget, DataBudgetInput, DataBudgetOutput};
