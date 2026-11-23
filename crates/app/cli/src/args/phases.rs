//! フェーズのサブコマンド（`fune phase …`）。綴りは `fune_ops::to_cli` と同じにする。`set` は無ければ作り、あれば見出しの欄を置き換える
//! （軌道 `[orbit]` と姿勢則 `[law.<モード>]` は今のまま残し、`set-field`・`unset-field` で書く）。

use super::EditOpts;
use clap::{Args, Subcommand};

/// `fune phase …`
#[derive(Debug, Subcommand)]
pub enum PhaseCmd {
    /// List phases (in order)
    Ls,
    /// Show a phase with its orbit and attitude law overrides
    Show { id: String },
    /// Add a phase (refused if the ID exists)
    Add(PhaseArgs),
    /// Write a phase (create it, or replace its fields if it exists; the orbit and attitude law overrides are kept)
    Set(PhaseArgs),
    /// Remove a phase (refused while an analysis case refers to it)
    Rm {
        id: String,
        #[command(flatten)]
        edit: EditOpts,
    },
    /// Rename a phase (analysis cases that refer to it follow)
    Rename {
        id: String,
        to: String,
        #[command(flatten)]
        edit: EditOpts,
    },
    /// Write one value of the orbit override (`orbit.<key>`) or of the attitude law override (`law.<mode>.<key>`)
    SetField {
        phase: String,
        /// `orbit.altitude`, `orbit.inclination`, `law.NOMINAL.law`, `law.NOMINAL.axis`, …
        path: String,
        #[arg(allow_hyphen_values = true)]
        value: String,
        #[command(flatten)]
        edit: EditOpts,
    },
    /// Remove one value of the orbit or attitude law override
    UnsetField {
        phase: String,
        path: String,
        #[command(flatten)]
        edit: EditOpts,
    },
}

/// `fune phase add|set` (the flags map one to one to `fune_service::dto::PhaseText`)
#[derive(Debug, Args)]
pub struct PhaseArgs {
    pub id: String,
    /// Display name
    #[arg(long)]
    pub label: Option<String>,
    /// Position in the list (an integer; phases are listed by order, then ID)
    #[arg(long, allow_hyphen_values = true)]
    pub order: Option<String>,
    /// Condition that starts the phase (free text)
    #[arg(long)]
    pub start: Option<String>,
    /// Condition that ends the phase (free text)
    #[arg(long)]
    pub end: Option<String>,
    /// Duration (e.g. `"30 d"`)
    #[arg(long)]
    pub duration: Option<String>,
    /// Modes used in the phase (comma-separated)
    #[arg(long)]
    pub modes: Option<String>,
    #[command(flatten)]
    pub edit: EditOpts,
}
