//! 編集の関数。**書き込みは [`apply`] だけ**（Op の列を fune-ops で計画・適用する）。CLI のコマンドも GUI の操作も Op にしてここを呼ぶ。
//! 人が打つ値の文字列（`"620 g"`）を属性の型に合わせて Op にするのも、属性定義を見る必要があるのでここで行う。
//!
//! | モジュール | 打った文字列 → Op |
//! |---|---|
//! | `text` | `fune set`・`fune unset`・`fune setting set|unset` のパスと値（[`set_op`]・[`unset_op`]）と、値の読み方 |
//! | `catalog` | `fune catalog set|unset`・`fune catalog source add`・`add-local`（[`catalog_source_add_op`]・[`catalog_local_add_op`]） |
//! | `command` | 名詞のコマンドの引数（[`command_ops`]。`fune net set|unset`・`fune part place`・`fune mode set|unset`） |
//! | `defs` | `fune attr add`・`fune kind add` |
//! | `eclipse` | 季節解析の代表標本から新しい解析ケースの SetCase を作る |
//! | `sun_distance` | 太陽を回る軌道の近日点・遠日点から新しい解析ケースの SetCase を作る |
//! | `requirement` | `fune requirement add\|set` の引数（[`requirement_op`]。逆の向きは `query::requirements`） |
//! | `maneuver` | `fune maneuver add\|set` の引数（[`maneuver_op`]。逆の向きは `query::maneuvers`） |
//! | `phase` | `fune phase add\|set\|set-field` と `fune case set-phase` の引数（[`phase_op`]・[`phase_field_op`]・[`case_phase_op`]。逆の向きは `query::phases`） |
//! | `ground` | `fune ground add` の引数（[`ground_add_op`]） |
//! | `link` | `fune link add` の引数（[`link_add_op`]） |
//! | `project` | `fune project set\|unset` の引数（[`project_field_op`]） |
//! | `group` | `fune part mv|add` の置き先 `aocs/RW`（[`group_target`]。ID の列に読む） |
//! | `place` | `fune part place` の取付・位置・姿勢（[`place_op`]。姿勢の角度 `--rotation` は [`rotation_from_text`]）と、機体座標での配置（`--body-pos`・`POST /parts/{id}/place_body`） |

use crate::diff;
use crate::dto::{ApplyRequest, ApplyResult, ChangedFile, FileChangeKind, InitResult};
use crate::error::{Result, ServiceError};
use crate::workspace::{Workspace, op_error, store_error, with_excerpt};
use fune_model::design::Revision;
use fune_ops::{ApplyOptions, Op};
use fune_store::FileSet;
use std::path::Path;

mod catalog;
mod command;
mod defs;
mod eclipse;
mod ground;
mod group;
mod link;
mod maneuver;
mod phase;
mod place;
mod project;
mod requirement;
mod sun_distance;
mod text;
mod window;

pub use catalog::{catalog_local_add_op, catalog_source_add_op};
pub use command::{
    command_ops, duty_from_text, mode_duration_op, mode_move_op, mode_set_op, mode_timing_op,
    net_set_ops, net_unset_op, sequence_set_op,
};
pub use defs::{
    attr_add_op, attr_replace_op, kind_add_op, kind_replace_op, port_template_from_text,
};
pub use eclipse::case_from_eclipse_op;
pub use fune_ops::{to_cli, to_cli_script};
pub use ground::ground_add_op;
pub use group::group_target;
pub use link::link_add_op;
pub use maneuver::maneuver_op;
pub use phase::{
    case_phase_op, copy_orbit_op, phase_field_op, phase_op, unset_law_op, unset_orbit_op,
};
pub use place::{place_body, place_body_op, place_op, rotation_from_text};
pub use project::project_field_op;
pub use requirement::requirement_op;
pub use sun_distance::case_from_sun_distance_op;
pub use text::{
    SetText, UnsetText, list_from_text, maturity, net_kind, pair_from_text, points_from_text,
    quantity_from_text, quaternion_from_text, vector_from_text, xy_from_text,
};
pub use window::maneuver_window_op;

/// 人が打った値の文字列 → Op（`fune set`・`fune catalog set`・`POST /ops/from_text`）。
/// `item` があればカタログ品の値かフィールド（`SetCatalogValue`・`SetCatalogField`）、無ければ部品の値・フィールドか設定
/// （`SetValue`・`SetField`・`SetSetting`）。値の型は属性定義・設定の今の型から決める（`ops` があれば、それを書き込まずに
/// 当てた設計の定義と型。同じ編集で作る属性の値を読むため）。
pub fn set_op(ws: &Workspace, t: &SetText) -> Result<Op> {
    match &t.item {
        Some(item) => catalog::catalog_set_op(ws, item, t),
        None => text::param_set_op(ws, t),
    }
}

/// 人が打った消す先のパス → Op（`fune unset`・`fune catalog unset`・`POST /ops/unset_from_text`）。
/// `item` があれば `UnsetCatalogValue`・`SetCatalogField`（空）、無ければ `UnsetValue`・`SetField`（空）・`SetSetting`（消す）。
pub fn unset_op(t: &UnsetText) -> Result<Op> {
    match &t.item {
        Some(item) => catalog::catalog_unset_op(item, &t.path),
        None => text::param_unset_op(&t.path),
    }
}

/// Op の一括適用（`POST /ops`）。全部が通るか、何も書かないか。
pub fn apply(ws: &Workspace, req: &ApplyRequest) -> Result<ApplyResult> {
    let options = ApplyOptions {
        base_revision: req.base_revision.clone().map(Revision),
        dry_run: req.dry_run,
    };
    let out = fune_ops::apply(ws.root(), &req.ops, &options).map_err(|e| op_error(ws, e))?;
    let changed = changed_files(&out.plan.changes);
    // 診断の抜粋は、変わったファイルの適用後の本文から取る
    let mut after = FileSet::new();
    fune_store::apply_changes(&mut after, &out.plan.changes);
    let diagnostics = out
        .plan
        .diagnostics
        .into_iter()
        .map(|d| with_excerpt(d, &after))
        .collect();
    Ok(ApplyResult {
        revision: if out.written {
            out.plan.revision.0
        } else {
            out.plan.base_revision.0.clone()
        },
        previous_revision: out.plan.base_revision.0,
        dry_run: !out.written,
        changed,
        diagnostics,
        cli: req.ops.iter().flat_map(fune_ops::to_cli).collect(),
    })
}

/// 変わったファイルの一覧（同じ本文の削除と新規の組は移動とみなす）。
pub(crate) fn changed_files(changes: &[fune_store::FileChange]) -> Vec<ChangedFile> {
    let mut out = Vec::new();
    let mut moved_from: Vec<&str> = Vec::new();
    for c in changes {
        let (kind, from) = match (&c.before, &c.after) {
            (None, Some(after)) => match changes
                .iter()
                .find(|d| d.after.is_none() && d.before.as_ref() == Some(after))
            {
                Some(d) => {
                    moved_from.push(&d.path);
                    (FileChangeKind::Renamed, Some(d.path.clone()))
                }
                None => (FileChangeKind::Created, None),
            },
            (Some(_), None) => (FileChangeKind::Deleted, None),
            _ => (FileChangeKind::Modified, None),
        };
        let before = c.before.as_deref().unwrap_or_default();
        let after = c.after.as_deref().unwrap_or_default();
        out.push(ChangedFile {
            path: c.path.clone(),
            change: kind,
            diff: diff::unified(
                &c.path,
                if kind == FileChangeKind::Renamed {
                    after
                } else {
                    before
                },
                after,
            ),
            from,
        });
    }
    out.retain(|f| !(f.change == FileChangeKind::Deleted && moved_from.contains(&f.path.as_str())));
    out
}

/// 新しいプロジェクトを作る（`fune init`）。
pub fn init(dir: &Path, name: &str) -> Result<InitResult> {
    std::fs::create_dir_all(dir)
        .map_err(|e| ServiceError::Store(format!("{}: {e}", dir.display())))?;
    let created = fune_store::project::init(dir, name).map_err(store_error)?;
    let ws = Workspace::open(dir)?;
    Ok(InitResult {
        project: crate::query::project_info(&ws)?,
        created,
    })
}
