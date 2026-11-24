//! 名詞ごとのサブコマンド（`fune part …`・`fune mode …` など）と `fune set`。
//! 綴りは `fune_ops::to_cli` が出すコマンドと同じにする（GUI の「CLI としてコピー」をそのまま貼れるように）。

use super::EditOpts;
use clap::{Args, Subcommand};

/// `fune set`
#[derive(Debug, Args)]
pub struct SetArgs {
    /// Path (`RW_X.mass`, `RW_X.power.on`, `RW_X.label`, `analysis.power.dod_max`, or one field of an analysis case: `cases.<ID>.<field>`
    /// with label, description, mode, phase, side, thermal_case, beta, orbits, sun_distance or epoch)
    pub path: String,
    /// Value (e.g. `"620 g"`, `"-20 .. 60 degC"`, `"0 0 1"`). A list setting (`analysis.thermal.nodes.<NODE>`, `analysis.comm.stations`)
    /// takes comma-separated words (`"BAT_A, BAT_B"`) or a TOML array of strings (`'["BAT_A", "BAT_B"]'`)
    #[arg(allow_hyphen_values = true)]
    pub value: String,
    /// Source ID (sources/)
    #[arg(long)]
    pub source: Option<String>,
    /// Location in the document (e.g. `"p.12 table 3"`)
    #[arg(long)]
    pub at: Option<String>,
    #[arg(long)]
    pub note: Option<String>,
    /// Lower bound of the uncertainty (e.g. `"1 W"`)
    #[arg(long, allow_hyphen_values = true)]
    pub min: Option<String>,
    /// Upper bound of the uncertainty
    #[arg(long, allow_hyphen_values = true)]
    pub max: Option<String>,
    #[command(flatten)]
    pub edit: EditOpts,
}

/// `fune part …`
#[derive(Debug, Subcommand)]
pub enum PartCmd {
    /// List parts
    Ls {
        #[arg(long)]
        group: Option<String>,
        #[arg(long)]
        kind: Option<String>,
        /// Poses (`mount`, `body` in `--json`) as they are in this configuration (default: the base placement)
        #[arg(long)]
        configuration: Option<String>,
    },
    /// Show everything about one part
    Show {
        id: String,
        /// Poses as they are in this configuration (default: the base placement)
        #[arg(long)]
        configuration: Option<String>,
        /// Apply unsaved edits (a JSON file of Ops, or `-` for stdin) without writing, then show
        #[arg(long)]
        ops: Option<std::path::PathBuf>,
    },
    /// Add a part (`--catalog` or `--kind`)
    Add {
        id: String,
        #[arg(long)]
        catalog: Option<String>,
        #[arg(long)]
        kind: Option<String>,
        /// Subsystem, group or assembly to place it in; directly under parts/ if omitted.
        /// `aocs/RW` puts it in the group RW under aocs (the group is created if it does not exist)
        #[arg(long)]
        group: Option<String>,
        #[arg(long)]
        count: Option<u32>,
        #[arg(long)]
        label: Option<String>,
        /// State of the row added to each mode (default off)
        #[arg(long)]
        mode_state: Option<String>,
        #[command(flatten)]
        edit: EditOpts,
    },
    /// Remove a part
    Rm {
        id: String,
        #[command(flatten)]
        edit: EditOpts,
    },
    /// Move a part to another subsystem, group or assembly (`--root` for directly under parts/).
    /// `aocs/RW` moves it into the group RW under aocs, creating the group if needed; moving the last part out removes the empty group
    /// (move parts back to the subsystem with `fune part mv <id> aocs`)
    Mv {
        id: String,
        #[arg(required_unless_present = "root")]
        group: Option<String>,
        #[arg(long, conflicts_with = "group")]
        root: bool,
        #[command(flatten)]
        edit: EditOpts,
    },
    /// Rename a part (references are rewritten too)
    Rename {
        id: String,
        to: String,
        #[command(flatten)]
        edit: EditOpts,
    },
    /// Rename a nested folder (diagram layout references are rewritten too; at the top level, subsystem IDs only)
    RenameGroup {
        id: String,
        to: String,
        #[command(flatten)]
        edit: EditOpts,
    },
    /// Set the mount target and the relative position and attitude
    Place(PlaceArgs),
    /// Write or remove a mount point (where other parts attach)
    Point(super::records::PointArgs),
}

/// `fune part place`
#[derive(Debug, Args)]
pub struct PlaceArgs {
    pub id: String,
    /// Part to mount on
    #[arg(long, required_unless_present_any = ["root", "body_pos", "configuration"])]
    pub on: Option<String>,
    /// Mount point
    #[arg(long)]
    pub point: Option<String>,
    /// Unmount and make the part a root of the vehicle. The position and attitude of a root are in vehicle coordinates
    /// (not written: the origin, no rotation). Also use it to move a part that is already a root
    #[arg(long, conflicts_with_all = ["on", "point"])]
    pub root: bool,
    /// Position (e.g. `"0 0 12 mm"`)
    #[arg(long, allow_hyphen_values = true)]
    pub position: Option<String>,
    /// Attitude quaternion (`"1 0 0 0"`, w x y z)
    #[arg(long, allow_hyphen_values = true)]
    pub orientation: Option<String>,
    /// Attitude as rotation angles about the X, Y and Z axes (e.g. `"0 90 0 deg"`; applied about the mount point's axes (the vehicle axes for a root) in the order X, Y, Z).
    /// Written to the file as a quaternion. Cannot be combined with `--orientation`
    #[arg(long, allow_hyphen_values = true, conflicts_with = "orientation")]
    pub rotation: Option<String>,
    /// Position in vehicle coordinates (e.g. `"30 0 -100 mm"`); the mount target and point stay as they are, and the result is written as a relative position and attitude (for a root, as its own position and attitude)
    #[arg(long, allow_hyphen_values = true, conflicts_with_all = ["on", "point", "root", "position", "orientation", "rotation"])]
    pub body_pos: Option<String>,
    /// Attitude in vehicle coordinates (part to vehicle, `"w x y z"`); use with `--body-pos`
    #[arg(long, allow_hyphen_values = true, requires = "body_pos")]
    pub body_rot: Option<String>,
    /// Write the pose to this configuration (`fune configuration add`) instead of the part's own `[pose]`. The mount stays as it is
    /// (`--on` and `--point` may be omitted, and must match the part's mount if given); the pose replaces the part's pose in that
    /// configuration as a whole, and `--body-pos` reads vehicle coordinates in that configuration. Undo with `fune configuration unset-pose`
    #[arg(long, conflicts_with = "root")]
    pub configuration: Option<String>,
    #[command(flatten)]
    pub edit: EditOpts,
}

// `fune mode view` の指定（`POST /system_modes/{system}` の依頼と同じ）。
#[derive(Debug, clap::Args)]
pub struct ModeViewOpts {
    /// Apply unsaved edits (a JSON file of Ops, or `-` for stdin) without writing, then compute
    #[arg(long)]
    pub ops: Option<std::path::PathBuf>,
    /// Analysis case ID (cases/<ID>.toml) for the analyses that `--analyzed` runs
    #[arg(long)]
    pub case: Option<String>,
    /// Also run the subsystem's analyses and add their per-mode view (power: budget destinations, verdict and charts)
    #[arg(long)]
    pub analyzed: bool,
    /// Return only the total row of the table footer (no available power or margin rows)
    #[arg(long)]
    pub totals_only: bool,
}

/// `fune mode …`
#[derive(Debug, Subcommand)]
pub enum ModeCmd {
    /// List modes
    Ls,
    /// Table of parts by mode
    Table,
    /// Per-mode values of one subsystem (power, cdh, thermal, aocs; sunlit and eclipse), as the GUI's mode view shows them
    ///
    /// Exit code 1 if an input is missing or a verdict is a violation.
    View {
        /// Subsystem ID (`fune mode view power`)
        system: String,
        #[command(flatten)]
        opts: ModeViewOpts,
    },
    /// Add a mode (`--from` copies the table of another mode)
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
    /// Remove a mode
    Rm {
        id: String,
        #[command(flatten)]
        edit: EditOpts,
    },
    /// Rename a mode
    Rename {
        id: String,
        to: String,
        #[command(flatten)]
        edit: EditOpts,
    },
    /// Change the display order of a mode
    Move {
        id: String,
        #[arg(long, group = "mode_position")]
        before: Option<String>,
        #[arg(long, group = "mode_position")]
        after: Option<String>,
        #[arg(long, group = "mode_position")]
        first: bool,
        #[arg(long, group = "mode_position")]
        last: bool,
        #[command(flatten)]
        edit: EditOpts,
    },
    /// Write the maximum duration of a mode (e.g. `fune mode duration NOMINAL --max-duration "90 min"`; omit the flag to remove it)
    Duration {
        mode: String,
        #[arg(long)]
        max_duration: Option<String>,
        #[command(flatten)]
        edit: EditOpts,
    },
    /// Write when a mode is used in the longest-duration evaluation (e.g. `fune mode timing MISSION --when sunlit`
    /// or `fune mode timing DOWNLINK --start eclipse_exit --offset "5 min"`; omit every flag to remove the timing)
    Timing {
        mode: String,
        /// Time window the mode may be used in (any / sunlit / eclipse)
        #[arg(long)]
        when: Option<String>,
        /// Fix the start at an eclipse event (eclipse_exit / eclipse_entry)
        #[arg(long)]
        start: Option<String>,
        /// Offset from the eclipse event (negative means before; needs --start)
        #[arg(long, allow_hyphen_values = true)]
        offset: Option<String>,
        #[command(flatten)]
        edit: EditOpts,
    },
    /// Set the state of a part in a mode (e.g. `fune mode set NOMINAL TRX rx --duty tx=0.1`)
    Set {
        mode: String,
        part: String,
        state: String,
        /// Time fractions of the other states (`state=fraction`, comma-separated)
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
    /// Remove a part's row from the mode table
    Unset {
        mode: String,
        part: String,
        #[arg(long, default_value_t = false, action = clap::ArgAction::Set, num_args = 0..=1, default_missing_value = "true")]
        eclipse: bool,
        #[command(flatten)]
        edit: EditOpts,
    },
}

/// Arguments of `fune net add|set`.
#[derive(Debug, Args)]
pub struct NetArgs {
    pub id: String,
    /// power / data / rf
    #[arg(long)]
    pub kind: String,
    #[arg(long)]
    pub label: Option<String>,
    /// Source port (`PCDU.OUT3`)
    #[arg(long)]
    pub source: Option<String>,
    /// Rail voltage (e.g. `"12 V"`)
    #[arg(long)]
    pub rail: Option<String>,
    /// Current limit (e.g. `"2 A"`)
    #[arg(long)]
    pub limit: Option<String>,
    #[arg(long)]
    pub protocol: Option<String>,
    /// Bit rate (e.g. `"1 Mbit/s"`)
    #[arg(long)]
    pub bitrate: Option<String>,
    /// Bus master (`OBC.CAN1`)
    #[arg(long)]
    pub master: Option<String>,
    #[command(flatten)]
    pub edit: EditOpts,
}

/// Arguments of `fune net set` (one per flag given).
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
    /// List nets
    Ls,
    /// Add a net
    Add(NetArgs),
    /// Write net fields (only the flags given; e.g. `fune net set P5V --rail "5.1 V"`)
    Set(NetSetArgs),
    /// Remove net fields (e.g. `fune net unset P5V limit`)
    Unset {
        id: String,
        /// label, source, rail, limit, protocol, bitrate, master
        field: String,
        #[command(flatten)]
        edit: EditOpts,
    },
    /// Replace the properties of a net entirely (flags not given are removed)
    Replace(NetArgs),
    /// Remove a net
    Rm {
        id: String,
        #[command(flatten)]
        edit: EditOpts,
    },
    /// Rename a net (the `net` of ports, the `nets` of harness segments and diagram nodes are rewritten too)
    Rename {
        id: String,
        to: String,
        #[command(flatten)]
        edit: EditOpts,
    },
    /// Connect a part's port to a net (e.g. `fune net connect RW_X.PWR P12V_AOCS`)
    Connect {
        port: String,
        net: String,
        #[command(flatten)]
        edit: EditOpts,
    },
    /// Disconnect a part's port from a net
    Disconnect {
        port: String,
        #[command(flatten)]
        edit: EditOpts,
    },
}

/// `fune sequence …`
#[derive(Debug, Subcommand)]
pub enum SequenceCmd {
    /// List the named mode sequences
    Ls,
    /// Write a named mode sequence in full (e.g. `fune sequence set imaging_day --segments "NOMINAL=76 min,MISSION=5 min" --anchor eclipse_exit`).
    /// Choose the one the analyses use with `fune setting set analysis.power.active_sequence <name>`
    Set {
        name: String,
        /// Segments as `mode=length`, separated by commas (repeated every cycle)
        #[arg(long)]
        segments: String,
        /// Start the sequence at an eclipse event (eclipse_exit / eclipse_entry); omit to start at the beginning of the analysis
        #[arg(long)]
        anchor: Option<String>,
        /// Offset from the eclipse event (negative means before; needs --anchor)
        #[arg(long, allow_hyphen_values = true)]
        offset: Option<String>,
        #[command(flatten)]
        edit: EditOpts,
    },
    /// Remove a named mode sequence
    Rm {
        name: String,
        #[command(flatten)]
        edit: EditOpts,
    },
}

/// `fune acknowledged …`
#[derive(Debug, Subcommand)]
pub enum AcknowledgedCmd {
    /// Acknowledge a geometry finding with a reason, so it no longer warns but stays listed
    /// (e.g. `fune acknowledged add interference SAP_PY SP_PY --reason "Hinge edge of the box model"`).
    /// For `cone_clearance`, give the cone carrier first and the part inside its cone second
    Add {
        /// Kind of finding (`interference` or `cone_clearance`)
        finding: String,
        /// First part (the two overlapping parts in either order; for a cone, the carrier)
        first: String,
        /// Second part (for a cone, the part inside the cone)
        second: String,
        /// Why the finding is acceptable
        #[arg(long)]
        reason: String,
        /// Only for this configuration (omit for every configuration and the base placement)
        #[arg(long)]
        configuration: Option<String>,
        #[command(flatten)]
        edit: EditOpts,
    },
    /// Withdraw an acknowledgement (the finding warns again)
    Rm {
        /// Kind of finding (`interference` or `cone_clearance`)
        finding: String,
        first: String,
        second: String,
        /// The configuration the acknowledgement was written for (omit if it applies to all)
        #[arg(long)]
        configuration: Option<String>,
        #[command(flatten)]
        edit: EditOpts,
    },
}

/// `fune setting …`
#[derive(Debug, Subcommand)]
pub enum SettingCmd {
    /// List settings
    Ls,
    /// Change a setting (e.g. `fune setting set analysis.power.dod_max 0.3`; a list setting such as `analysis.thermal.nodes.<NODE>`
    /// takes comma-separated words `"A, B"` or a TOML array of strings `'["A", "B"]'`)
    Set {
        key: String,
        #[arg(allow_hyphen_values = true)]
        value: String,
        #[command(flatten)]
        edit: EditOpts,
    },
    /// Remove a setting
    Unset {
        key: String,
        #[command(flatten)]
        edit: EditOpts,
    },
}

/// `fune view ...` (layout of the system diagram and the like; appearance only)
#[derive(Debug, Subcommand)]
pub enum ViewCmd {
    /// Set the route of a line by hand (e.g. `fune view edge-set electrical "P5V:PCDU.OUT2>CAM.PWR" --points "100,80 400,80 400,200"`)
    EdgeSet {
        view: String,
        /// Key that identifies the line (`net:start port>end port`; docs/model.md §10.4)
        key: String,
        /// Polyline points (`x,y` separated by spaces; adjacent points must be horizontal or vertical)
        #[arg(long, allow_hyphen_values = true)]
        points: String,
        /// Side the start port leaves from (left / right / top / bottom)
        #[arg(long)]
        source_side: Option<fune_service::dto::ViewPortSide>,
        /// Side the end port leaves from
        #[arg(long)]
        target_side: Option<fune_service::dto::ViewPortSide>,
        /// Position of the start port (distance in px from the left or top corner of that side; if omitted, set by the port order)
        #[arg(long)]
        source_offset: Option<i32>,
        /// Position of the end port
        #[arg(long)]
        target_offset: Option<i32>,
        #[command(flatten)]
        edit: EditOpts,
    },
    /// Reset the route of a line to automatic layout
    EdgeReset {
        view: String,
        key: String,
        #[command(flatten)]
        edit: EditOpts,
    },
    /// Remove all hand-set positions and line routes of a diagram and return to automatic layout
    Reset {
        view: String,
        #[command(flatten)]
        edit: EditOpts,
    },
    /// List node positions of a diagram
    Ls { view: String },
    /// Set the position (and size) of a node (e.g. `fune view set electrical RW_X --pos 420,120`)
    Set {
        view: String,
        id: String,
        /// Position `X,Y` (integers)
        #[arg(long, allow_hyphen_values = true)]
        pos: String,
        /// Size `W,H` (integers)
        #[arg(long)]
        size: Option<String>,
        #[command(flatten)]
        edit: EditOpts,
    },
    /// Remove a node from the diagram
    Rm {
        view: String,
        id: String,
        #[command(flatten)]
        edit: EditOpts,
    },
}
