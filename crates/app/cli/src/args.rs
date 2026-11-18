//! 引数の定義（clap）。コマンド体系は `fune <名詞> <動詞>` と、よく使うものの短縮。

mod defs;
mod nouns;
mod records;

pub use defs::{AttrAddArgs, AttrCmd, KindAddArgs, KindCmd};
pub use nouns::{
    ModeCmd, NetArgs, NetCmd, NetSetArgs, PartCmd, PlaceArgs, SequenceCmd, SetArgs, SettingCmd,
    ViewCmd,
};
pub use records::{
    CaseArgs, CaseCmd, CatalogCmd, CatalogSourceCmd, HarnessArgs, HarnessCmd, ManeuverArgs,
    ManeuverCmd, PointArgs, SourceArgs, SourceCmd,
};

use clap::{Args, Parser, Subcommand, ValueEnum};
use std::path::PathBuf;

/// Read and write satellite design data from the CLI and run analyses.
#[derive(Debug, Parser)]
#[command(name = "fune", version, about, propagate_version = true)]
pub struct Cli {
    #[command(flatten)]
    pub global: GlobalOpts,
    #[command(subcommand)]
    pub command: Command,
}

/// Options available on every command.
#[derive(Debug, Args)]
pub struct GlobalOpts {
    /// Project location (the directory with fune.toml, or one below it)
    #[arg(
        short = 'C',
        long = "dir",
        global = true,
        default_value = ".",
        value_name = "DIR"
    )]
    pub dir: PathBuf,
    /// Print machine-readable JSON
    #[arg(long, global = true)]
    pub json: bool,
    /// Whether to use colour (auto is plain with NO_COLOR or when output is not a terminal)
    #[arg(long, global = true, value_enum, default_value_t = ColorChoice::Auto)]
    pub color: ColorChoice,
}

/// Colour choice.
#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
pub enum ColorChoice {
    Auto,
    Always,
    Never,
}

/// Options shared by the editing commands.
#[derive(Debug, Args, Default)]
pub struct EditOpts {
    /// Show the diff of the files that would change, without writing
    #[arg(long)]
    pub dry_run: bool,
    /// Do not write if the design has changed since this revision (exit code 3 on conflict)
    #[arg(long, value_name = "REV")]
    pub base_revision: Option<String>,
}

/// Subcommands.
#[derive(Debug, Subcommand)]
pub enum Command {
    /// Create a new project
    Init { name: String },
    /// Check the design data (exit code 1 on errors)
    Check {
        /// Treat warnings as errors
        #[arg(long)]
        deny_warnings: bool,
    },
    /// Start the GUI (a local HTTP server) and open it in the browser
    ///
    /// The API can write design files, so requests whose Host header is not on the allow list are rejected with 403 (host_not_allowed)
    /// (a DNS rebinding defence). Allowed: localhost, 127.0.0.1, [::1] (any port),
    /// the IP given to --host unless it is 0.0.0.0, and names added with --allow-host.
    /// Example (over a VPN): fune gui --host 0.0.0.0 --allow-host 100.64.0.1 --no-open
    Gui {
        /// Port to listen on (default 3000; when it is in use and --port is not given, the next free port is used)
        #[arg(long)]
        port: Option<u16>,
        /// Address to listen on (default 127.0.0.1; 0.0.0.0 for all interfaces)
        #[arg(long, default_value = "127.0.0.1")]
        host: String,
        /// Name or IP to allow as the Host header (repeatable; e.g. 100.64.0.1, my-pc.example.net)
        #[arg(long = "allow-host", value_name = "NAME")]
        allow_host: Vec<String>,
        /// Do not open the browser
        #[arg(long)]
        no_open: bool,
    },
    /// Project overview
    Status,
    /// Parts
    #[command(subcommand)]
    Part(PartCmd),
    /// Set a value (e.g. `fune set RW_X.mass "620 g" --source em_test`)
    Set(SetArgs),
    /// Show a value (e.g. `fune get RW_X.power --state on`)
    Get {
        path: String,
        #[arg(long)]
        state: Option<String>,
    },
    /// Remove an overridden value and revert to the catalog value
    Unset {
        path: String,
        #[command(flatten)]
        edit: EditOpts,
    },
    /// Attribute definitions
    #[command(subcommand)]
    Attr(AttrCmd),
    /// Kind definitions
    #[command(subcommand)]
    Kind(KindCmd),
    /// Operating modes
    #[command(subcommand)]
    Mode(ModeCmd),
    /// Named mode sequences of the power analysis (analysis/power.toml)
    #[command(subcommand)]
    Sequence(SequenceCmd),
    /// Nets (electrical connections)
    #[command(subcommand)]
    Net(NetCmd),
    /// Settings (conditions/, analysis/, budget/)
    #[command(subcommand)]
    Setting(SettingCmd),
    /// Catalog items (catalog/)
    #[command(subcommand)]
    Catalog(CatalogCmd),
    /// Analysis cases (cases/)
    #[command(subcommand)]
    Case(CaseCmd),
    /// Source register (sources/)
    #[command(subcommand)]
    Source(SourceCmd),
    /// Harness segments (harness/)
    #[command(subcommand)]
    Harness(HarnessCmd),
    /// Maneuvers (maneuvers/)
    #[command(subcommand)]
    Maneuver(ManeuverCmd),
    /// Diagram layout (node positions of the system diagrams)
    #[command(subcommand)]
    View(ViewCmd),
    /// Units
    #[command(subcommand)]
    Units(UnitsCmd),
    /// Format the design files (`fune fmt`, `fune fmt --check`)
    Fmt {
        /// Report the files that need formatting without rewriting them (exit code 1 if any)
        #[arg(long)]
        check: bool,
    },
    /// List the edit history that `fune undo` and `fune redo` use (oldest first)
    History,
    /// Undo the last edit (restores the files the edit changed; refused if they changed since)
    Undo,
    /// Redo the edit that was undone
    Redo,
    /// Run an analysis (`fune analyze ls`, `fune analyze power.energy_balance`)
    Analyze(AnalyzeArgs),
    /// The power page: per-mode power budgets and the analyses behind them (exit code 1 on a violation or a failed analysis)
    PowerPage {
        /// Analysis case ID (cases/<ID>.toml; default is resolved as in `fune analyze`)
        #[arg(long)]
        case: Option<String>,
    },
    /// Budgets (mass / power / data)
    Budget {
        kind: String,
        /// Analysis case ID (cases/<ID>.toml); power and data volume are counted from the case's mode only
        #[arg(long)]
        case: Option<String>,
        /// Apply unsaved edits (a JSON file of Ops, or `-` for stdin) without writing, then count
        #[arg(long)]
        ops: Option<PathBuf>,
    },
    /// External data (DE440 ephemeris)
    #[command(subcommand)]
    Data(DataCmd),
    /// Replace this fune binary with the latest release (or a given version)
    ///
    /// Downloads the release archive from GitHub, verifies its SHA-256, checks that the new binary reports the expected
    /// version, and swaps it in place of the running binary. Needs curl and tar. Works outside a project.
    #[command(disable_version_flag = true)]
    SelfUpdate {
        /// Only report the current and latest versions; change nothing
        #[arg(long)]
        check: bool,
        /// Install exactly this version (major.minor.patch), even if it is older or the same as the current one
        #[arg(long, value_name = "VERSION")]
        version: Option<String>,
    },
    /// Apply Op JSON (the same form the GUI uses)
    Apply {
        /// JSON file of an array of Ops (`-` for stdin)
        file: PathBuf,
        #[command(flatten)]
        edit: EditOpts,
    },
}

/// `fune units …`
#[derive(Debug, Subcommand)]
pub enum UnitsCmd {
    /// List units
    Ls {
        #[arg(long)]
        dimension: Option<String>,
    },
    /// Parse an input string (e.g. `fune units parse "18 mN*m*s"`)
    Parse {
        input: String,
        #[arg(long)]
        dimension: Option<String>,
    },
    /// Format SI values for display (e.g. `fune units format 2.213e10 --dimension Information`)
    Format {
        /// SI values (one or more)
        #[arg(required = true, allow_negative_numbers = true)]
        values: Vec<f64>,
        #[arg(long)]
        dimension: String,
        /// Unit to use (e.g. `Mbit`)
        #[arg(long)]
        unit: Option<String>,
    },
}

/// `fune data …`
#[derive(Debug, Subcommand)]
pub enum DataCmd {
    /// State of the cached data
    Status,
    /// Fetch data into the cache (e.g. `fune data fetch de440` (needs gh), `fune data fetch space_weather` (needs curl))
    Fetch { id: String },
    /// Look up space weather (F10.7, Ap) for a date (e.g. `fune data space-weather 2003-10-29`)
    SpaceWeather {
        /// ISO 8601 epoch (a date alone is fine)
        date: String,
        /// Solar activity to use outside the table range (low, moderate, high; default moderate)
        #[arg(long)]
        activity: Option<String>,
    },
}

/// `fune analyze`
#[derive(Debug, Args)]
pub struct AnalyzeArgs {
    /// `ls` or an analysis ID (e.g. `power.energy_balance`)
    pub target: String,
    /// Analysis case
    #[arg(long)]
    pub case: Option<String>,
    /// JSON file that supplies the whole input (not built from the design data)
    #[arg(long)]
    pub input: Option<PathBuf>,
    /// Apply unsaved edits (a JSON file of Ops, or `-` for stdin) without writing, then analyze
    #[arg(long)]
    pub ops: Option<PathBuf>,
}
