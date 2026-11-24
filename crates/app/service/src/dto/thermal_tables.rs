//! 熱の結合と表面の編集の表（`POST /thermal/tables`）と、その下書きの編集（`CommandText::ThermalSet`）の型。
//! 値・出どころ・節点の一覧・表面の割合の合計はここの値を作る `thermal_tables` が決め、Web は並べるだけ。

use super::{AnalysisLabel, QuantityView};
use sat_analysis::spec::Verdict;
use sat_analysis::thermal::network::ThermalCouplingSource;
use serde::{Deserialize, Serialize};
use ts_rs::TS;

/// 結合の種類。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "snake_case")]
pub enum ThermalCouplingKind {
    /// 伝導（`analysis.thermal.conductive.<節点>.<節点>`、W/K）
    Conductive,
    /// 放射（`analysis.thermal.radiative.<節点>.<節点>`、有効放射交換面積 m²）
    Radiative,
}

/// 熱の下書きの編集（`POST /ops/from_command` の `command: "thermal_set"`）。変えたセルを順に並べる。
/// それぞれが、同じ設定の `fune set <パス> <値>`・`fune unset <パス>` と同じ `SetSetting` になる
/// （パスを組み立てるのと、値の型を決めるのは service。CLI は `fune set`・`fune unset` をそのまま使う）。
#[derive(Debug, Clone, Default, Serialize, Deserialize, TS)]
pub struct ThermalSetText {
    pub cells: Vec<ThermalCellText>,
}

/// 熱の下書きの 1 つの編集。
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(tag = "cell", rename_all = "snake_case")]
pub enum ThermalCellText {
    /// 明示の結合（`a`・`b` は書いた向き。`value` が null なら取り消して既定の出どころへ戻す）
    Coupling {
        kind: ThermalCouplingKind,
        a: String,
        b: String,
        #[serde(default)]
        value: Option<String>,
    },
    /// 表面材の欄（書いた欄だけ。表面材は吸収率・放射率が両方そろうまで検査が注意を出す）
    Coating {
        id: String,
        #[serde(default)]
        label: Option<String>,
        #[serde(default)]
        absorptivity: Option<String>,
        #[serde(default)]
        emissivity: Option<String>,
    },
    /// 表面材を消す（面が使っていれば検査が拒む）
    CoatingRemove { id: String },
    /// 面（部品）の表面材の面積割合（`fraction` が null ならその表面材を面から外す）
    Surface {
        part: String,
        coating: String,
        #[serde(default)]
        fraction: Option<String>,
    },
    /// 面の表面材の割合をすべて消す（部品の吸収率・放射率に戻る）
    SurfaceRemove { part: String },
}

/// 熱の結合・表面の表の依頼。`ops` は保存していない編集（書き込まずに当てた設計で表を作る）。
#[derive(Debug, Clone, Default, Serialize, Deserialize, TS)]
#[serde(default)]
pub struct ThermalTablesRequest {
    /// 解析ケースの ID（省略は既定のケース）
    pub case: Option<String>,
    pub ops: Vec<fune_ops::Op>,
}

/// 熱の結合・表面の表。
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
pub struct ThermalTablesView {
    pub couplings: ThermalCouplingsTable,
    /// 表面材（ID 順）
    pub coatings: Vec<ThermalCoatingRow>,
    /// 外面（入力に載るもの、と表面材を書いたもの。部品の ID 順）
    pub surfaces: Vec<ThermalSurfaceRow>,
    /// 節点ごとの最低・最高の温度（全ケース。下書きの前後を並べて見せるのに使う）
    pub temperatures: Vec<ThermalNodeTemperature>,
}

/// 結合の表。解析が回らないとき（足りない入力・失敗）は `available` が偽で、行と節点は空。
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
pub struct ThermalCouplingsTable {
    pub available: bool,
    /// 結合の種類と表示名（並びは表の並び。追加の欄の選択肢にする）
    pub kinds: Vec<ThermalCouplingKindView>,
    /// 解析の節点（入力の順。追加する結合の両端の選択肢）
    pub nodes: Vec<String>,
    pub rows: Vec<ThermalCouplingRow>,
}

/// 結合の種類の表示名。
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
pub struct ThermalCouplingKindView {
    pub kind: ThermalCouplingKind,
    pub label: AnalysisLabel,
    /// 値の次元の名前（入力欄が単位を検査する先。伝導は `ThermalConductance`、放射は `Area`）
    pub dimension: String,
}

/// 節点の対 1 つの結合（解析が使った値）。
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
pub struct ThermalCouplingRow {
    pub kind: ThermalCouplingKind,
    /// 節点の対。明示の結合は設計に書いた向き（取り消しと書き換えの宛先）、ほかは ID の昇順
    pub a: String,
    pub b: String,
    /// 使われている値（明示は書いた単位のまま、ほかは自動の単位）
    pub value: QuantityView,
    pub source: ThermalCouplingSource,
    pub source_label: AnalysisLabel,
    /// 設計に書いた値か（取り消せる）
    pub explicit: bool,
    /// 接触面積から出した取付があるとき、その内訳
    pub contact: Option<ThermalContactRow>,
}

/// 接触面積から出した取付の内訳。
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
pub struct ThermalContactRow {
    pub area: QuantityView,
    pub mounts: u32,
    /// 同じ対で 1 取付あたりの値に戻した取付の数
    pub fixed_mounts: u32,
}

/// 表面材 1 つ。
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
pub struct ThermalCoatingRow {
    pub id: String,
    pub label: Option<String>,
    pub absorptivity: Option<QuantityView>,
    pub emissivity: Option<QuantityView>,
    /// この表面材を使う面（部品の ID）
    pub used_by: Vec<String>,
}

/// 外面 1 つ（部品）。表面材を書いていなければ `uniform` が部品の一様な吸収率・放射率。
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
pub struct ThermalSurfaceRow {
    pub part: String,
    /// 面が属する節点（入力に載るものだけ）
    pub node: Option<String>,
    pub area: Option<QuantityView>,
    pub uniform: Option<ThermalUniformOptics>,
    pub fractions: Vec<ThermalFractionRow>,
    /// 割合の合計（1 でなければ検査が注意を出し、解析は入力の誤りで止まる）
    pub total: QuantityView,
}

/// 面全体が一様な光学特性。
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
pub struct ThermalUniformOptics {
    pub absorptivity: QuantityView,
    pub emissivity: QuantityView,
}

/// 面の表面材 1 つの面積割合。
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
pub struct ThermalFractionRow {
    pub coating: String,
    pub fraction: QuantityView,
}

/// 節点の全ケースの最低・最高の温度。
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
pub struct ThermalNodeTemperature {
    pub node: String,
    pub min: QuantityView,
    pub max: QuantityView,
    pub verdict: Verdict,
}
