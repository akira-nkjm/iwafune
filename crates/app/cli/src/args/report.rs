//! 設計書の表と図の書き出しのサブコマンド（`fune report …`）。

use clap::{Args, Subcommand};
use std::path::PathBuf;

/// `fune report …`
#[derive(Debug, Subcommand)]
pub enum ReportCmd {
    /// Write the report tables and figures (data.json, SVG figures, and a Typst module) for your own Typst document
    ///
    /// Exports six items from the design data and the analyses: the mass table, the power-by-mode table, the link budget
    /// tables (inputs and results per link), the thermal result table, the temperature history figure and the generation
    /// and battery history figure. Nothing in the design changes. The files are written to --out; include them from your
    /// own .typ with `#import "iwafune.typ": *` (see example.typ).
    Export(ReportExportArgs),
}

/// `fune report export`
#[derive(Debug, Args)]
pub struct ReportExportArgs {
    /// Output directory (default: exports/report under the project root)
    #[arg(long, value_name = "DIR")]
    pub out: Option<PathBuf>,
    /// Analysis case to export the case-dependent items for (cases/<ID>.toml; repeat for several).
    /// Without it, the analyses' default case is used once
    #[arg(long = "case", value_name = "ID")]
    pub cases: Vec<String>,
    /// Item to export (mass, mode-power, link-budget, thermal-result, thermal-history, power-history; repeat for several).
    /// Without it, all six
    #[arg(long = "item", value_name = "NAME")]
    pub items: Vec<String>,
    /// Language of headings, captions and notes
    #[arg(long, value_name = "LANG", default_value = "en", value_parser = ["ja", "en"])]
    pub lang: String,
}
