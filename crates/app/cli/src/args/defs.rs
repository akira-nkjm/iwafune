//! 属性定義・種別定義のサブコマンド（`fune attr …`・`fune kind …`）。
//! 旗は `fune_ops::to_cli` が書き出す定義のコマンドとフィールドが 1 対 1。

use super::EditOpts;
use clap::{Args, Subcommand};

/// `fune attr ...` (an argument struct, so the size difference between variants does not matter)
#[derive(Debug, Subcommand)]
#[allow(clippy::large_enum_variant)]
pub enum AttrCmd {
    /// List attribute definitions
    Ls,
    /// Add an attribute definition (checks the dimension and allowed units)
    Add(AttrAddArgs),
    /// Replace an attribute definition entirely (same flags as `add`; project-specific definitions only)
    Replace(AttrAddArgs),
    /// Rename a project-defined attribute (kinds' required/optional lists and the values of parts and catalog items are rewritten too)
    Rename {
        id: String,
        to: String,
        #[command(flatten)]
        edit: EditOpts,
    },
    /// Remove an attribute definition (refused if a kind lists it or a part or catalog item has a value for it)
    Rm {
        id: String,
        #[command(flatten)]
        edit: EditOpts,
    },
}

/// `fune attr add|replace`
#[derive(Debug, Args)]
pub struct AttrAddArgs {
    pub id: String,
    #[arg(long)]
    pub label: Option<String>,
    /// English display name
    #[arg(long)]
    pub label_en: Option<String>,
    #[arg(long)]
    pub description: Option<String>,
    /// Value type (scalar, vec3, quat, tensor3, enum, bool, text, range, table; default scalar)
    #[arg(long = "type", default_value = "scalar")]
    pub value_type: String,
    /// Whether the value is held per state
    #[arg(long)]
    pub by_state: bool,
    #[arg(long)]
    pub dimension: Option<String>,
    /// Allowed units (comma-separated; e.g. `"A*m^2"`)
    #[arg(long)]
    pub units: Option<String>,
    #[arg(long)]
    pub display_unit: Option<String>,
    /// Lower bound of the range (SI number; read by the service)
    #[arg(long, allow_negative_numbers = true)]
    pub min: Option<String>,
    /// Upper bound of the range (SI number)
    #[arg(long, allow_negative_numbers = true)]
    pub max: Option<String>,
    /// Default value (SI number)
    #[arg(long, allow_negative_numbers = true)]
    pub default: Option<String>,
    /// Enum vocabulary (comma-separated)
    #[arg(long)]
    pub symbols: Option<String>,
    /// Which budget it belongs to (mass / power / data)
    #[arg(long)]
    pub budget: Option<String>,
    /// Direction vector (should be a unit vector)
    #[arg(long)]
    pub unit_vector: bool,
    /// For the half angle of a field of view: the direction attribute it is around (for example boresight)
    #[arg(long)]
    pub fov_axis: Option<String>,
    /// Subsystem it is listed under on screen (basic, mass, aocs, cdh, comm, payload, power, propulsion, structure, thermal)
    #[arg(long)]
    pub section: Option<String>,
    #[command(flatten)]
    pub edit: EditOpts,
}

/// `fune kind ...` (an argument struct, so the size difference between variants does not matter)
#[derive(Debug, Subcommand)]
#[allow(clippy::large_enum_variant)]
pub enum KindCmd {
    /// List kind definitions
    Ls,
    /// Add a kind definition
    Add(KindAddArgs),
    /// Replace a kind definition entirely (same flags as `add`; project-specific definitions only)
    Replace(KindAddArgs),
    /// Rename a project-defined kind (the kind of parts and catalog items and the `extends` of other kinds are rewritten too)
    Rename {
        id: String,
        to: String,
        #[command(flatten)]
        edit: EditOpts,
    },
    /// Remove a kind definition (refused if a part or catalog item uses it or another kind inherits it)
    Rm {
        id: String,
        #[command(flatten)]
        edit: EditOpts,
    },
}

/// `fune kind add|replace`
#[derive(Debug, Args)]
pub struct KindAddArgs {
    pub id: String,
    #[arg(long)]
    pub label: Option<String>,
    /// English display name
    #[arg(long)]
    pub label_en: Option<String>,
    #[arg(long)]
    pub description: Option<String>,
    #[arg(long)]
    pub extends: Option<String>,
    #[arg(long)]
    pub role: Option<String>,
    /// Equipment states (comma-separated; e.g. `"off,on"`)
    #[arg(long)]
    pub states: Option<String>,
    /// Required attributes (comma-separated)
    #[arg(long)]
    pub required: Option<String>,
    /// Optional attributes (comma-separated)
    #[arg(long)]
    pub optional: Option<String>,
    /// Port template `name=power|data|rf` (repeatable)
    #[arg(long = "port", value_name = "NAME=KIND")]
    pub ports: Vec<String>,
    #[command(flatten)]
    pub edit: EditOpts,
}
