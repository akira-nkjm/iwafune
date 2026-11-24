//! 解析の API の型。解析ごとの入出力の型は sat-analysis が定義し、そのまま TS に出す。

use super::{AnalysisLabel, CheckView, KpiDef, KpiView};
use fune_projection::MissingInput;
use fune_projection::configuration_policy::ConfigurationSource;
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
    /// 頼んだときだけ返す重い任意の出力（`AnalysisRunResult.detail`）の TS の型名。持たない解析は欄を出さない
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub detail_type: Option<String>,
}

/// 解析の実行の依頼（`POST /analyses/{id}`、`fune analyze <id>`）。
#[derive(Debug, Clone, Default, Serialize, Deserialize, TS)]
#[serde(default)]
pub struct RunAnalysisRequest {
    /// 解析ケースの ID（`cases/<ID>.toml`）。省略時は projection の対象解析だけ既定ケースを適用
    pub case: Option<String>,
    /// 実行時に明示する形態の ID（設計データから組み立てる入力用）。
    #[serde(skip_serializing_if = "Option::is_none")]
    #[ts(optional = nullable)]
    pub configuration: Option<String>,
    /// 入力の与え方（省略時は設計データから組み立てる）
    pub source: AnalysisInputSource,
    /// 保存していない編集。書き込まずに当てた設計から入力を組み立てる（空なら保存済みの設計）。
    /// `source` が `given` のときは使わない
    pub ops: Vec<fune_ops::Op>,
    /// 重い任意の出力（メッシュ上の場など。`AnalysisEntry.detail_type` を持つ解析だけ）も返す。既定は返さない
    pub detail: bool,
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
    /// 解析が使った形態の ID（基準なら省略）。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub configuration: Option<String>,
    /// 形態の選択元（基準なら省略）。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub configuration_source: Option<ConfigurationSource>,
    /// 解析が読んだ不確かさの側（ケースの `side`。ケースがあれば書かれていなくても `nominal`、ケースが無ければ欄を出さない）。
    /// `min` / `max` のときは、その側が書かれた値だけがその値で、書かれていない値は nominal を使っている
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub side: Option<fune_model::design::UncertaintySide>,
    pub status: AnalysisRunStatus,
    /// 出力中の判定を、出てきた順に平らに並べたもの（場所と持ち主の名前つき）。
    pub checks: Vec<CheckView>,
    /// 判定が無ければ null。
    pub verdict: Option<sat_analysis::spec::Verdict>,
    /// この解析の KPI（`AnalysisEntry.kpis` と同じ並び。出力が無ければ空。設計が持たない手段の KPI だけは値が無ければ外れる）。代表値の選び方・判定・塗りの比は
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
    /// 系の概要の解析の表の行（主な結果・判定のまとまり・開いたときの棒グラフ・隠す表）。主な結果と判定のまとまりは
    /// `analysis::overview` が作り、棒グラフと隠す表は系ごとの組み立て（電源は `analysis::overview::power`）が足す。ほかは null と空
    pub overview: super::AnalysisOverview,
    /// 部品ごとの代表の温度（熱の定常解析だけ。それ以外は欄を出さない）。節点と部品の対応と代表の選び方は
    /// `analysis::part_heat` が決める。Web は 3D の色に値を写すだけ
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub part_heat: Option<Vec<PartHeat>>,
    /// 入力に使った宇宙天気の表の版（日付で引いた宇宙天気を使った解析だけ。使わなければ欄を出さない）
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub space_weather: Option<super::data::SpaceWeatherVersion>,
    /// 重い任意の出力（依頼の `detail` が真で、解析が持つときだけ。型は `AnalysisEntry.detail_type`。それ以外は欄を出さない）
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(type = "unknown", optional)]
    pub detail: Option<serde_json::Value>,
}

/// 解析の実行の状態。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "snake_case")]
pub enum AnalysisRunStatus {
    Ok,
    MissingInputs,
    Failed,
}

/// 部品 1 つの代表の温度（熱の 3D の色）。代表は全ケースを通した節点の最高温度で、部品は節点の温度を共有する。
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
pub struct PartHeat {
    /// 部品の ID
    pub part: String,
    /// 部品の属する熱の節点の ID
    pub node: String,
    /// その節点が既定の 1 部品 1 節点か（偽は設計が `[nodes]` で複数の部品をまとめた節点）
    pub per_part: bool,
    /// 代表の温度（全ケースの最高）
    pub temperature: super::QuantityView,
    /// 最高になったケースの ID
    pub case: String,
    /// 節点の温度範囲との比較のうち最も悪い判定（全ケース・全種類を通す）
    pub verdict: sat_analysis::spec::Verdict,
}
