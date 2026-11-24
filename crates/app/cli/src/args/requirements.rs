//! 要求のサブコマンド（`fune requirement …`）。綴りは `fune_ops::to_cli` と同じにする。`set` は無ければ作り、あれば丸ごと置き換える。

use super::EditOpts;
use clap::{Args, Subcommand};

/// `fune requirement …`
#[derive(Debug, Subcommand)]
pub enum RequirementCmd {
    /// List requirements
    Ls,
    /// Show the requirement compliance table: for each requirement, the verdict of the analysis
    /// check or budget that verifies it, over the cases it must hold for (exit code 1 on a violation)
    Compliance {
        /// Also exit with code 1 when a requirement could not be verified
        #[arg(long)]
        deny_unverified: bool,
    },
    /// Add a requirement (refused if the ID exists)
    Add(RequirementArgs),
    /// Write a requirement (create it, or replace it entirely if it exists: fields not given are removed and listed in the output;
    /// there is no one-field edit for requirements yet, so give every field you want to keep)
    Set(RequirementArgs),
    /// Remove a requirement
    Rm {
        id: String,
        #[command(flatten)]
        edit: EditOpts,
    },
}

/// `fune requirement add|set` (the flags map one to one to `fune_service::dto::RequirementText`)
#[derive(Debug, Args)]
pub struct RequirementArgs {
    pub id: String,
    /// The requirement statement ("The satellite shall …")
    #[arg(long)]
    pub statement: String,
    /// English statement (shown by the GUI in English)
    #[arg(long)]
    pub statement_en: Option<String>,
    /// Short name
    #[arg(long)]
    pub label: Option<String>,
    /// Dimension of the constrained quantity (e.g. `Mass`, `Power`, `Temperature`)
    #[arg(long)]
    pub quantity: String,
    /// Source ID (sources/)
    #[arg(long)]
    pub source: Option<String>,
    /// Direction of the constraint: at_most / at_least / equal / range
    #[arg(long)]
    pub kind: String,
    /// Bound (e.g. `"12 kg"`; at_most, at_least, equal)
    #[arg(long, allow_hyphen_values = true)]
    pub value: Option<String>,
    /// Tolerance around the value (equal only)
    #[arg(long)]
    pub tolerance: Option<String>,
    /// Lower bound (range only)
    #[arg(long, allow_hyphen_values = true)]
    pub min: Option<String>,
    /// Upper bound (range only)
    #[arg(long, allow_hyphen_values = true)]
    pub max: Option<String>,
    /// Must hold in every analysis case (true / false)
    #[arg(long)]
    pub all_cases: Option<String>,
    /// Analysis cases it must hold in (comma-separated; not with --all-cases)
    #[arg(long)]
    pub cases: Option<String>,
    /// Analysis that verifies it (e.g. `power.energy_balance`)
    #[arg(long)]
    pub analysis: Option<String>,
    /// Key of the analysis check that verifies it (e.g. `dod`; the worst check with this key counts)
    #[arg(long)]
    pub check: Option<String>,
    /// Budget that verifies it instead of an analysis (mass / power / data)
    #[arg(long)]
    pub budget: Option<String>,
    /// Analysis input this requirement gives instead of a check (`mission_duration`: its at_least value is the mission duration)
    #[arg(long)]
    pub input: Option<String>,
    #[command(flatten)]
    pub edit: EditOpts,
}
