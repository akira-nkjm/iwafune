//! API の TS 書き出し。
use crate::dto::*;
mod names;
use std::path::{Path, PathBuf};
use ts_rs::{Config, TS};

/// TS の書き出しのエラー。
#[derive(Debug, thiserror::Error)]
pub enum ExportError {
    #[error("TS の型を書き出せません: {0}")]
    Ts(#[from] ts_rs::ExportError),
    #[error("TS の型名が重複しています（registry.md §4）: {0}")]
    DuplicateNames(String),
    #[error("{path}: {source}")]
    Io {
        path: PathBuf,
        source: std::io::Error,
    },
}

macro_rules! export_types {
    ($cfg:expr, $names:expr; $($t:ty),+ $(,)?) => {{
        $( <$t as TS>::export_all($cfg)?; $names.visit::<$t>(); )+
    }};
}

/// API のすべての型（と、それが参照する型）を `dir` に TS として書き出し、`index.ts` で再公開する。
/// 先に `dir` の `.ts` を消してから書くので、消えた型のファイルは残らない。出力は決定的。
pub fn export_ts(dir: &Path) -> Result<Vec<PathBuf>, ExportError> {
    clear_ts_files(dir)?;
    let cfg = Config::new().with_out_dir(dir).with_large_int("number");
    let cfg = &cfg;
    let mut names = names::NameCheck::new(cfg);
    export_types!(cfg, names;
        fune_store::FileChange,
        FormatDesignRequest,
        FormatDesignResult,
        DataStatus,
        DataItem,
        SpaceWeatherLookup,
        SpaceWeatherVersion,
        ErrorEnvelope,
        QuantityView,
        DiagnosticCounts,
        CheckReport,
        ProjectInfo,
        DesignView,
        SystemModesView,
        PartView,
        PartViewRequest,
        PartsQuery,
        ValueQuery,
        ValueAnswer,
        ApplyRequest,
        ApplyResult,
        PlaceBodyRequest,
        SetText,
        UnsetText,
        CommandText,
        CaseFromEclipseRequest,
        InitResult,
        BudgetKind,
        BudgetRequest,
        BudgetView,
        UnitsCatalog,
        UnitsQuery,
        ParseQuantityRequest,
        ParsedQuantity,
        FormatRequest,
        FormatResponse,
        DisplaySeries,
        PowerPageView,
        PowerPageQuery,
        AnalysisEntry,
        RunAnalysisRequest,
        AnalysisRunResult,
        CheckView,
    );
    fune_projection::dispatch::export_ts(cfg)?;
    fune_projection::dispatch::visit_ts(&mut names);
    names.finish()?;
    write_analysis_outputs(dir)?;
    write_index(dir)
}

fn write_analysis_outputs(dir: &Path) -> Result<(), ExportError> {
    let mut text = String::from("// Generated from fune_projection::dispatch. Do not edit.\n");
    for row in fune_projection::entries() {
        text.push_str(&format!(
            "import type {{ {} }} from \"./{}\";\n",
            row.output_type, row.output_type
        ));
    }
    text.push_str("export type AnalysisOutputs = {\n");
    for row in fune_projection::entries() {
        text.push_str(&format!("  \"{}\": {};\n", row.id, row.output_type));
    }
    text.push_str("};\n");
    let path = dir.join("AnalysisOutputs.ts");
    std::fs::write(&path, text).map_err(io_err(&path))
}

fn io_err(path: &Path) -> impl FnOnce(std::io::Error) -> ExportError + '_ {
    move |source| ExportError::Io {
        path: path.to_owned(),
        source,
    }
}

fn ts_files(dir: &Path) -> Result<Vec<PathBuf>, ExportError> {
    let mut files = Vec::new();
    if !dir.exists() {
        return Ok(files);
    }
    for entry in std::fs::read_dir(dir).map_err(io_err(dir))? {
        let path = entry.map_err(io_err(dir))?.path();
        if path.extension().is_some_and(|e| e == "ts") {
            files.push(path);
        }
    }
    files.sort();
    Ok(files)
}

fn clear_ts_files(dir: &Path) -> Result<(), ExportError> {
    for path in ts_files(dir)? {
        std::fs::remove_file(&path).map_err(io_err(&path))?;
    }
    Ok(())
}

fn write_index(dir: &Path) -> Result<Vec<PathBuf>, ExportError> {
    let files = ts_files(dir)?;
    let mut text = String::from(
        "// 生成物です。手で編集しないでください（`cargo test -p fune-service export_ts` で作り直す）。\n",
    );
    for path in &files {
        if let Some(stem) = path.file_stem().and_then(|s| s.to_str()) {
            text.push_str(&format!("export type * from \"./{stem}\";\n"));
        }
    }
    let index = dir.join("index.ts");
    std::fs::write(&index, text).map_err(io_err(&index))?;
    let mut all = files;
    all.push(index);
    Ok(all)
}
