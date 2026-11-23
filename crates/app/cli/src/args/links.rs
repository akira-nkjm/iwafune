//! 通信の回線とプロジェクトのサブコマンド（`fune link …`・`fune project …`）。旗は `fune_ops::to_cli` が書き出すコマンドと 1 対 1。

use super::EditOpts;
use clap::{Args, Subcommand};

/// `fune link ...` (communication links, `[link.<name>]` in analysis/comm.toml; to change a value use `fune set analysis.comm.link.<name>.<field> …`)
#[derive(Debug, Subcommand)]
pub enum LinkCmd {
    /// Add a link with the fields that make it load (direction, part, band)
    Add(LinkAddArgs),
    /// Remove a link (refused while a requirement's `verify.check` refers to it; `analysis.comm.data_downlink` is cleared if it names the link)
    Rm {
        id: String,
        #[command(flatten)]
        edit: EditOpts,
    },
    /// Rename a link (`analysis.comm.data_downlink` and requirement checks such as `margin:<link>@<station>` are rewritten too)
    Rename {
        id: String,
        to: String,
        #[command(flatten)]
        edit: EditOpts,
    },
}

/// `fune link add`
#[derive(Debug, Args)]
pub struct LinkAddArgs {
    pub id: String,
    /// down / up
    #[arg(long)]
    pub direction: String,
    /// Transceiver part on the spacecraft
    #[arg(long)]
    pub part: String,
    /// Band of the ground station (`[link.<band>]` in conditions/ground/<ID>.toml)
    #[arg(long)]
    pub band: String,
    #[command(flatten)]
    pub edit: EditOpts,
}

/// `fune project ...` (the `[project]` table of fune.toml)
#[derive(Debug, Subcommand)]
pub enum ProjectCmd {
    /// Write a field: `name` (the project name) or `label` (the display name)
    Set {
        /// name / label
        field: String,
        value: String,
        #[command(flatten)]
        edit: EditOpts,
    },
    /// Remove a field (only `label` can be removed)
    Unset {
        /// label
        field: String,
        #[command(flatten)]
        edit: EditOpts,
    },
}
