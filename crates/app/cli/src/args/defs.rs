//! 属性定義・種別定義のサブコマンド（`fune attr …`・`fune kind …`）。
//! 旗は `fune_ops::to_cli` が書き出す定義のコマンドとフィールドが 1 対 1。

use super::EditOpts;
use clap::{Args, Subcommand};

/// `fune attr …`（引数の構造体なので、列挙の大きさの差は気にしない）
#[derive(Debug, Subcommand)]
#[allow(clippy::large_enum_variant)]
pub enum AttrCmd {
    /// 属性定義の一覧
    Ls,
    /// 属性定義を足す（次元と許可単位を検査する）
    Add(AttrAddArgs),
    /// 属性定義を丸ごと置き換える（旗は `add` と同じ。プロジェクト独自の定義だけ）
    Replace(AttrAddArgs),
    /// 属性定義を消す（種別が挙げているか、部品・カタログ品が値を書いていれば拒む）
    Rm {
        id: String,
        #[command(flatten)]
        edit: EditOpts,
    },
}

/// `fune attr add|replace`
#[derive(Debug, Args)]
pub struct AttrAddArgs {
    pub id: String,
    #[arg(long)]
    pub label: Option<String>,
    /// 英語の表示名
    #[arg(long)]
    pub label_en: Option<String>,
    #[arg(long)]
    pub description: Option<String>,
    /// 値の型（scalar・vec3・quat・tensor3・enum・bool・text・range・table。既定 scalar）
    #[arg(long = "type", default_value = "scalar")]
    pub value_type: String,
    /// 状態ごとに値を持つか
    #[arg(long)]
    pub by_state: bool,
    #[arg(long)]
    pub dimension: Option<String>,
    /// 許可する単位（カンマ区切り。例: `"A*m^2"`）
    #[arg(long)]
    pub units: Option<String>,
    #[arg(long)]
    pub display_unit: Option<String>,
    /// 値域の下限（SI の数値。読むのは service）
    #[arg(long, allow_negative_numbers = true)]
    pub min: Option<String>,
    /// 値域の上限（SI の数値）
    #[arg(long, allow_negative_numbers = true)]
    pub max: Option<String>,
    /// 既定値（SI の数値）
    #[arg(long, allow_negative_numbers = true)]
    pub default: Option<String>,
    /// 列挙の語彙（カンマ区切り）
    #[arg(long)]
    pub symbols: Option<String>,
    /// どのバジェットに入るか（mass / power / data）
    #[arg(long)]
    pub budget: Option<String>,
    /// 方向ベクトル（単位ベクトルであるべきもの）
    #[arg(long)]
    pub unit_vector: bool,
    /// 画面で並べる系（basic・mass・power・thermal・aocs・comm・cdh・structure）
    #[arg(long)]
    pub section: Option<String>,
    #[command(flatten)]
    pub edit: EditOpts,
}

/// `fune kind …`（引数の構造体なので、列挙の大きさの差は気にしない）
#[derive(Debug, Subcommand)]
#[allow(clippy::large_enum_variant)]
pub enum KindCmd {
    /// 種別定義の一覧
    Ls,
    /// 種別定義を足す
    Add(KindAddArgs),
    /// 種別定義を丸ごと置き換える（旗は `add` と同じ。プロジェクト独自の定義だけ）
    Replace(KindAddArgs),
    /// 種別定義を消す（部品・カタログ品が使っているか、ほかの種別が継承していれば拒む）
    Rm {
        id: String,
        #[command(flatten)]
        edit: EditOpts,
    },
}

/// `fune kind add|replace`
#[derive(Debug, Args)]
pub struct KindAddArgs {
    pub id: String,
    #[arg(long)]
    pub label: Option<String>,
    /// 英語の表示名
    #[arg(long)]
    pub label_en: Option<String>,
    #[arg(long)]
    pub description: Option<String>,
    #[arg(long)]
    pub extends: Option<String>,
    #[arg(long)]
    pub role: Option<String>,
    /// 機器の状態（カンマ区切り。例: `"off,on"`）
    #[arg(long)]
    pub states: Option<String>,
    /// 必須の属性（カンマ区切り）
    #[arg(long)]
    pub required: Option<String>,
    /// 任意の属性（カンマ区切り）
    #[arg(long)]
    pub optional: Option<String>,
    /// ポートの雛形 `名前=power|data|rf`（繰り返し指定できる）
    #[arg(long = "port", value_name = "NAME=KIND")]
    pub ports: Vec<String>,
    #[command(flatten)]
    pub edit: EditOpts,
}
