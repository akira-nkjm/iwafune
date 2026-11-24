//! 形態のサブコマンド（`fune configuration …`）。綴りは `fune_ops::to_cli` と同じにする。`set` は無ければ作り、あれば見出しの欄を
//! 置き換える（部品ごとの姿勢 `[pose.<部品>]` は今のまま残す）。形態の中への配置は `fune part place --configuration`。

use super::EditOpts;
use clap::{Args, Subcommand};

/// `fune configuration …`
#[derive(Debug, Subcommand)]
pub enum ConfigurationCmd {
    /// List configurations and the parts whose pose each one overrides
    Ls,
    /// Show a configuration (use `fune part ls --configuration <ID>` for the resulting poses)
    Show { id: String },
    /// Add a configuration (refused if the ID exists). Place parts in it with `fune part place <PART> --configuration <ID> …`
    Add(ConfigurationArgs),
    /// Write a configuration (create it, or replace its label and description if it exists: one not given is removed and listed in the output;
    /// the part poses are kept)
    Set(ConfigurationArgs),
    /// Remove a configuration (its part poses go with it)
    Rm {
        id: String,
        #[command(flatten)]
        edit: EditOpts,
    },
    /// Rename a configuration
    Rename {
        id: String,
        to: String,
        #[command(flatten)]
        edit: EditOpts,
    },
    /// Remove the pose override of a part in a configuration (the part's own pose applies again)
    UnsetPose {
        configuration: String,
        part: String,
        #[command(flatten)]
        edit: EditOpts,
    },
}

/// `fune configuration add|set` (the flags map one to one to `fune_service::dto::ConfigurationText`)
#[derive(Debug, Args)]
pub struct ConfigurationArgs {
    pub id: String,
    /// Display name
    #[arg(long)]
    pub label: Option<String>,
    /// Description (free text)
    #[arg(long)]
    pub description: Option<String>,
    #[command(flatten)]
    pub edit: EditOpts,
}
