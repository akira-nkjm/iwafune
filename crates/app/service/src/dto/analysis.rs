//! 解析の API の型。解析ごとの入出力の型は sat-analysis が定義し、そのまま TS に出す。

use fune_projection::MissingInput;
use sat_analysis::spec::AnalysisInfo;
use serde::{Deserialize, Serialize};
use ts_rs::TS;

/// 解析の一覧の 1 行（`GET /analyses`、`fune analyze ls`）。
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
pub struct AnalysisEntry {
    pub info: AnalysisInfo,
    /// 入力の TS 型名（例: `"EnergyBalanceInput"`）
    pub input_type: String,
    /// 出力の TS 型名（例: `"EnergyBalanceOutput"`）
    pub output_type: String,
    /// 設計データから入力を組み立てられるか（projection が実装済みか）
    pub projectable: bool,
}

/// 解析の実行の依頼（`POST /analyses/{id}`、`fune analyze <id>`）。
#[derive(Debug, Clone, Default, Serialize, Deserialize, TS)]
#[serde(default)]
pub struct RunAnalysisRequest {
    /// 解析ケースの ID（`cases/<ID>.toml`）。省略すると既定のケース
    pub case: Option<String>,
    /// 設計データから組み立てた入力に重ねる JSON Merge Patch（RFC 7396）。設定の一時的な上書きに使う
    #[ts(type = "unknown")]
    pub input_patch: Option<serde_json::Value>,
    /// 入力を丸ごと与える（設計データから組み立てない）。`input_patch` より優先
    #[ts(type = "unknown")]
    pub input: Option<serde_json::Value>,
    /// 保存していない編集。書き込まずに当てた設計から入力を組み立てる（空なら保存済みの設計）
    pub ops: Vec<fune_ops::Op>,
}

/// 解析の判定 1 つ。出力のあちこち（モードごと・ケースごとなど）にある判定を並べるので、どこにあったかを添える。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
pub struct CheckView {
    /// 判定の種類（例: `"dod"`）。同じ解析の中でも、モードごとに同じ key が並ぶことがある
    pub key: String,
    pub label: String,
    pub verdict: sat_analysis::spec::Verdict,
    pub message: String,
    /// 判定そのものの場所（出力の JSON Pointer。例: `"/cases/3/checks/1"`）。判定ごとに一意
    pub at: String,
    /// 判定の持ち主の名前（同じ階層の `label`、無ければ `id`。例: `"ダウンリンク"`）。出力の最上位なら null
    pub scope: Option<String>,
}

/// 解析の実行結果。
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
pub struct AnalysisRunResult {
    pub id: String,
    pub case: Option<String>,
    pub status: AnalysisRunStatus,
    /// 出力中の判定を、出てきた順に平らに並べたもの（場所と持ち主の名前つき）。
    pub checks: Vec<CheckView>,
    /// 判定が無ければ null。
    pub verdict: Option<sat_analysis::spec::Verdict>,
    /// 実際に使った入力（`input_type` の形）
    #[ts(type = "unknown")]
    pub input: Option<serde_json::Value>,
    /// 出力（`output_type` の形）
    #[ts(type = "unknown")]
    pub output: Option<serde_json::Value>,
    /// 出力の量の次元（JSON Pointer → 次元名。例: `"/modes/0/total": "Power"`）。表示は `POST /units/format` に渡す
    #[ts(type = "Record<string, string>")]
    pub dims: std::collections::BTreeMap<String, String>,
    /// 足りない入力（`status` が `missing_inputs` のとき）
    pub missing: Vec<MissingInput>,
    /// 解析が失敗したときの理由
    pub error: Option<String>,
}

/// 解析の実行の状態。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "snake_case")]
pub enum AnalysisRunStatus {
    Ok,
    MissingInputs,
    Failed,
}

/// service が表示情報を添えた時系列チャンネル。
#[derive(Debug, Clone, serde::Serialize, ts_rs::TS)]
pub struct DisplayChannel {
    /// 解析が返す SI の系列。
    #[serde(flatten)]
    pub channel: sat_analysis::spec::Channel,
    /// 表示単位。
    pub display_unit: String,
    /// SI 値に掛ける表示倍率。
    pub display_scale: f64,
}

/// run result の JSON に含まれる表示用時系列。
#[derive(Debug, Clone, serde::Serialize, ts_rs::TS)]
pub struct DisplaySeries {
    /// 元期からの経過秒。
    pub t: Vec<f64>,
    /// 表示情報を持つ系列。
    pub channels: Vec<DisplayChannel>,
}
