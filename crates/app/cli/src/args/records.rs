//! 解析ケース・出典・ハーネス・マヌーバのサブコマンド（`fune case …`・`fune source …`・`fune harness …`・`fune maneuver …`）。
//! 綴りは `fune_ops::to_cli` と同じにする。`set` は無ければ作り、あれば丸ごと置き換える。

use super::EditOpts;
use clap::{Args, Subcommand};

/// `fune case …`
#[derive(Debug, Subcommand)]
pub enum CaseCmd {
    /// 解析ケースの一覧
    Ls,
    /// 解析ケースを書く
    Set(CaseArgs),
    /// 季節解析の代表日から解析ケースを作る
    FromEclipse {
        id: String,
        /// longest / beta_min
        #[arg(long, value_parser = ["longest", "beta_min"])]
        at: String,
        #[arg(long)]
        mode: Option<String>,
        #[command(flatten)]
        edit: EditOpts,
    },
    /// 解析ケースを消す
    Rm {
        id: String,
        #[command(flatten)]
        edit: EditOpts,
    },
}

/// `fune case set`
#[derive(Debug, Args)]
pub struct CaseArgs {
    pub id: String,
    #[arg(long)]
    pub label: Option<String>,
    #[arg(long)]
    pub description: Option<String>,
    /// 使うモード
    #[arg(long)]
    pub mode: Option<String>,
    /// 不確かさのどちら側を読むか（nominal / min / max）
    #[arg(long)]
    pub side: Option<String>,
    /// 熱の環境条件（`analysis.thermal` の `[case.<名前>]`）
    #[arg(long)]
    pub thermal_case: Option<String>,
    /// β 角（例: `"60 deg"`）
    #[arg(long, allow_hyphen_values = true)]
    pub beta: Option<String>,
    /// 時系列の周回数
    #[arg(long)]
    pub orbits: Option<u32>,
    #[command(flatten)]
    pub edit: EditOpts,
}

/// `fune source …`
#[derive(Debug, Subcommand)]
pub enum SourceCmd {
    /// 出典の一覧
    Ls,
    /// 出典を書く
    Set(Box<SourceArgs>),
    /// 出典を消す（値から参照されていれば拒む）
    Rm {
        id: String,
        #[command(flatten)]
        edit: EditOpts,
    },
}

/// `fune source set`
#[derive(Debug, Args)]
pub struct SourceArgs {
    pub id: String,
    /// placeholder / judgement / analysis / datasheet / measured
    #[arg(long)]
    pub kind: String,
    #[arg(long)]
    pub label: Option<String>,
    /// 参照先（URL・リポジトリ内のパス・文書番号）
    #[arg(long = "ref")]
    pub reference: Option<String>,
    /// 資料の版（例: `"Rev. C"`）
    #[arg(long)]
    pub revision: Option<String>,
    /// 資料を取得した日（`2026-10-15`）
    #[arg(long)]
    pub retrieved: Option<String>,
    /// 取得したファイルの SHA-256（64 桁の 16 進数）
    #[arg(long)]
    pub sha256: Option<String>,
    /// 配布の区分（public / on_request / restricted）
    #[arg(long)]
    pub distribution: Option<String>,
    /// 日付（試験・解析・判断をした日。`2026-10-15`）
    #[arg(long)]
    pub date: Option<String>,
    #[arg(long)]
    pub by: Option<String>,
    #[arg(long)]
    pub note: Option<String>,
    #[command(flatten)]
    pub edit: EditOpts,
}

/// `fune harness …`
#[derive(Debug, Subcommand)]
pub enum HarnessCmd {
    /// ハーネスの一覧
    Ls,
    /// ハーネス区間を書く
    Set(HarnessArgs),
    /// ハーネス区間を消す
    Rm {
        id: String,
        #[command(flatten)]
        edit: EditOpts,
    },
}

/// `fune harness set`
#[derive(Debug, Args)]
pub struct HarnessArgs {
    pub id: String,
    /// 一端のポート（`PCDU.OUT3`）
    #[arg(long)]
    pub from: String,
    /// もう一端のポート（`RW_X.PWR`）
    #[arg(long)]
    pub to: String,
    /// 通すネット（カンマ区切り）
    #[arg(long)]
    pub nets: Option<String>,
    /// 線材のカタログ品
    #[arg(long)]
    pub wire: Option<String>,
    #[arg(long)]
    pub cores: Option<u32>,
    /// 長さ（例: `"0.35 m"`）
    #[arg(long)]
    pub length: Option<String>,
    #[arg(long)]
    pub label: Option<String>,
    #[command(flatten)]
    pub edit: EditOpts,
}

/// `fune maneuver …`
#[derive(Debug, Subcommand)]
pub enum ManeuverCmd {
    /// マヌーバの一覧（実行の順）
    Ls,
    /// マヌーバを足す（同じ ID があれば拒む）
    Add(ManeuverArgs),
    /// マヌーバを書く（無ければ作り、あれば丸ごと置き換える）
    Set(ManeuverArgs),
    /// マヌーバを消す（後ろのマヌーバは消したマヌーバの前につなぎ直す）
    Rm {
        id: String,
        #[command(flatten)]
        edit: EditOpts,
    },
}

/// `fune maneuver add|set`（旗は `fune_service::dto::ManeuverText` と 1 対 1）
#[derive(Debug, Args)]
pub struct ManeuverArgs {
    pub id: String,
    /// insertion / station_keeping / unloading / deorbit / other
    #[arg(long)]
    pub category: String,
    /// impulsive / finite_burn / low_thrust / propagated
    #[arg(long)]
    pub method: String,
    #[arg(long)]
    pub label: Option<String>,
    /// スラスタの部品
    #[arg(long)]
    pub thruster: Option<String>,
    /// 前のマヌーバ
    #[arg(long)]
    pub after: Option<String>,
    /// バイエリプティック遷移の中間の遠地点の高度（例: `"20000 km"`）
    #[arg(long)]
    pub via_altitude: Option<String>,
    /// 目標の中心天体（例: `moon`・`mars`。天体間の遷移になり、高度はその天体から）
    #[arg(long)]
    pub body: Option<String>,
    /// 目標の円軌道の高度（例: `"600 km"`）
    #[arg(long)]
    pub altitude: Option<String>,
    /// 目標の近地点の高度
    #[arg(long, allow_hyphen_values = true)]
    pub perigee_altitude: Option<String>,
    /// 目標の遠地点の高度
    #[arg(long)]
    pub apogee_altitude: Option<String>,
    /// 目標の傾斜角（例: `"97.8 deg"`）
    #[arg(long)]
    pub inclination: Option<String>,
    /// 日照中だけ噴射する（true / false。粗い段階の解析では使わない）
    #[arg(long)]
    pub sunlit_only: Option<String>,
    /// 噴射するモード（粗い段階の解析では使わない）
    #[arg(long)]
    pub mode: Option<String>,
    /// デューティ（噴いてよい時間のうち噴く割合。propagated だけ）
    #[arg(long)]
    pub duty: Option<String>,
    /// 推力の向き（tangential / inertial。propagated だけ）
    #[arg(long)]
    pub steering: Option<String>,
    #[command(flatten)]
    pub edit: EditOpts,
}

/// `fune part point`（取付点）
#[derive(Debug, Args)]
pub struct PointArgs {
    pub part: String,
    pub point: String,
    /// 位置（例: `"30 0 5 mm"`）
    #[arg(long, allow_hyphen_values = true)]
    pub position: Option<String>,
    /// 姿勢のクォータニオン（`"1 0 0 0"`、w x y z）
    #[arg(long, allow_hyphen_values = true)]
    pub orientation: Option<String>,
    /// 取付点を消す
    #[arg(long, conflicts_with_all = ["position", "orientation"])]
    pub rm: bool,
    #[command(flatten)]
    pub edit: EditOpts,
}

/// `fune catalog …`（カタログ品。編集できるのはこのプロジェクトのカタログ品だけ。共有カタログは取得と更新）
#[derive(Debug, Subcommand)]
pub enum CatalogCmd {
    /// カタログ品の一覧
    Ls,
    /// 1 品の内容
    Show { id: String },
    /// 共有カタログ（`[[catalog]] git = …`）を取得して `.fune/` に置き、commit を catalog.lock に記録する
    Fetch {
        /// catalog.lock を書き換えない（lock が無い・fune.toml と違えば失敗する。CI 向け）
        #[arg(long)]
        locked: bool,
    },
    /// 共有カタログの lock を `rev` の今の commit に上げ、変わった品と値を出す
    Update {
        /// 共有カタログの名前（fune.toml の `[[catalog]]` の `name`）
        name: String,
    },
    /// カタログ品を足す（`--from-part` で部品の値を写す）
    Add {
        /// 出所付きの ID（例: `local/rw_30mNms`）
        id: String,
        #[arg(long)]
        kind: String,
        #[arg(long)]
        label: Option<String>,
        #[arg(long)]
        label_en: Option<String>,
        /// off_the_shelf / modified / new
        #[arg(long)]
        maturity: Option<String>,
        #[arg(long)]
        from_part: Option<String>,
        #[command(flatten)]
        edit: EditOpts,
    },
    /// 属性の値かフィールドを書く（例: `fune catalog set local/rw_30mNms mass "0.9 kg"`）
    Set {
        id: String,
        /// 属性（`mass`・`power.on`）かフィールド（`label`・`manufacturer` …）
        path: String,
        #[arg(allow_hyphen_values = true)]
        value: String,
        #[arg(long)]
        source: Option<String>,
        /// 資料の中の位置（例: `"p.12 表 3"`）
        #[arg(long)]
        at: Option<String>,
        #[arg(long)]
        note: Option<String>,
        #[arg(long, allow_hyphen_values = true)]
        min: Option<String>,
        #[arg(long, allow_hyphen_values = true)]
        max: Option<String>,
        #[command(flatten)]
        edit: EditOpts,
    },
    /// 属性の値かフィールドを消す
    Unset {
        id: String,
        path: String,
        #[command(flatten)]
        edit: EditOpts,
    },
    /// カタログ品を消す（参照されていれば拒む）
    Rm {
        id: String,
        #[command(flatten)]
        edit: EditOpts,
    },
    /// カタログ品の名前を変える（部品の catalog・candidates も書き換える）
    Rename {
        from: String,
        to: String,
        #[command(flatten)]
        edit: EditOpts,
    },
}
