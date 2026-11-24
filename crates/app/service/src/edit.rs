//! 編集の関数。**書き込みは [`apply`] だけ**（Op の列を fune-ops で計画・適用する）。CLI のコマンドも GUI の操作も Op にしてここを呼ぶ。
//! 人が打つ値の文字列（`"620 g"`）を属性の型に合わせて Op にするのも、属性定義を見る必要があるのでここで行う。
//!
//! | モジュール | 打った文字列 → Op |
//! |---|---|
//! | `text` | `fune set`・`fune unset`・`fune setting set|unset` のパスと値（[`set_op`]・[`unset_op`]）と、値の読み方 |
//! | `case_field` | `fune set|unset cases.<ID>.<欄>` のケースの欄 1 つ（`SetCaseField`。フェーズと時刻は `SetCasePhase`・`SetCaseEpoch`） |
//! | `catalog` | `fune catalog set|unset`・`fune catalog source add`・`add-local`（[`catalog_source_add_op`]・[`catalog_local_add_op`]） |
//! | `command` | 名詞のコマンドの引数（[`command_ops`]。`fune net set|unset`・`fune part place`・`fune mode set|unset`） |
//! | `configuration` | `fune configuration add\|set\|unset-pose` の引数（[`configuration_op`]・[`unset_pose_op`]。逆の向きは `query::configurations`） |
//! | `defs` | `fune attr add`・`fune kind add` |
//! | `eclipse` | 季節解析の代表標本から新しい解析ケースの SetCase を作る |
//! | `geometry_cases` | 軌道のミッション幾何の最悪の日（太陽距離の最大・最小、地球距離の最大）から `epoch` を持つ解析ケースの SetCase を作る・更新する |
//! | `sun_distance` | 太陽を回る軌道の近日点・遠日点から新しい解析ケースの SetCase を作る |
//! | `requirement` | `fune requirement add\|set` の引数（[`requirement_op`]。逆の向きは `query::requirements`） |
//! | `maneuver` | `fune maneuver add\|set` の引数（[`maneuver_op`]。逆の向きは `query::maneuvers`） |
//! | `phase` | `fune phase add\|set\|set-field` と `fune case set-phase` の引数（[`phase_op`]・[`phase_field_op`]・[`case_phase_op`]。逆の向きは `query::phases`） |
//! | `ground` | `fune ground add` の引数（[`ground_add_op`]） |
//! | `link` | `fune link add` の引数（[`link_add_op`]） |
//! | `project` | `fune project set\|unset` の引数（[`project_field_op`]） |
//! | `group` | `fune part mv|add` の置き先 `aocs/RW`（[`group_target`]。ID の列に読む） |
//! | `thermal` | 熱の結合・表面材・面の割合の下書き（[`thermal_set_ops`]。`fune set|unset analysis.thermal.…` と同じ `SetSetting` の列） |
//! | `place` | `fune part place` の取付・位置・姿勢（[`place_op`]。姿勢の角度 `--rotation` は [`rotation_from_text`]）と、機体座標での配置（`--body-pos`・`POST /parts/{id}/place_body`） |

use crate::diff;
use crate::dto::{ApplyRequest, ApplyResult, ChangedFile, FileChangeKind, InitResult};
use crate::error::{Result, ServiceError};
use crate::workspace::{Workspace, op_error, store_error, with_excerpt};
use fune_model::design::Revision;
use fune_model::path::parse_case_field;
use fune_ops::{ApplyOptions, Op};
use fune_store::FileSet;
use std::path::{Component, Path, PathBuf};

mod case_field;
mod catalog;
mod command;
mod configuration;
mod defs;
mod eclipse;
mod geometry_cases;
mod ground;
mod group;
mod link;
mod maneuver;
mod phase;
mod place;
mod project;
#[cfg(test)]
mod record_roundtrip;
mod requirement;
mod sun_distance;
mod text;
mod thermal;
mod window;

pub use catalog::{catalog_local_add_op, catalog_source_add_op};
pub use command::{
    command_ops, duty_from_text, mode_duration_op, mode_move_op, mode_set_op, mode_timing_op,
    net_set_ops, net_unset_op, sequence_set_op,
};
pub use configuration::{configuration_op, unset_pose_op};
pub use defs::{
    attr_add_op, attr_replace_op, kind_add_op, kind_replace_op, port_template_from_text,
};
pub use eclipse::case_from_eclipse_op;
pub use fune_ops::{to_cli, to_cli_script};
pub use geometry_cases::cases_from_geometry_ops;
pub use ground::ground_add_op;
pub use group::group_target;
pub use link::link_add_op;
pub use maneuver::maneuver_op;
pub use phase::{
    case_epoch_op, case_phase_op, copy_orbit_op, phase_field_op, phase_op, unset_law_op,
    unset_orbit_op,
};
pub use place::{place_body, place_body_op, place_op, rotation_from_text};
pub use project::project_field_op;
pub use requirement::requirement_op;
pub use sun_distance::case_from_sun_distance_op;
pub use text::{
    SetText, UnsetText, list_from_text, maturity, net_kind, pair_from_text, points_from_text,
    quantity_from_text, quaternion_from_text, vector_from_text, xy_from_text,
};
pub use thermal::thermal_set_ops;
pub use window::maneuver_window_op;

/// 人が打った値の文字列 → Op（`fune set`・`fune catalog set`・`POST /ops/from_text`）。
/// `item` があればカタログ品の値かフィールド（`SetCatalogValue`・`SetCatalogField`）、無ければ部品の値・フィールドか設定
/// （`SetValue`・`SetField`・`SetSetting`）、`cases.<ID>.<欄>` なら解析ケースの欄 1 つ（`SetCaseField`など。`case_field`）。値の型は属性定義・設定の今の型から決める（`ops` があれば、それを書き込まずに
/// 当てた設計の定義と型。同じ編集で作る属性の値を読むため）。
pub fn set_op(ws: &Workspace, t: &SetText) -> Result<Op> {
    match &t.item {
        Some(item) => catalog::catalog_set_op(ws, item, t),
        None => match parse_case_field(&t.path) {
            // 解析ケースの欄 1 つ（`cases.<ID>.<欄>`）
            Some(case) => {
                let (id, field) = case.map_err(text::invalid)?;
                case_field::case_field_op(&id, &field, Some(&t.value))
            }
            None => text::param_set_op(ws, t),
        },
    }
}

/// 人が打った消す先のパス → Op（`fune unset`・`fune catalog unset`・`POST /ops/unset_from_text`）。
/// `item` があれば `UnsetCatalogValue`・`SetCatalogField`（空）、無ければ `UnsetValue`・`SetField`（空）・`SetSetting`（消す）。
pub fn unset_op(t: &UnsetText) -> Result<Op> {
    match &t.item {
        Some(item) => catalog::catalog_unset_op(item, &t.path),
        None => match parse_case_field(&t.path) {
            Some(case) => {
                let (id, field) = case.map_err(text::invalid)?;
                case_field::case_field_op(&id, &field, None)
            }
            None => text::param_unset_op(&t.path),
        },
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
        removed: out.plan.removed,
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

/// 新しいプロジェクトを作る（`fune init [DIR] [--name NAME]`）。
///
/// - `target` があれば `base/target`（親ディレクトリも作る）、なければ `base` そのものをプロジェクトのルートにする。
/// - `name` がなければルートのディレクトリ名を使う（プロジェクト名として使えない名前なら、何も作らずに誤りにする）。
/// - 既にあるディレクトリは、`fune.toml` がなければ使う（雛形のうち既にあるファイルは上書きしない）。
/// - このコマンドが作ったディレクトリは、書き込みに失敗したら（空のものだけ）消す。
pub fn init(base: &Path, target: Option<&Path>, name: Option<&str>) -> Result<InitResult> {
    let joined = match target {
        Some(t) => base.join(t),
        None => base.to_path_buf(),
    };
    let dir = clean_absolute(&joined)?;
    let name = match name {
        Some(n) => n.to_owned(),
        None => derived_project_name(&dir)?,
    };
    let created_dirs = missing_ancestors(&dir);
    std::fs::create_dir_all(&dir)
        .map_err(|e| ServiceError::Store(format!("{}: {e}", dir.display())))?;
    let result = fune_store::project::init(&dir, &name)
        .map_err(store_error)
        .and_then(|created| {
            let ws = Workspace::open(&dir)?;
            Ok(InitResult {
                project: crate::query::project_info(&ws)?,
                created,
            })
        });
    if result.is_err() {
        // 深い方から、空のものだけ消す（`remove_dir` は空でなければ失敗するので、中身は消えない）
        for d in created_dirs.iter().rev() {
            let _ = std::fs::remove_dir(d);
        }
    }
    result
}

/// 絶対パスにして、`.` と `..` を文字の上で解く（まだ存在しないディレクトリにも使える）。
fn clean_absolute(path: &Path) -> Result<PathBuf> {
    let abs = std::path::absolute(path)
        .map_err(|e| ServiceError::Store(format!("{}: {e}", path.display())))?;
    let mut clean = PathBuf::new();
    for c in abs.components() {
        match c {
            Component::CurDir => {}
            Component::ParentDir => {
                clean.pop();
            }
            other => clean.push(other),
        }
    }
    Ok(clean)
}

/// ディレクトリ名からプロジェクト名を決める。使えない名前は `--name` を促す誤りにする。
fn derived_project_name(dir: &Path) -> Result<String> {
    let name = dir.file_name().and_then(|n| n.to_str()).unwrap_or_default();
    let usable = !name.trim().is_empty()
        && !name.contains(['"', '\\'])
        && !name.chars().any(char::is_control);
    if usable {
        Ok(name.to_owned())
    } else {
        Err(ServiceError::InvalidInput(format!(
            "Cannot use the directory name {name:?} as the project name; pass one with --name"
        )))
    }
}

/// まだ存在しない祖先ディレクトリ（浅い方から。`create_dir_all` が作るもの）。
fn missing_ancestors(dir: &Path) -> Vec<PathBuf> {
    let mut missing: Vec<PathBuf> = dir
        .ancestors()
        .take_while(|a| !a.as_os_str().is_empty() && !a.exists())
        .map(Path::to_path_buf)
        .collect();
    missing.reverse();
    missing
}
