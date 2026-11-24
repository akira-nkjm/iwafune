//! 不足入力の直し方の候補の型。**何をすれば足りない入力が埋まるか（操作の種類と絞り込みの条件）を、構造で持つ。**
//!
//! 候補を決めるのは `fune_service::fixes` だけ（設計の中の候補の部品などを引くため）。ここは型だけを持つ。
//! 受け取る側（CLI・Web）は対象の文字列を読み直さず、この構造を見る。`command` は同じ直しを CLI で打つ文（値の所は
//! `<value>` のような山括弧の置き換え欄）で、CLI の表示とコピーに使う。どの候補も、実行すると既存の Op（`fune_ops::Op`）になる。

use fune_model::Target;
use fune_model::path::ParamPath;
use fune_model::role::Role;
use serde::{Deserialize, Serialize};
use ts_rs::TS;

/// 値を入れる先が、部品の選んだカタログ品でもあるときの、品の側の書き先。
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize, TS)]
pub struct CatalogFix {
    /// カタログ品の参照（`local/rw_10mNms`）
    pub item: String,
    /// 品の中のパス（`max_momentum`・`power.on`。`SetText.item` と一緒に使う）
    pub path: String,
    /// 品へ書く CLI のコマンド
    pub command: String,
}

/// 選べる値 1 つ（値の綴りと日英の表示名）。候補の絞り込みはサービスが決め、受け取る側は並べるだけ。
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize, TS)]
pub struct Choice {
    /// 入れる値の綴り（`local/lv_alpha`・`ECI` など。そのまま `fune set` に渡せる）
    pub value: String,
    /// 日本語の表示名（無ければ `value`）
    pub label: String,
    /// 英語の表示名（無ければ None）
    pub label_en: Option<String>,
}

/// 欄が指すものの種類（[`Fix::PickRef`]）。
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize, TS)]
#[serde(rename_all = "snake_case")]
pub enum RefKind {
    /// 設計の部品
    Part,
    /// カタログの品（`<カタログ>/<ID>`）
    CatalogItem,
    /// 通信の回線（`analysis.comm` の回線の名前）
    CommLink,
    /// 部品の配置の形態（`configurations/<ID>.toml`）
    Configuration,
}

/// 足りない入力を一覧に足す先の一覧（「何を足すか」）。
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize, TS)]
#[serde(rename_all = "snake_case")]
pub enum ListKind {
    /// 運用モード（`modes/<ID>.toml`）
    Mode,
    /// マヌーバ（`maneuvers/<ID>.toml`）
    Maneuver,
    /// 地上局（`conditions/ground/<ID>.toml`）
    GroundStation,
    /// 通信の回線（`analysis/comm.toml` の `[link.<名前>]`）
    CommLink,
    /// 解析ケース（`cases/<ID>.toml`）
    Case,
}

/// 足りない入力の直し方の候補 1 つ。
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize, TS)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum Fix {
    /// 部品の属性・部品のフィールド・設定に値を入れる。
    SetValue {
        /// 書く先（`fune set` と `POST /ops/from_text` に渡すパス。対象の表示と同じ綴り）
        path: String,
        /// 書く先の構造
        target: ParamPath,
        /// 期待する次元名（`Mass`。量でなければ None）
        dimension: Option<String>,
        /// 部品の上書きとして書く CLI のコマンド（`fune set RW_X.mass "<value>"`）
        command: String,
        /// 部品がカタログ品を選んでいるときの、品の側へ書く代わりの書き先
        catalog: Option<CatalogFix>,
        /// 受け付ける入力の例（量なら `1 kg`。次元から `sat_units::format::input_example` が作る）
        example: Option<String>,
        /// 値が閉じた語彙（属性の列挙・真偽）のときの選べる値（空なら自由入力）
        choices: Vec<Choice>,
    },
    /// 役割のどれかの部品の属性に値を入れる（その役割の部品はあるが、値が無い）。
    SetValueOnPart {
        /// 属性の ID
        attr: String,
        dimension: Option<String>,
        /// 候補の部品（その役割で、値が無いもの。役割が空なら設計のすべての部品）
        parts: Vec<String>,
        /// `fune set <部品>.<属性> "<value>"`
        command: String,
        /// 受け付ける入力の例（[`Fix::SetValue`] の `example` と同じ）
        example: Option<String>,
        /// 値が閉じた語彙のときの選べる値（空なら自由入力）
        choices: Vec<Choice>,
    },
    /// その役割の部品が 1 つも無い: 種別（と、その種別のカタログ品）を選んで部品を足す。
    AddPart {
        roles: Vec<Role>,
        /// その役割を持つ種別の ID
        kinds: Vec<String>,
        /// その種別のカタログ品の参照
        catalog_items: Vec<String>,
        /// `fune part add <ID> --kind <kind>`
        command: String,
    },
    /// その役割の部品が 1 つも無い: 既にある部品の種別をその役割のものに替える。
    AssignPart {
        roles: Vec<Role>,
        /// その役割を持つ種別の ID
        kinds: Vec<String>,
        /// 候補の部品（その役割でないもの）
        parts: Vec<String>,
        /// `fune set <部品>.kind <kind>`
        command: String,
    },
    /// 別のもの（部品・カタログ品・通信の回線）を指す欄が空か、指したものが無い: 設計にあるものから選んで入れる。
    PickRef {
        /// 欄の対象（マヌーバの `thruster`・解析設定の回線の `part`・`antenna`・`conditions.launch.launcher`）
        target: Target,
        /// 欄の表示パス（`SetText.path` に使う）
        path: String,
        /// 指すものの種類（入れる値の意味と、選べるものが無いときの案内の文を分ける）
        what: RefKind,
        /// 選べる部品の役割（`what` が部品のときだけ。それ以外は空）
        roles: Vec<Role>,
        /// 選べるカタログ品の種別（`what` がカタログ品のときだけ。継承を含めた子の種別も選べる。それ以外は空）
        kinds: Vec<String>,
        /// 選べるもの（部品の ID・カタログの参照 `<カタログ>/<ID>`・回線の名前と、表示名）。空なら選べるものが無い
        choices: Vec<Choice>,
        /// 欄に入れる CLI のコマンド（`fune maneuver set …`・`fune set …`）
        command: String,
        /// 選べるものが 1 つも無いときに、それを足す CLI のコマンド（足し方が無ければ None）
        add_command: Option<String>,
    },
    /// 一覧の項目（モード・マヌーバ・地上局など）を足す。
    AddListItem {
        list: ListKind,
        /// その一覧に足す CLI のコマンド（必須の欄は置き換え欄）
        command: String,
    },
}

impl Fix {
    /// 直しの CLI のコマンド（`fune analyze` の表示に使う）。
    pub fn command(&self) -> &str {
        match self {
            Fix::SetValue { command, .. }
            | Fix::SetValueOnPart { command, .. }
            | Fix::AddPart { command, .. }
            | Fix::AssignPart { command, .. }
            | Fix::PickRef { command, .. }
            | Fix::AddListItem { command, .. } => command,
        }
    }
}
