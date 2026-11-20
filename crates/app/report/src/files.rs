//! 書き出すファイルの一覧。ディスクには書かない（CLI が書き、HTTP は本文を返す）。順序と本文は入力だけで決まる。

use crate::ReportError;
use crate::model::ReportData;
use crate::typst::{self, MODULE_FILE};
use serde::{Deserialize, Serialize};

/// `data.json` のファイル名。
pub const DATA_FILE: &str = "data.json";
/// 例の文書のファイル名。
pub const EXAMPLE_FILE: &str = "example.typ";
/// 図を置くディレクトリ。
pub const FIGURE_DIR: &str = "figs";

/// 書き出すファイル 1 つ（出力先からの相対パスと本文）。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct ReportFile {
    pub path: String,
    pub content: String,
}

/// 図の SVG のパス（`figs/<項目>-<ケース>.svg`。ケースが無ければ `default`）。
pub fn figure_path(item: &str, case: Option<&str>) -> String {
    format!("{FIGURE_DIR}/{item}-{}.svg", case.unwrap_or("default"))
}

/// `data.json`・Typst のモジュール・例・図（パス順）を作る。
pub fn build_files(
    data: &ReportData,
    figures: &[(String, String)],
) -> Result<Vec<ReportFile>, ReportError> {
    let mut json = serde_json::to_string_pretty(data)?;
    json.push('\n');
    let mut files = vec![
        ReportFile {
            path: DATA_FILE.into(),
            content: json,
        },
        ReportFile {
            path: MODULE_FILE.into(),
            content: typst::MODULE.into(),
        },
        ReportFile {
            path: EXAMPLE_FILE.into(),
            content: typst::example(data),
        },
    ];
    let mut figs: Vec<ReportFile> = figures
        .iter()
        .map(|(path, svg)| ReportFile {
            path: path.clone(),
            content: svg.clone(),
        })
        .collect();
    figs.sort_by(|a, b| a.path.cmp(&b.path));
    files.extend(figs);
    Ok(files)
}
