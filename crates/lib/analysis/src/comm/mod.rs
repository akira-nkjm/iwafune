//! 通信系の解析。各解析は 1 ファイル 1 つ（入出力の型と実装）。系の中で共有する計算はサブモジュールに置き、台帳に登録する。
//!
//! | モジュール | 責務 |
//! |---|---|
//! | [`link`] | 回線計算の部品（斜距離・FSPL・EIRP・G/T・C/N0・Eb/N0・回線が閉じる仰角）。回線バジェットと送信量で共有 |
//! | [`passes`] | 複数局のパスの探索と、開始からの秒の区間・日の窓への変換。可視と送信量で共有 |
//! | [`link_budget`] | `comm.link_budget` |
//! | [`visibility`] | `comm.visibility` |
//! | [`data_throughput`] | `comm.data_throughput` |

pub mod data_throughput;
pub mod link;
pub mod link_budget;
pub mod passes;
pub mod visibility;

pub use data_throughput::{DataThroughput, DataThroughputInput, DataThroughputOutput};
pub use link::RfLinkParams;
pub use link_budget::{LinkBudget, LinkBudgetInput, LinkBudgetOutput};
pub use visibility::{Visibility, VisibilityInput, VisibilityOutput};
