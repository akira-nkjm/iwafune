//! 読み取りの関数（設計全体・部品・値・定義・診断）。どれもディスクから読み直したスナップショットを見る
//! （`part_view` は、依頼の `ops` があれば書き込まずに当てたスナップショット。`preview::snapshot_with`）。
//!
//! | モジュール | 中身 |
//! |---|---|
//! | `views` | Design → 製品ツリー・モード表・ネット・ハーネス・定義・ケース |
//! | `parts` | 部品の行・1 部品の全情報・1 つの値 |
//! | `maneuvers` | マヌーバ（実行の順・目標の要素・編集の欄の初期値） |
//! | `catalog` | カタログ品（値・パス・使っている部品・書き換えられるか） |
//! | `sources` | 出典の台帳（Web のリンク・ファイル）とデータシートのリンク |
//! | `values` | 解決済みの値・書かれた量 → 表示用の型 |
//! | `settings` | 設定 → キーと値の一覧、設定・バジェットと系の対応 |

mod catalog;
pub(crate) mod maneuvers;
mod parts;
pub(crate) mod settings;
mod sources;
pub(crate) mod values;
pub(crate) mod views;

use crate::dto::{
    CheckReport, DefsView, DesignCounts, DesignView, DiagnosticCounts, PartRow, PartView,
    PartViewRequest, PartsQuery, ProjectInfo, ValueAnswer, ValueQuery,
};
use crate::error::Result;
use crate::workspace::{Snapshot, Workspace};

/// 組み込みの列挙の値の表示名（正本は `fune_model::builtin` の labels.toml。`ty` は `origin`・`evidence`・`maturity` など）。
/// 表に無ければ値の綴り（`kind_default` など）。CLI の表示に使う（GUI は `GET /defs` の `labels` を引く）。
pub fn enum_label<T: std::fmt::Display>(ty: &str, value: &T) -> String {
    let code = value.to_string();
    fune_model::builtin::enum_label(ty, &code).map_or(code, |l| l.label.clone())
}

/// API の系の ID。綴りは model の系から取り、CLI は列挙の対応を持たない。
pub fn system_id(system: sat_analysis::spec::System) -> &'static str {
    fune_projection::system::from_analysis(system).as_str()
}

/// プロジェクトの概要（`GET /project`）。
pub fn project_info(ws: &Workspace) -> Result<ProjectInfo> {
    Ok(info_of(ws, &ws.snapshot()?))
}

fn info_of(ws: &Workspace, s: &Snapshot) -> ProjectInfo {
    let d = &s.design;
    let count = |n: usize| u32::try_from(n).unwrap_or(u32::MAX);
    ProjectInfo {
        name: d.project.meta.name.clone(),
        root: ws.root().display().to_string(),
        revision: d.revision.0.clone(),
        branch: ws.git_branch(),
        schema: d.project.meta.schema,
        counts: DesignCounts {
            parts: count(d.parts.len()),
            groups: count(d.groups.len()),
            modes: count(d.modes.len()),
            nets: count(d.nets.len()),
            harnesses: count(d.harnesses.len()),
            cases: count(d.cases.len()),
        },
        diagnostics: DiagnosticCounts::of(&s.diagnostics),
        catalogs: catalog::sources(d),
    }
}

/// 画面表示用の設計全体（`GET /design`）。
pub fn design_view(ws: &Workspace) -> Result<DesignView> {
    let s = ws.snapshot()?;
    let d = &s.design;
    let parts = parts::list(d, &s.resolved, &PartsQuery::default());
    Ok(DesignView {
        project: info_of(ws, &s),
        tree: views::tree(d),
        parts,
        modes: views::mode_table(d),
        nets: views::nets(d),
        harnesses: views::harnesses(d),
        defs: views::defs(d),
        settings: settings::settings(d),
        cases: views::cases(d),
        maneuvers: maneuvers::maneuvers(d),
        sources: sources::sources(d),
        catalog: catalog::list(d),
        views: d.views.values().cloned().collect(),
        diagnostics: s.diagnostics.clone(),
    })
}

/// マヌーバの一覧（実行の順。`GET /maneuvers`・`fune maneuver ls`）。
pub fn maneuvers(ws: &Workspace) -> Result<Vec<crate::dto::ManeuverView>> {
    Ok(maneuvers::maneuvers(&ws.snapshot()?.design))
}

/// 部品の一覧（`GET /parts`）。
pub fn list_parts(ws: &Workspace, q: &PartsQuery) -> Result<Vec<PartRow>> {
    let s = ws.snapshot()?;
    Ok(parts::list(&s.design, &s.resolved, q))
}

/// 1 部品の全情報（`GET /parts/{id}`・`fune part show`）。`req.ops` があれば、それを書き込まずに当てた設計で表示する
/// （`POST /parts/{id}`・`fune part show --ops`。同じ編集で定義を作る属性のパスも `paths` に入る）。
pub fn part_view(ws: &Workspace, id: &str, req: &PartViewRequest) -> Result<PartView> {
    let s = crate::preview::snapshot_with(ws, &req.ops)?;
    parts::view(&s.design, &s.resolved, &s.diagnostics, id)
}

/// 1 つの値（`GET /values/{path}`）。
pub fn get_value(ws: &Workspace, path: &str, q: &ValueQuery) -> Result<ValueAnswer> {
    let s = ws.snapshot()?;
    parts::get_value(&s.resolved, path, q)
}

/// 属性定義と種別定義（`GET /defs`）。
pub fn defs_view(ws: &Workspace) -> Result<DefsView> {
    Ok(views::defs(&ws.snapshot()?.design))
}

/// 検証（`GET /diagnostics`）。
pub fn check(ws: &Workspace) -> Result<CheckReport> {
    let s = ws.snapshot()?;
    Ok(CheckReport {
        revision: s.design.revision.0.clone(),
        counts: DiagnosticCounts::of(&s.diagnostics),
        diagnostics: s.diagnostics,
    })
}
