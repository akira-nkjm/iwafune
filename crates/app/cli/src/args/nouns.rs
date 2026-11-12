//! 名詞ごとのサブコマンド（`fune part …`・`fune mode …` など）と `fune set`。
//! 綴りは `fune_ops::to_cli` が出すコマンドと同じにする（GUI の「CLI としてコピー」をそのまま貼れるように）。

use super::EditOpts;
use clap::{Args, Subcommand};

/// `fune set`
#[derive(Debug, Args)]
pub struct SetArgs {
    /// パス（`RW_X.mass`・`RW_X.power.on`・`RW_X.label`・`analysis.power.dod_max`）
    pub path: String,
    /// 値（例: `"620 g"`・`"-20 .. 60 degC"`・`"0 0 1"`）
    #[arg(allow_hyphen_values = true)]
    pub value: String,
    /// 出典 ID（sources/）
    #[arg(long)]
    pub source: Option<String>,
    #[arg(long)]
    pub note: Option<String>,
    /// 不確かさの下限（例: `"1 W"`）
    #[arg(long, allow_hyphen_values = true)]
    pub min: Option<String>,
    /// 不確かさの上限
    #[arg(long, allow_hyphen_values = true)]
    pub max: Option<String>,
    #[command(flatten)]
    pub edit: EditOpts,
}

/// `fune part …`
#[derive(Debug, Subcommand)]
pub enum PartCmd {
    /// 部品の一覧
    Ls {
        #[arg(long)]
        group: Option<String>,
        #[arg(long)]
        kind: Option<String>,
    },
    /// 1 部品の全情報
    Show {
        id: String,
        /// 保存していない編集（Op の配列の JSON ファイル、`-` で標準入力）を書き込まずに当てて表示する
        #[arg(long)]
        ops: Option<std::path::PathBuf>,
    },
    /// 部品を追加する（`--catalog` か `--kind`）
    Add {
        id: String,
        #[arg(long)]
        catalog: Option<String>,
        #[arg(long)]
        kind: Option<String>,
        /// 置く系（または組立品）。無ければ parts/ の直下
        #[arg(long)]
        group: Option<String>,
        #[arg(long)]
        count: Option<u32>,
        #[arg(long)]
        label: Option<String>,
        /// 各モードに足す行の状態（既定は off）
        #[arg(long)]
        mode_state: Option<String>,
        #[command(flatten)]
        edit: EditOpts,
    },
    /// 部品を消す
    Rm {
        id: String,
        #[command(flatten)]
        edit: EditOpts,
    },
    /// 部品を別の系・組立品に移す（`--root` で parts/ の直下）
    Mv {
        id: String,
        #[arg(required_unless_present = "root")]
        group: Option<String>,
        #[arg(long, conflicts_with = "group")]
        root: bool,
        #[command(flatten)]
        edit: EditOpts,
    },
    /// 部品の名前を変える（参照も書き換える）
    Rename {
        id: String,
        to: String,
        #[command(flatten)]
        edit: EditOpts,
    },
    /// 入れ子のフォルダの名前を変える（図の配置の参照も書き換える。最上位は系 ID のみ）
    RenameGroup {
        id: String,
        to: String,
        #[command(flatten)]
        edit: EditOpts,
    },
    /// 取付先と相対位置・姿勢を決める
    Place(PlaceArgs),
    /// 取付点（ほかの部品を付ける点）を書く・消す
    Point(super::records::PointArgs),
}

/// `fune part place`
#[derive(Debug, Args)]
pub struct PlaceArgs {
    pub id: String,
    /// 取付先の部品
    #[arg(long, required_unless_present_any = ["root", "body_pos"])]
    pub on: Option<String>,
    /// 取付点
    #[arg(long)]
    pub point: Option<String>,
    /// 取付を外して機体の根にする
    #[arg(long, conflicts_with_all = ["on", "point"])]
    pub root: bool,
    /// 位置（例: `"0 0 12 mm"`）
    #[arg(long, allow_hyphen_values = true)]
    pub position: Option<String>,
    /// 姿勢のクォータニオン（`"1 0 0 0"`、w x y z）
    #[arg(long, allow_hyphen_values = true)]
    pub orientation: Option<String>,
    /// 機体座標での位置（例: `"30 0 -100 mm"`）。取付先・取付点はそのままで、相対位置・姿勢に直して書く
    #[arg(long, allow_hyphen_values = true, conflicts_with_all = ["on", "point", "root", "position", "orientation"])]
    pub body_pos: Option<String>,
    /// 機体座標での姿勢（部品 → 機体、`"w x y z"`）。`--body-pos` と使う
    #[arg(long, allow_hyphen_values = true, requires = "body_pos")]
    pub body_rot: Option<String>,
    #[command(flatten)]
    pub edit: EditOpts,
}

/// `fune mode …`
#[derive(Debug, Subcommand)]
pub enum ModeCmd {
    /// モードの一覧
    Ls,
    /// 部品 × モードの表
    Table,
    /// モードを足す（`--from` でほかのモードの表を写す）
    Add {
        id: String,
        #[arg(long)]
        label: Option<String>,
        #[arg(long)]
        description: Option<String>,
        #[arg(long)]
        from: Option<String>,
        #[command(flatten)]
        edit: EditOpts,
    },
    /// モードを消す
    Rm {
        id: String,
        #[command(flatten)]
        edit: EditOpts,
    },
    /// モードの名前を変える
    Rename {
        id: String,
        to: String,
        #[command(flatten)]
        edit: EditOpts,
    },
    /// モードの最長継続時間を書く（例: `fune mode duration NOMINAL --max-duration "90 min"`。旗を省くと消す）
    Duration {
        mode: String,
        #[arg(long)]
        max_duration: Option<String>,
        #[command(flatten)]
        edit: EditOpts,
    },
    /// モードでの部品の状態を決める（例: `fune mode set NOMINAL TRX rx --duty tx=0.1`）
    Set {
        mode: String,
        part: String,
        state: String,
        /// ほかの状態の時間割合（`状態=割合`、カンマ区切り）
        #[arg(long)]
        duty: Option<String>,
        #[arg(long)]
        active: Option<String>,
        #[arg(long)]
        spare_state: Option<String>,
        #[arg(long, default_value_t = false, action = clap::ArgAction::Set, num_args = 0..=1, default_missing_value = "true")]
        eclipse: bool,
        #[command(flatten)]
        edit: EditOpts,
    },
    /// モードの表から部品の行を消す
    Unset {
        mode: String,
        part: String,
        #[arg(long, default_value_t = false, action = clap::ArgAction::Set, num_args = 0..=1, default_missing_value = "true")]
        eclipse: bool,
        #[command(flatten)]
        edit: EditOpts,
    },
}

/// `fune net add|set` の引数。
#[derive(Debug, Args)]
pub struct NetArgs {
    pub id: String,
    /// power / data / rf
    #[arg(long)]
    pub kind: String,
    #[arg(long)]
    pub label: Option<String>,
    /// 供給元のポート（`PCDU.OUT3`）
    #[arg(long)]
    pub source: Option<String>,
    /// レール電圧（例: `"12 V"`）
    #[arg(long)]
    pub rail: Option<String>,
    /// 電流制限（例: `"2 A"`）
    #[arg(long)]
    pub limit: Option<String>,
    #[arg(long)]
    pub protocol: Option<String>,
    /// ビットレート（例: `"1 Mbit/s"`）
    #[arg(long)]
    pub bitrate: Option<String>,
    /// バスのマスター（`OBC.CAN1`）
    #[arg(long)]
    pub master: Option<String>,
    #[command(flatten)]
    pub edit: EditOpts,
}

/// `fune net set` の引数（指定した旗ごとに 1 つ書く）。
#[derive(Debug, Args)]
pub struct NetSetArgs {
    pub id: String,
    /// power / data / rf
    #[arg(long)]
    pub kind: Option<String>,
    #[arg(long)]
    pub label: Option<String>,
    #[arg(long)]
    pub source: Option<String>,
    #[arg(long, allow_hyphen_values = true)]
    pub rail: Option<String>,
    #[arg(long, allow_hyphen_values = true)]
    pub limit: Option<String>,
    #[arg(long)]
    pub protocol: Option<String>,
    #[arg(long)]
    pub bitrate: Option<String>,
    #[arg(long)]
    pub master: Option<String>,
    #[command(flatten)]
    pub edit: EditOpts,
}

/// `fune net …`
#[derive(Debug, Subcommand)]
pub enum NetCmd {
    /// ネットの一覧
    Ls,
    /// ネットを足す
    Add(NetArgs),
    /// ネットのフィールドを書く（指定した旗だけ。例: `fune net set P5V --rail "5.1 V"`）
    Set(NetSetArgs),
    /// ネットのフィールドを消す（例: `fune net unset P5V limit`）
    Unset {
        id: String,
        /// label・source・rail・limit・protocol・bitrate・master
        field: String,
        #[command(flatten)]
        edit: EditOpts,
    },
    /// ネットの性質を丸ごと置き換える（書かなかった旗は消える）
    Replace(NetArgs),
    /// ネットを消す
    Rm {
        id: String,
        #[command(flatten)]
        edit: EditOpts,
    },
    /// 部品のポートをネットにつなぐ（例: `fune net connect RW_X.PWR P12V_AOCS`）
    Connect {
        port: String,
        net: String,
        #[command(flatten)]
        edit: EditOpts,
    },
    /// 部品のポートをネットから外す
    Disconnect {
        port: String,
        #[command(flatten)]
        edit: EditOpts,
    },
}

/// `fune setting …`
#[derive(Debug, Subcommand)]
pub enum SettingCmd {
    /// 設定の一覧
    Ls,
    /// 設定を変える（例: `fune setting set analysis.power.dod_max 0.3`）
    Set {
        key: String,
        #[arg(allow_hyphen_values = true)]
        value: String,
        #[command(flatten)]
        edit: EditOpts,
    },
    /// 設定を消す
    Unset {
        key: String,
        #[command(flatten)]
        edit: EditOpts,
    },
}

/// `fune view …`（系統図などの配置。見た目だけ）
#[derive(Debug, Subcommand)]
pub enum ViewCmd {
    /// 線の経路を手で決める（例: `fune view edge-set electrical "P5V:PCDU.OUT2>CAM.PWR" --points "100,80 400,80 400,200"`）
    EdgeSet {
        view: String,
        /// 線を指すキー（`ネット:始点の端子>終点の端子`。docs/model.md §10.4）
        key: String,
        /// 折れ線の点（`x,y` を空白で区切る。隣り合う点は水平か垂直）
        #[arg(long, allow_hyphen_values = true)]
        points: String,
        /// 始点の端子を出す側（left / right / top / bottom）
        #[arg(long)]
        source_side: Option<fune_service::dto::ViewPortSide>,
        /// 終点の端子を出す側
        #[arg(long)]
        target_side: Option<fune_service::dto::ViewPortSide>,
        /// 始点の端子の位置（その側の左か上の角からの距離 [px]。省略なら端子の並びで決まる位置）
        #[arg(long)]
        source_offset: Option<i32>,
        /// 終点の端子の位置
        #[arg(long)]
        target_offset: Option<i32>,
        #[command(flatten)]
        edit: EditOpts,
    },
    /// 線の経路を自動配置に戻す
    EdgeReset {
        view: String,
        key: String,
        #[command(flatten)]
        edit: EditOpts,
    },
    /// 図の手で決めた位置と線の経路をすべて消し、自動配置に戻す
    Reset {
        view: String,
        #[command(flatten)]
        edit: EditOpts,
    },
    /// 図のノードの位置の一覧
    Ls { view: String },
    /// ノードの位置（と大きさ）を決める（例: `fune view set electrical RW_X --pos 420,120`）
    Set {
        view: String,
        id: String,
        /// 位置 `X,Y`（整数）
        #[arg(long, allow_hyphen_values = true)]
        pos: String,
        /// 大きさ `W,H`（整数）
        #[arg(long)]
        size: Option<String>,
        #[command(flatten)]
        edit: EditOpts,
    },
    /// ノードを図から消す
    Rm {
        view: String,
        id: String,
        #[command(flatten)]
        edit: EditOpts,
    },
}
