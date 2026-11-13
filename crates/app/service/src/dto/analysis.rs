//! 解析の API の型。解析ごとの入出力の型は sat-analysis が定義し、そのまま TS に出す。

use super::{AnalysisLabel, KpiDef, KpiView};
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
    /// この解析が出す KPI（並びは系のページに並べる順。値は実行の結果の `kpis`）
    pub kpis: Vec<KpiDef>,
}

/// 解析の実行の依頼（`POST /analyses/{id}`、`fune analyze <id>`）。
#[derive(Debug, Clone, Default, Serialize, Deserialize, TS)]
#[serde(default)]
pub struct RunAnalysisRequest {
    /// 解析ケースの ID（`cases/<ID>.toml`）。省略時は projection の対象解析だけ既定ケースを適用
    pub case: Option<String>,
    /// 入力の与え方（省略時は設計データから組み立てる）
    pub source: AnalysisInputSource,
    /// 保存していない編集。書き込まずに当てた設計から入力を組み立てる（空なら保存済みの設計）。
    /// `source` が `given` のときは使わない
    pub ops: Vec<fune_ops::Op>,
}

/// 解析の入力の与え方。
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize, TS)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum AnalysisInputSource {
    /// 設計データ（`ops` があれば書き込まずに当てたもの）から組み立てる
    #[default]
    Design,
    /// 設計データから組み立てた入力に JSON Merge Patch（RFC 7396）を重ねる。設定の一時的な上書きに使う
    Patched {
        #[ts(type = "unknown")]
        patch: serde_json::Value,
    },
    /// 入力を丸ごと与える（設計データから組み立てない。`input_type` の形）
    Given {
        #[ts(type = "unknown")]
        input: serde_json::Value,
    },
}

/// 解析の判定 1 つ。出力のあちこち（モードごと・ケースごとなど）にある判定を並べるので、どこにあったかを添える。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
pub struct CheckView {
    /// 判定の種類（例: `"dod"`）。同じ解析の中でも、モードごとに同じ key が並ぶことがある
    pub key: String,
    pub label: String,
    /// 英語の項目名（語彙から引いた判定だけ。null なら `label` を使う）
    pub label_en: Option<String>,
    pub verdict: sat_analysis::spec::Verdict,
    pub message: String,
    /// 英語の文（語彙から引いた判定だけ。null なら `message` を使う）
    pub message_en: Option<String>,
    /// 判定そのものの場所（出力の JSON Pointer。例: `"/cases/3/checks/1"`）。判定ごとに一意
    pub at: String,
    /// 判定の持ち主の名前（同じ階層の `label`、無ければ `id`。例: `"ダウンリンク"`）。出力の最上位なら null
    pub scope: Option<String>,
}

/// 解析ケースの選択元。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "snake_case")]
pub enum AnalysisCaseSource {
    Explicit,
    Default,
    /// 電力収支の上段の姿勢則だけ既定ケースから選択（case は null）。
    DefaultAttitude,
}

/// 解析の実行結果。
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
pub struct AnalysisRunResult {
    pub id: String,
    pub case: Option<String>,
    pub case_source: Option<AnalysisCaseSource>,
    pub status: AnalysisRunStatus,
    /// 出力中の判定を、出てきた順に平らに並べたもの（場所と持ち主の名前つき）。
    pub checks: Vec<CheckView>,
    /// 判定が無ければ null。
    pub verdict: Option<sat_analysis::spec::Verdict>,
    /// この解析の KPI（`AnalysisEntry.kpis` と同じ並び。出力が無ければ空）。代表値の選び方・判定・塗りの比は
    /// `fune_service::analysis::kpi` が決める
    pub kpis: Vec<KpiView>,
    /// 実際に使った入力（`input_type` の形）
    #[ts(type = "unknown")]
    pub input: Option<serde_json::Value>,
    /// 出力（`output_type` の形）
    #[ts(type = "unknown")]
    pub output: Option<serde_json::Value>,
    /// 出力の量の次元（JSON Pointer → 次元名。例: `"/modes/0/total": "Power"`）。表示は `POST /units/format` に渡す
    #[ts(type = "Record<string, string>")]
    pub dims: std::collections::BTreeMap<String, String>,
    /// 出力の中の語彙の値（列挙）の表示名（JSON Pointer → 表示名。例: `"/disturbances/0/key"`）。
    /// 解析は値の綴り（`gravity_gradient`）だけを返すので、表示はこれを使う
    pub labels: std::collections::BTreeMap<String, AnalysisLabel>,
    /// 足りない入力（`status` が `missing_inputs` のとき）
    pub missing: Vec<MissingInput>,
    /// 解析が失敗したときの理由
    pub error: Option<String>,
    /// 入力に使った宇宙天気の表の版（日付で引いた宇宙天気を使った解析だけ。使わなければ欄を出さない）
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub space_weather: Option<super::data::SpaceWeatherVersion>,
}

/// 解析の実行の状態。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "snake_case")]
pub enum AnalysisRunStatus {
    Ok,
    MissingInputs,
    Failed,
}
