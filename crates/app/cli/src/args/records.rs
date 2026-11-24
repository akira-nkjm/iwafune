//! 解析ケース・出典・ハーネス・マヌーバのサブコマンド（`fune case …`・`fune source …`・`fune harness …`・`fune maneuver …`）。
//! 綴りは `fune_ops::to_cli` と同じにする。`set` は無ければ作り、あれば丸ごと置き換える。

use super::EditOpts;
use clap::{Args, Subcommand};

/// `fune case …`
#[derive(Debug, Subcommand)]
pub enum CaseCmd {
    /// List analysis cases
    Ls,
    /// Write an analysis case (creates it, or replaces the whole record: fields not given are removed and listed in the output;
    /// to change one field and keep the rest use `fune set cases.<ID>.<field> <value>`, e.g. `fune set cases.hot.beta "60 deg"`)
    Set(CaseArgs),
    /// Create analysis cases from the representative days of a seasonal analysis
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
    /// Create an analysis case at the minimum or maximum Sun distance of an orbit about the Sun
    FromSunDistance {
        id: String,
        /// min / max (perihelion / aphelion within the mission duration)
        #[arg(long, value_parser = ["min", "max"])]
        at: String,
        #[arg(long)]
        mode: Option<String>,
        #[command(flatten)]
        edit: EditOpts,
    },
    /// Create (or update) the analysis cases of the worst days of an orbit: the farthest and nearest Sun distance and the farthest Earth distance, each as a case with an epoch
    FromGeometry {
        /// Phase whose orbit is read (required when the design has phases; also set as the cases' phase)
        #[arg(long)]
        phase: Option<String>,
        /// Prefix of the case IDs (`<prefix>_sun_far`, `<prefix>_sun_near`, `<prefix>_earth_far`; default: the phase ID in lower case, or none)
        #[arg(long)]
        prefix: Option<String>,
        /// Mode of the cases (an existing case keeps its mode when omitted)
        #[arg(long)]
        mode: Option<String>,
        #[command(flatten)]
        edit: EditOpts,
    },
    /// Set the phase of an analysis case (its orbit and attitude law overrides then apply; same as `fune set cases.<ID>.phase <PHASE>`)
    SetPhase {
        id: String,
        phase: String,
        #[command(flatten)]
        edit: EditOpts,
    },
    /// Clear the phase of an analysis case
    UnsetPhase {
        id: String,
        #[command(flatten)]
        edit: EditOpts,
    },
    /// Set the time an analysis case starts at on its orbit (ISO 8601; the other fields stay as they are; same as `fune set cases.<ID>.epoch <TIME>`)
    SetEpoch {
        id: String,
        /// e.g. `"2031-05-02T00:00:00Z"` (within the span for a one-off orbit)
        epoch: String,
        #[command(flatten)]
        edit: EditOpts,
    },
    /// Clear the time of an analysis case (the analyses start at the orbit's own epoch)
    UnsetEpoch {
        id: String,
        #[command(flatten)]
        edit: EditOpts,
    },
    /// Remove an analysis case
    Rm {
        id: String,
        #[command(flatten)]
        edit: EditOpts,
    },
    /// Rename an analysis case (`analysis.common.default_case` and the `quantify.cases` of requirements are rewritten too)
    Rename {
        id: String,
        to: String,
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
    /// Mode to use
    #[arg(long)]
    pub mode: Option<String>,
    /// Phase (phases/); its orbit and attitude law overrides apply to the case
    #[arg(long)]
    pub phase: Option<String>,
    /// Which side of the uncertainty to read (nominal / min / max)
    #[arg(long)]
    pub side: Option<String>,
    /// Thermal environment (`[case.<name>]` in `analysis.thermal`)
    #[arg(long)]
    pub thermal_case: Option<String>,
    /// Beta angle (e.g. `"60 deg"`)
    #[arg(long, allow_hyphen_values = true)]
    pub beta: Option<String>,
    /// Number of orbits for the time series
    #[arg(long)]
    pub orbits: Option<u32>,
    /// Distance from the Sun (e.g. `"0.7 AU"`; orbits about the Sun only: analyze a circular orbit at this distance)
    #[arg(long)]
    pub sun_distance: Option<String>,
    /// Time the analyses start at on the case's orbit (ISO 8601, e.g. `"2031-05-02T00:00:00Z"`; within the span for a one-off orbit)
    #[arg(long)]
    pub epoch: Option<String>,
    #[command(flatten)]
    pub edit: EditOpts,
}

/// `fune source …`
#[derive(Debug, Subcommand)]
pub enum SourceCmd {
    /// List sources
    Ls,
    /// Write a source (creates it, or replaces the whole record: fields not given are removed and listed in the output;
    /// there is no one-field edit for sources yet, so give every field you want to keep)
    Set(Box<SourceArgs>),
    /// Remove a source (refused if a value refers to it)
    Rm {
        id: String,
        #[command(flatten)]
        edit: EditOpts,
    },
    /// Rename a source (the sources of values, catalog items and requirements are rewritten too)
    Rename {
        id: String,
        to: String,
        #[command(flatten)]
        edit: EditOpts,
    },
    /// Move a source to another directory of `sources/` (the ID and references do not change)
    Mv {
        id: String,
        /// Directory relative to `sources/` (e.g. `datasheets` or `a/b`); `.` is `sources/` itself
        #[arg(value_name = "DIR")]
        target: String,
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
    /// Reference (URL, path in the repository, or document number)
    #[arg(long = "ref")]
    pub reference: Option<String>,
    /// Document revision (e.g. `"Rev. C"`)
    #[arg(long)]
    pub revision: Option<String>,
    /// Date the document was retrieved (`2026-10-15`)
    #[arg(long)]
    pub retrieved: Option<String>,
    /// SHA-256 of the retrieved file (64 hex digits)
    #[arg(long)]
    pub sha256: Option<String>,
    /// Distribution class (public / on_request / restricted)
    #[arg(long)]
    pub distribution: Option<String>,
    /// Date (when the test, analysis or judgement was made, `2026-10-15`)
    #[arg(long)]
    pub date: Option<String>,
    #[arg(long)]
    pub by: Option<String>,
    #[arg(long)]
    pub note: Option<String>,
    /// Directory to put a new source in, relative to `sources/` (e.g. `datasheets` or `a/b`; default: `sources/` itself).
    /// An existing source stays where it is (a different directory is refused; use `fune source mv`).
    /// Not `--dir`: that is the global project location (`-C`)
    #[arg(long)]
    pub subdir: Option<String>,
    #[command(flatten)]
    pub edit: EditOpts,
}

/// `fune harness …`
#[derive(Debug, Subcommand)]
pub enum HarnessCmd {
    /// List harness segments
    Ls,
    /// Write a harness segment (creates it, or replaces the whole record: fields not given are removed and listed in the output;
    /// there is no one-field edit for harness segments yet, so give every field you want to keep)
    Set(HarnessArgs),
    /// Remove a harness segment
    Rm {
        id: String,
        #[command(flatten)]
        edit: EditOpts,
    },
    /// Rename a harness segment (nothing else refers to a segment, so only the file moves)
    Rename {
        id: String,
        to: String,
        #[command(flatten)]
        edit: EditOpts,
    },
}

/// `fune harness set`
#[derive(Debug, Args)]
pub struct HarnessArgs {
    pub id: String,
    /// Port at one end (`PCDU.OUT3`)
    #[arg(long)]
    pub from: String,
    /// Port at the other end (`RW_X.PWR`)
    #[arg(long)]
    pub to: String,
    /// Nets carried (comma-separated)
    #[arg(long)]
    pub nets: Option<String>,
    /// Catalog item for the wire
    #[arg(long)]
    pub wire: Option<String>,
    #[arg(long)]
    pub cores: Option<u32>,
    /// Length (e.g. `"0.35 m"`)
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
    /// List maneuvers (in execution order)
    Ls,
    /// Add a maneuver (refused if the ID exists)
    Add(ManeuverArgs),
    /// Write a maneuver (create it, or replace it entirely if it exists: fields not given are removed and listed in the output;
    /// there is no one-field edit for maneuvers yet, so give every field you want to keep)
    Set(ManeuverArgs),
    /// Remove a maneuver (later maneuvers are re-linked to the one before it)
    Rm {
        id: String,
        #[command(flatten)]
        edit: EditOpts,
    },
    /// Rename a maneuver (the `after` of other maneuvers is rewritten too)
    Rename {
        id: String,
        to: String,
        #[command(flatten)]
        edit: EditOpts,
    },
    /// Set the departure and arrival dates of a transfer between bodies to the minimum of the date search (`propulsion.launch_window`)
    SetWindow {
        id: String,
        /// dv (minimum maneuver delta-V) / c3 (minimum departure C3)
        #[arg(long, value_parser = ["dv", "c3"])]
        min: String,
        #[command(flatten)]
        edit: EditOpts,
    },
}

/// `fune maneuver add|set` (the flags map one to one to `fune_service::dto::ManeuverText`)
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
    /// Thruster part
    #[arg(long)]
    pub thruster: Option<String>,
    /// Previous maneuver
    #[arg(long)]
    pub after: Option<String>,
    /// Intermediate apoapsis altitude of a bi-elliptic transfer (e.g. `"20000 km"`)
    #[arg(long)]
    pub via_altitude: Option<String>,
    /// Written delta-V of an allowance (e.g. `"15 m/s"`: trajectory correction, orbit maintenance, margin). The orbit does not change; not combined with the target orbit flags, --via-altitude or dates
    #[arg(long)]
    pub delta_v: Option<String>,
    /// Central body of the target (e.g. `moon`, `mars`; makes it a transfer between bodies, with altitudes measured from that body)
    #[arg(long)]
    pub body: Option<String>,
    /// Altitude of the target circular orbit (e.g. `"600 km"`)
    #[arg(long)]
    pub altitude: Option<String>,
    /// Target periapsis altitude
    #[arg(long, allow_hyphen_values = true)]
    pub perigee_altitude: Option<String>,
    /// Target apoapsis altitude
    #[arg(long)]
    pub apogee_altitude: Option<String>,
    /// Target inclination (e.g. `"97.8 deg"`)
    #[arg(long)]
    pub inclination: Option<String>,
    /// Burn only in sunlight (true / false; not used by the coarse analysis)
    #[arg(long)]
    pub sunlit_only: Option<String>,
    /// Mode in which to burn (not used by the coarse analysis)
    #[arg(long)]
    pub mode: Option<String>,
    /// Duty (fraction of the allowed burn time spent burning; propagated only)
    #[arg(long)]
    pub duty: Option<String>,
    /// Thrust direction (tangential / inertial; propagated only)
    #[arg(long)]
    pub steering: Option<String>,
    /// Departure date of a dated transfer between bodies (ISO 8601, UTC, e.g. `2028-12-10T00:00:00Z`; needs --body and --arrive; omit both to use the Hohmann-based estimate)
    #[arg(long)]
    pub depart: Option<String>,
    /// Arrival date of a dated transfer between bodies (after --depart)
    #[arg(long)]
    pub arrive: Option<String>,
    /// Who performs the departure burn of a dated transfer: `spacecraft` (default) or `launcher` (the launcher injects onto the departure asymptote, so the spacecraft does not count that burn; needs --depart and --arrive)
    #[arg(long)]
    pub departure_by: Option<String>,
    #[command(flatten)]
    pub edit: EditOpts,
}

/// `fune part point` (mounting point)
#[derive(Debug, Args)]
pub struct PointArgs {
    pub part: String,
    pub point: String,
    /// Position (e.g. `"30 0 5 mm"`)
    #[arg(long, allow_hyphen_values = true)]
    pub position: Option<String>,
    /// Attitude quaternion (`"1 0 0 0"`, w x y z)
    #[arg(long, allow_hyphen_values = true)]
    pub orientation: Option<String>,
    /// Remove the mount point
    #[arg(long, conflicts_with_all = ["position", "orientation"])]
    pub rm: bool,
    #[command(flatten)]
    pub edit: EditOpts,
}

/// `fune catalog ...` (catalog items; only this project's own catalog items can be edited; shared catalogs are fetched and updated)
#[derive(Debug, Subcommand)]
pub enum CatalogCmd {
    /// List catalog items
    Ls,
    /// Show one catalog item
    Show { id: String },
    /// Fetch shared catalogs (`[[catalog]] git = …`) into `.fune/` and record the commit in catalog.lock
    Fetch {
        /// Do not rewrite catalog.lock (fails if the lock is missing or differs from fune.toml; for CI)
        #[arg(long)]
        locked: bool,
    },
    /// Move the shared catalog lock to the current commit of `rev` and show the items and values that changed
    Update {
        /// Name of the shared catalog (`name` in `[[catalog]]` of fune.toml)
        name: String,
    },
    /// Add a catalog item (`--from-part` copies the values of a part)
    Add {
        /// ID with its origin (e.g. `local/rw_30mNms`)
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
        /// Directory to put the item in, relative to the catalog directory (e.g. `vendor_a` or `a/b`; default: the catalog root).
        /// Not `--dir`: that is the global project location (`-C`)
        #[arg(long)]
        subdir: Option<String>,
        #[command(flatten)]
        edit: EditOpts,
    },
    /// Write an attribute value or a field (e.g. `fune catalog set local/rw_30mNms mass "0.9 kg"`)
    Set {
        id: String,
        /// Attribute (`mass`, `power.on`) or field (`label`, `manufacturer` …)
        path: String,
        #[arg(allow_hyphen_values = true)]
        value: String,
        #[arg(long)]
        source: Option<String>,
        /// Location in the document (e.g. `"p.12 table 3"`)
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
    /// Remove an attribute value or a field
    Unset {
        id: String,
        path: String,
        #[command(flatten)]
        edit: EditOpts,
    },
    /// Remove a catalog item (refused if referenced)
    Rm {
        id: String,
        #[command(flatten)]
        edit: EditOpts,
    },
    /// Rename a catalog item (also rewrites `catalog` and `candidates` of parts)
    Rename {
        from: String,
        to: String,
        #[command(flatten)]
        edit: EditOpts,
    },
    /// Move a catalog item to another directory of the catalog (the ID and references do not change)
    Mv {
        id: String,
        /// Directory relative to the catalog directory (e.g. `vendor_a` or `a/b`); `.` is the catalog root
        #[arg(value_name = "DIR")]
        target: String,
        #[command(flatten)]
        edit: EditOpts,
    },
    /// Catalog origins (`[[catalog]]` in fune.toml): add a shared catalog (`git = …`) or one of this project (`path = …`), or remove one
    #[command(subcommand)]
    Source(CatalogSourceCmd),
}

/// `fune catalog source ...` (the shared catalogs of fune.toml: `[[catalog]] name = ... git = ... rev = ...`)
#[derive(Debug, Subcommand)]
pub enum CatalogSourceCmd {
    /// Add a shared catalog to fune.toml (does not fetch; run `fune catalog fetch` next)
    Add {
        /// Name of the catalog (items appear as `<name>/<ID>`)
        name: String,
        /// Git URL to fetch from (https, ssh, file://)
        #[arg(long)]
        git: String,
        /// Branch, tag or commit (default branch if omitted)
        #[arg(long)]
        rev: Option<String>,
        #[command(flatten)]
        edit: EditOpts,
    },
    /// Add a catalog of this project to fune.toml (`[[catalog]] path = …`; the directory is created when the first item is added)
    AddLocal {
        /// Name of the catalog (items appear as `<name>/<ID>`)
        name: String,
        /// Directory inside the project (e.g. `vendor_parts` or `a/b`; not one the project layout uses, such as `parts`)
        #[arg(long)]
        path: String,
        #[command(flatten)]
        edit: EditOpts,
    },
    /// Remove a catalog from fune.toml (refused if a part or harness uses its items, or, for a catalog of this project, while it holds items; the catalog.lock entry of a shared catalog is dropped on the next fetch)
    Rm {
        name: String,
        #[command(flatten)]
        edit: EditOpts,
    },
}
