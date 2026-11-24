//! # fune-projection — Resolved → 解析の入力型
//!
//! 解析（sat-analysis）は設計モデルを知らないので、ここで入力を組み立てる。足りない入力はパスで列挙する。
//! 物理計算はしない（必要なら sat-analysis / sat-astro の関数を呼ぶ）。
//!
//! | モジュール | 責務 |
//! |---|---|
//! | `case_policy` | 解析ごとの既定ケースの対象宣言と解決 |
//! | `context` | 解析ケースの文脈と、どの解析でも使う入力（軌道・地上局・姿勢則・モード） |
//! | `timeline` | 時系列の仕様（軌道・姿勢則・期間・刻み・β） |
//! | `parts` | 解決済みの部品から値を取り出す部品（役割での選択・型付きの量・機体座標の方向） |
//! | `pose` | 取付の連鎖 → 機体座標の位置・姿勢（sat-astro の Quat で合成。合成はここだけ） |
//! | `quantity` | 書かれた量 → 型付きの SI 量（換算は sat-units） |
//! | `missing` | 不足入力の型と収集 |
//! | `fix` | 不足入力の直し方の候補の型（決めるのは service の `fixes`） |
//! | `reads` | 射影が読んだもの（部品の属性・設定・モード・ケース）の記録。記録の入口は `Context::note` だけ |
//! | `launch` | 打ち上げ条件（`conditions/launch.toml` が選ぶ打ち上げ機・ディスペンサー）の読み出しと、値の優先順（launch.toml → ディスペンサー → 打ち上げ機）、限度の候補（要求を足す） |
//! | `launch_fit` | `structure.launch_fit` の射影 |
//! | `maneuver_dates` | マヌーバの日付つきの遷移（`[transfer]`）の日付の読み方と、暦を要る検査（`fune check` の診断に service が足す） |
//! | `aocs` `cdh` `comm` `power` `propulsion` `structure` `thermal` | 系ごとの射影 |
//! | `orbit` | 軌道条件の解析の入力（季節解析: 軌道・期間・任意の刻み。高度低下: 軌道・質量・外形・抗力係数・`[decay]` の設定・推進系） |
//! | `configuration_policy` | 解析が使う形態と選択元の決定 |
//! | `dispatch` | 解析 ID → 射影（入力の JSON）の対応表と、同じ表から引く型付きの射影（`Projects`・`project_typed`）。実行は service（`sat_analysis::catalog::run_json`・`run_typed`） |
//! | `system` | 解析の系の対応（model の `AnalysisSystem` ⇄ sat-analysis の `System`）と一致のテスト |

pub mod aocs;
pub mod case_policy;
pub mod cdh;
pub mod comm;
pub mod configuration_policy;
pub mod context;
pub mod dispatch;
pub mod fix;
pub mod launch;
pub mod launch_fit;
pub mod maneuver_dates;
pub mod missing;
/// 軌道条件の解析（季節解析・高度低下）への写し替え。
pub mod orbit;
pub mod parts;
pub mod pose;
pub mod power;
pub mod propulsion;
pub mod quantity;
pub mod reads;
pub mod structure;
pub mod system;
pub mod thermal;
pub mod timeline;

pub use context::Context;
pub use dispatch::{
    Entry, OutputVisitor, ProjectionError, Projects, entries, project_json,
    project_json_with_configuration, project_typed, visit_output,
};
pub use fix::{CatalogFix, Choice, Fix, ListKind, RefKind};
pub use missing::{MissingInput, MissingInputs};
pub use reads::ReadLog;
