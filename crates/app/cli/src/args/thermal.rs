//! `fune thermal …` の引数（熱の結合・表面の表。編集は `fune set|unset analysis.thermal.…`）。

use clap::Subcommand;
use std::path::PathBuf;

/// `fune thermal …`
#[derive(Debug, Subcommand)]
pub enum ThermalCmd {
    /// Couplings between nodes (value in use and source), outer-surface coatings, coatings and node temperatures, as the GUI's
    /// "Couplings and surfaces" tab shows them. Edit with `fune set|unset analysis.thermal.conductive.<A>.<B>` (and `radiative`, `coating`, `surface`)
    Tables {
        /// Apply unsaved edits (a JSON file of Ops, or `-` for stdin) without writing, then compute
        #[arg(long)]
        ops: Option<PathBuf>,
        /// Analysis case ID (cases/<ID>.toml)
        #[arg(long)]
        case: Option<String>,
    },
}
