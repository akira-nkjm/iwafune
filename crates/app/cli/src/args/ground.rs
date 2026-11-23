//! 地上局のサブコマンド（`fune ground …`）。旗は `fune_ops::to_cli` が書き出す地上局のコマンドと 1 対 1。

use super::EditOpts;
use clap::{Args, Subcommand};

/// `fune ground ...` (ground stations, conditions/ground/; to change a value use `fune set conditions.ground.<ID>.<field> …`)
#[derive(Debug, Subcommand)]
pub enum GroundCmd {
    /// Add a ground station (a band's performance is written with `fune set conditions.ground.<ID>.link.<band>.<field> …`)
    Add(GroundAddArgs),
    /// Remove a ground station (refused while `analysis.comm.stations` lists it)
    Rm {
        id: String,
        #[command(flatten)]
        edit: EditOpts,
    },
    /// Rename a ground station (`analysis.comm.stations` is rewritten too)
    Rename {
        id: String,
        to: String,
        #[command(flatten)]
        edit: EditOpts,
    },
}

/// `fune ground add`
#[derive(Debug, Args)]
pub struct GroundAddArgs {
    pub id: String,
    /// Display name
    #[arg(long)]
    pub label: Option<String>,
    /// Latitude (e.g. `"35.7126 deg"`)
    #[arg(long, allow_hyphen_values = true)]
    pub latitude: Option<String>,
    /// Longitude (e.g. `"139.762 deg"`)
    #[arg(long, allow_hyphen_values = true)]
    pub longitude: Option<String>,
    /// Altitude above the reference surface (e.g. `"40 m"`)
    #[arg(long, allow_hyphen_values = true)]
    pub altitude: Option<String>,
    /// Lowest elevation at which the station operates (e.g. `"10 deg"`)
    #[arg(long, allow_hyphen_values = true)]
    pub min_elevation: Option<String>,
    #[command(flatten)]
    pub edit: EditOpts,
}
