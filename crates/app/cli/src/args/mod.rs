//! 引数の定義（clap）。コマンド体系は `fune <名詞> <動詞>` と、よく使うものの短縮。

mod defs;
mod nouns;
mod records;

pub use defs::{AttrAddArgs, AttrCmd, KindAddArgs, KindCmd};
pub use nouns::{
    ModeCmd, NetArgs, NetCmd, NetSetArgs, PartCmd, PlaceArgs, SetArgs, SettingCmd, ViewCmd,
};
pub use records::{
    CaseArgs, CaseCmd, CatalogCmd, HarnessArgs, HarnessCmd, PointArgs, SourceArgs, SourceCmd,
};

use clap::{Args, Parser, Subcommand, ValueEnum};
use std::path::PathBuf;

/// 衛星の設計データを CLI から読み書きし、解析を回す。
#[derive(Debug, Parser)]
#[command(name = "fune", version, about, propagate_version = true)]
pub struct Cli {
    #[command(flatten)]
    pub global: GlobalOpts,
    #[command(subcommand)]
    pub command: Command,
}

/// どのコマンドにも付けられるオプション。
#[derive(Debug, Args)]
pub struct GlobalOpts {
    /// プロジェクトの場所（fune.toml のあるディレクトリかその下）
    #[arg(
        short = 'C',
        long = "dir",
        global = true,
        default_value = ".",
        value_name = "DIR"
    )]
    pub dir: PathBuf,
    /// 機械向けの JSON で出力する
    #[arg(long, global = true)]
    pub json: bool,
    /// 色を付けるか（NO_COLOR と端末でない出力では auto は無色）
    #[arg(long, global = true, value_enum, default_value_t = ColorChoice::Auto)]
    pub color: ColorChoice,
}

/// 色の指定。
#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
pub enum ColorChoice {
    Auto,
    Always,
    Never,
}

/// 編集系のコマンドに共通のオプション。
#[derive(Debug, Args, Default)]
pub struct EditOpts {
    /// 書き込まずに、変更されるファイルの差分を表示する
    #[arg(long)]
    pub dry_run: bool,
    /// この revision から変わっていたら書き込まない（競合で終了コード 3）
    #[arg(long, value_name = "REV")]
    pub base_revision: Option<String>,
}

/// サブコマンド。
#[derive(Debug, Subcommand)]
pub enum Command {
    /// 新しいプロジェクトを作る
    Init { name: String },
    /// 設計データを検証する（エラーがあれば終了コード 1）
    Check {
        /// 警告もエラーとして扱う
        #[arg(long)]
        deny_warnings: bool,
    },
    /// GUI（ローカルの HTTP サーバー）を起動してブラウザで開く
    ///
    /// API は設計ファイルを書けるので、Host ヘッダーが許可リストに無い要求は 403（host_not_allowed）で拒む
    /// （DNS リバインディング対策）。許可するのは localhost・127.0.0.1・[::1]（ポートは問わない）、
    /// --host に 0.0.0.0 以外を指定したときはその IP、--allow-host で足した名前。
    /// 例（Tailscale 越し）: fune gui --host 0.0.0.0 --allow-host 100.64.0.1 --no-open
    Gui {
        #[arg(long, default_value_t = 8787)]
        port: u16,
        /// 待ち受けるアドレス（既定は 127.0.0.1。0.0.0.0 ですべてのインターフェース）
        #[arg(long, default_value = "127.0.0.1")]
        host: String,
        /// Host ヘッダーとして許可する名前か IP（繰り返し指定できる。例: 100.64.0.1、my-pc.tailnet.ts.net）
        #[arg(long = "allow-host", value_name = "NAME")]
        allow_host: Vec<String>,
        /// ブラウザを開かない
        #[arg(long)]
        no_open: bool,
    },
    /// プロジェクトの概要
    Status,
    /// 部品
    #[command(subcommand)]
    Part(PartCmd),
    /// 値を設定する（例: `fune set RW_X.mass "620 g" --source em_test`）
    Set(SetArgs),
    /// 値を表示する（例: `fune get RW_X.power --state on`）
    Get {
        path: String,
        #[arg(long)]
        state: Option<String>,
    },
    /// 上書きした値を消してカタログの値に戻す
    Unset {
        path: String,
        #[command(flatten)]
        edit: EditOpts,
    },
    /// 属性定義
    #[command(subcommand)]
    Attr(AttrCmd),
    /// 種別定義
    #[command(subcommand)]
    Kind(KindCmd),
    /// 運用モード
    #[command(subcommand)]
    Mode(ModeCmd),
    /// ネット（電気的な接続）
    #[command(subcommand)]
    Net(NetCmd),
    /// 設定（mission/・analysis/・budget/）
    #[command(subcommand)]
    Setting(SettingCmd),
    /// カタログ品（catalog/）
    #[command(subcommand)]
    Catalog(CatalogCmd),
    /// 解析ケース（cases/）
    #[command(subcommand)]
    Case(CaseCmd),
    /// 出典の台帳（sources/）
    #[command(subcommand)]
    Source(SourceCmd),
    /// ハーネス区間（harness/）
    #[command(subcommand)]
    Harness(HarnessCmd),
    /// 図の配置（系統図のノードの位置）
    #[command(subcommand)]
    View(ViewCmd),
    /// 単位
    #[command(subcommand)]
    Units(UnitsCmd),
    /// 設計ファイルを整形する（`fune fmt`、`fune fmt --check`）
    Fmt {
        /// 書き換えずに、整形が要るファイルを報告する（あれば終了コード 1）
        #[arg(long)]
        check: bool,
    },
    /// 解析（`fune analyze ls`、`fune analyze power.energy_balance`）
    Analyze(AnalyzeArgs),
    /// バジェット（mass / power / data）
    Budget {
        kind: String,
        /// 解析ケースの ID（cases/<ID>.toml）。電力はケースのモード、データ量はケースのモードだけで数える
        #[arg(long)]
        case: Option<String>,
        /// 保存していない編集（Op の配列の JSON ファイル、`-` で標準入力）を書き込まずに当てて数える
        #[arg(long)]
        ops: Option<PathBuf>,
    },
    /// 外部データ（DE440 の暦表）
    #[command(subcommand)]
    Data(DataCmd),
    /// Op の JSON（GUI と同じ形）を適用する
    Apply {
        /// Op の配列の JSON ファイル（`-` で標準入力）
        file: PathBuf,
        #[command(flatten)]
        edit: EditOpts,
    },
}

/// `fune units …`
#[derive(Debug, Subcommand)]
pub enum UnitsCmd {
    /// 単位の一覧
    Ls {
        #[arg(long)]
        dimension: Option<String>,
    },
    /// 入力文字列を解析する（例: `fune units parse "18 mN*m*s"`）
    Parse {
        input: String,
        #[arg(long)]
        dimension: Option<String>,
    },
    /// SI 値を表示用に整形する（例: `fune units format 2.213e10 --dimension Information`）
    Format {
        /// SI 値（複数可）
        #[arg(required = true, allow_negative_numbers = true)]
        values: Vec<f64>,
        #[arg(long)]
        dimension: String,
        /// 単位を指定する（例: `Mbit`）
        #[arg(long)]
        unit: Option<String>,
    },
}

/// `fune data …`
#[derive(Debug, Subcommand)]
pub enum DataCmd {
    /// キャッシュにあるデータの状態
    Status,
    /// データを取得してキャッシュに置く（例: `fune data fetch de440`。gh が要る）
    Fetch { id: String },
}

/// `fune analyze`
#[derive(Debug, Args)]
pub struct AnalyzeArgs {
    /// `ls` か解析 ID（例: `power.energy_balance`）
    pub target: String,
    /// 解析ケース
    #[arg(long)]
    pub case: Option<String>,
    /// 入力を丸ごと与える JSON ファイル（設計データから組み立てない）
    #[arg(long)]
    pub input: Option<PathBuf>,
    /// 保存していない編集（Op の配列の JSON ファイル、`-` で標準入力）を書き込まずに当てて解析する
    #[arg(long)]
    pub ops: Option<PathBuf>,
}
