//! 設計書に入れる表と図の書き出し（`fune report export`・`POST /report/export`）。
//! 質量表・モードごとの電力表・回線バジェットの表・温度の結果表・温度の時間変化の図・発電と電池の時間変化の図を、
//! 解析とバジェットの結果から組み立て、`fune-report` が `data.json`・SVG・Typst のモジュールのファイルにする。
//! **ここは読むだけで、物理も判定も再計算しない**（値は解析・バジェットの結果、整形は `sat_units::format`、判定は解析の判定）。
//! ファイルはディスクに書かない（内容を返す。CLI が `--out` に書き、HTTP は本文として返す）ので、設計ファイルの書き込みの例外を増やさない。
//!
//! | モジュール | 責務 |
//! |---|---|
//! | `common` | 実行の共通の入口・ファイルの脚注・判定と状態の表示名 |
//! | `column` | 列の単位の選び方と、数値だけのセル |
//! | `json` | 解析の出力（JSON）の読み取り |
//! | `mass` | 質量表（バジェットと部品の行） |
//! | `power` | モードごとの電力表 |
//! | `link` | 回線バジェットの入力表と結果表（回線ごと） |
//! | `thermal` | 温度の結果表 |
//! | `history` | 温度の時間変化の図・発電と電池の状態の時間変化の図 |
//! | `values` | 文章から 1 つの値を引くための項目（KPI・部品の質量・質量バジェット） |

use crate::dto::{ReportExport, ReportItem, ReportRequest, ReportUnavailable};
use crate::error::{Result, ServiceError};
use crate::workspace::{Snapshot, Workspace};
use fune_report::model::{Figure, ReportData, SCHEMA, Table, ValueEntry};
use fune_report::{ReportLang, build_files};
use sat_analysis::spec::Analysis;

mod column;
mod common;
mod history;
mod json;
mod link;
mod mass;
mod power;
mod thermal;
mod values;

/// 書き出しの途中の値（組み立てた表・図・値と、数えられなかった項目）。
#[derive(Default)]
struct Out {
    tables: Vec<Table>,
    figures: Vec<Figure>,
    /// 図の SVG（パスと本文）
    svgs: Vec<(String, String)>,
    values: Vec<ValueEntry>,
    unavailable: Vec<ReportUnavailable>,
    /// 書き出したケースの ID（重複なし、書いた順）
    cases: Vec<String>,
}

/// 書き出しの間、組み立てが共有する値。
struct Ctx<'a> {
    ws: &'a Workspace,
    lang: ReportLang,
    snapshot: Snapshot,
}

/// 書き出し先の既定（プロジェクトの根からの相対。CLI の `--out` の既定）。派生のファイルなので、設計のリポジトリでは無視してよい。
pub const DEFAULT_DIR: &str = "exports/report";

/// `"ja"`・`"en"` から言語を読む。
pub fn parse_lang(text: &str) -> Result<ReportLang> {
    match text {
        "ja" => Ok(ReportLang::Ja),
        "en" => Ok(ReportLang::En),
        other => Err(ServiceError::InvalidInput(format!(
            "Unknown report language `{other}` (ja or en)"
        ))),
    }
}

/// `"mass"` などの文字列から項目を読む（`--item` と同じ綴り。人が打った文字列の解釈はここだけ）。
pub fn parse_item(text: &str) -> Result<ReportItem> {
    ReportItem::parse(text).ok_or_else(|| {
        let known: Vec<&str> = ReportItem::ALL.iter().map(|i| i.as_str()).collect();
        ServiceError::InvalidInput(format!(
            "Unknown report item `{text}` (one of: {})",
            known.join(", ")
        ))
    })
}

/// 表・図・値を組み立て、書き出すファイルの内容を返す。入力が足りない項目は「算出できません」の注記つきで書き出し、
/// `unavailable` に並べる（全体を失敗にしない）。同じ入力なら同じバイト列になる。
pub fn export(ws: &Workspace, req: &ReportRequest) -> Result<ReportExport> {
    let snapshot = ws.snapshot()?;
    let cases = request_cases(&snapshot, req)?;
    let items: Vec<ReportItem> = ReportItem::ALL
        .into_iter()
        .filter(|i| req.items.is_empty() || req.items.contains(i))
        .collect();
    let revision = snapshot.design.revision.0.clone();
    let project = snapshot
        .design
        .project
        .meta
        .label
        .clone()
        .unwrap_or_else(|| snapshot.design.project.meta.name.clone());
    let ctx = Ctx {
        ws,
        lang: req.lang,
        snapshot,
    };
    let mut out = Out::default();
    for &item in &items {
        build_item(&ctx, item, &cases, &mut out)?;
    }
    out.values.sort_by(|a, b| a.key.cmp(&b.key));
    out.values.dedup_by(|a, b| a.key == b.key);
    let data = ReportData {
        schema: SCHEMA,
        revision: revision.clone(),
        project,
        lang: req.lang,
        items,
        cases: out.cases,
        tables: out.tables,
        figures: out.figures,
        values: out.values,
    };
    let files = build_files(&data, &out.svgs).map_err(|e| ServiceError::Internal(e.to_string()))?;
    Ok(ReportExport {
        revision,
        files,
        unavailable: out.unavailable,
    })
}

/// 項目 1 つを組み立てる（ケースによる項目はケースごとに繰り返す）。
fn build_item(
    ctx: &Ctx<'_>,
    item: ReportItem,
    cases: &[Option<String>],
    out: &mut Out,
) -> Result<()> {
    match item {
        ReportItem::Mass => mass::build(ctx, out),
        ReportItem::ModePower => power::build(ctx, out),
        ReportItem::LinkBudget => link::build(ctx, out),
        ReportItem::ThermalResult => each(cases, |c| thermal::build(ctx, c, out)),
        ReportItem::ThermalHistory => each(cases, |c| history::thermal(ctx, c, out)),
        ReportItem::PowerHistory => each(cases, |c| history::power(ctx, c, out)),
    }
}

fn each(cases: &[Option<String>], mut f: impl FnMut(Option<&str>) -> Result<()>) -> Result<()> {
    cases.iter().try_for_each(|c| f(c.as_deref()))
}

/// 依頼のケース。空なら、解析の既定のケース（`analysis/common.toml` の `default_case`。時系列の解析が使うもの）で 1 回だけ
/// （設計に無ければケースなし）。すべての項目に同じケースを明示して渡すので、表と図の `case` はそろう。設計に無いケースは見つからないエラー。
fn request_cases(snapshot: &Snapshot, req: &ReportRequest) -> Result<Vec<Option<String>>> {
    if req.cases.is_empty() {
        let (default, _) = fune_projection::case_policy::effective_case(
            Some(&snapshot.design),
            sat_analysis::thermal::transient::ThermalTransient::ID,
            None,
        );
        return Ok(vec![default.map(str::to_owned)]);
    }
    let mut out: Vec<Option<String>> = Vec::new();
    for id in &req.cases {
        if !snapshot.design.cases.contains_key(id.as_str()) {
            return Err(ServiceError::not_found("case", id));
        }
        if !out.contains(&Some(id.clone())) {
            out.push(Some(id.clone()));
        }
    }
    Ok(out)
}
