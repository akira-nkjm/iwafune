//! Op の列 → ファイルの変更計画（適用後の検証込み）。**書き込みはしない。**
//!
//! Op を 1 つずつ解釈して store の編集（[`fune_store::Edits`]）に変え、そのたびに Design を読み直す。
//! 最後に適用前と適用後の `check` を比べ、新しく増えた error があれば [`OpError::Rejected`] で拒む
//! （元からある error と、取得で直る共有カタログの状態 [`fune_model::check::FETCH_STATE_CODES`] は編集を妨げない）。
//!
//! | モジュール | 中身 |
//! |---|---|
//! | `parts` | 部品の Op（追加・削除・名前・移動・値・フィールド・取付・ポート） |
//! | `groups` | 部品の置き先（グループ・組立品）の解釈（親の連鎖から `parts/` の下のディレクトリの列を作る。部品の Op が使う） |
//! | `defs` | 定義（属性・種別）の Op（足す・置き換える・消す） |
//! | `others` | モード・ネット・図・設定の Op |
//! | `records` | 解析ケース・出典・ハーネス・マヌーバの Op |
//! | `phases` | フェーズの Op（足す・書く・消す・名前を変える・軌道と姿勢則の値・ケースのフェーズ） |
//! | `configurations` | 形態の Op（足す・見出しを書く・消す・名前を変える・部品の姿勢の上書きを消す）と、形態の中への配置（`Place` の `configuration`） |
//! | `ground` | 地上局の Op（足す・消す・名前を変える） |
//! | `check_keys` | 要求の判定のキーが名指すエンティティ（部品・局・回線・熱の節点）の削除の拒否 |
//! | `remount` | 基準の取付の付け替えで形態の上書きが古い取付点に対するものになるときの警告（`configuration.mount_changed`） |
//! | `renamed` | 名前の変更の Op を適用前の診断に写す（元からある error を新しい error と取り違えない） |
//! | `renames` | ネット・ハーネス・解析ケース・出典・マヌーバの名前の変更の Op |
//! | `link` | 通信の回線の Op（足す・消す・名前を変える） |
//! | `project` | `fune.toml` の `[project]` の Op |
//! | `requirements` | 要求の Op（足す・書く・消す） |
//! | `catalog` | カタログ品の Op（このプロジェクトの `catalog/` だけ） |
//! | `catalog_source` | カタログの出所の Op（fune.toml の `[[catalog]]`。共有カタログ `git = …` とローカルのカタログ `path = …` を足す・外す） |
//! | `subdir` | ディレクトリの下に置けるエンティティ（カタログ品・出典）の Op に共通の計画（ディレクトリを選ぶ・移す） |
//! | `acknowledged` | 承知済みの組（幾何の所見）の Op（足す・取り消す） |
//! | `attr` | 属性の値の書き込み（部品とカタログ品で共有） |

mod acknowledged;
mod attr;
mod catalog;
mod catalog_source;
mod check_keys;
mod configurations;
mod defs;
mod ground;
mod groups;
mod link;
mod others;
mod parts;
mod phases;
mod project;
mod records;
mod remount;
mod renamed;
mod renames;
mod requirements;
mod subdir;

use crate::error::OpError;
use crate::op::Op;
use fune_model::check::FETCH_STATE_CODES;
use fune_model::design::Revision;
use fune_model::{Design, Diagnostic, check};
use fune_store::read::design_of;
use fune_store::{Edits, FileChange, Snapshot, revision};
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;
use ts_rs::TS;

/// 変更計画。
#[derive(Debug, Clone)]
pub struct Plan {
    /// 計画の元にした revision
    pub base_revision: Revision,
    /// 適用後の revision
    pub revision: Revision,
    pub changes: Vec<FileChange>,
    /// 適用後の設計の診断（全件）
    pub diagnostics: Vec<Diagnostic>,
    /// 適用後の設計
    pub design: Design,
}

/// 計画の要約（API・CLI の `--dry-run` 向け）。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
pub struct PlanSummary {
    pub base_revision: Revision,
    pub revision: Revision,
    pub changes: Vec<FileChange>,
    pub diagnostics: Vec<Diagnostic>,
}

impl From<&Plan> for PlanSummary {
    fn from(plan: &Plan) -> Self {
        Self {
            base_revision: plan.base_revision.clone(),
            revision: plan.revision.clone(),
            changes: plan.changes.clone(),
            diagnostics: plan.diagnostics.clone(),
        }
    }
}

/// Op の列を計画する。
pub fn plan(snapshot: &Snapshot, ops: &[Op]) -> Result<Plan, OpError> {
    let mut edits = Edits::new(&snapshot.files);
    let mut design = snapshot.design.clone();
    let mut remounts = Vec::new();
    for op in ops {
        remounts.extend(remount::remount_of(&design, op));
        interpret(&mut edits, &design, op)?;
        design = design_of(edits.files());
    }
    let before = check(&snapshot.design);
    let after = check(&design);
    let renames: Vec<(&str, &str)> = ops.iter().filter_map(renamed::rename_of).collect();
    let new_errors = new_errors(&before, &after, &renames);
    if !new_errors.is_empty() {
        return Err(OpError::Rejected {
            diagnostics: new_errors,
        });
    }
    let mut diagnostics = after;
    diagnostics.extend(remount::diagnostics(&design, &remounts));
    Ok(Plan {
        base_revision: snapshot.design.revision.clone(),
        revision: revision::of(edits.files()),
        changes: edits.changes(),
        diagnostics,
        design,
    })
}

/// 適用後に新しく増えた error（位置は比べない。編集で行がずれ、名前の変更でファイルも変わるため）。
/// 取得で直る共有カタログの状態（共有カタログを足した直後の `catalog.not_locked` など）は数えない。
/// 名前の変更があれば、適用前の error の対象と文の中の ID を新しい名前に替えてから比べる（`renamed`）。
fn new_errors(
    before: &[Diagnostic],
    after: &[Diagnostic],
    renames: &[(&str, &str)],
) -> Vec<Diagnostic> {
    let old: BTreeSet<_> = before
        .iter()
        .filter(|d| d.is_error())
        .map(|d| renamed::through_renames(d, renames))
        .collect();
    after
        .iter()
        .filter(|d| d.is_error() && !FETCH_STATE_CODES.contains(&d.code.as_str()))
        .filter(|d| !old.contains(&renamed::key(d)))
        .cloned()
        .collect()
}

/// Op 1 つを編集に変える。
fn interpret(edits: &mut Edits, design: &Design, op: &Op) -> Result<(), OpError> {
    match op {
        Op::CreatePart { .. }
        | Op::RemovePart { .. }
        | Op::RenamePart { .. }
        | Op::RenameGroup { .. }
        | Op::MovePart { .. }
        | Op::SetValue { .. }
        | Op::UnsetValue { .. }
        | Op::SetField { .. }
        | Op::Place { .. }
        | Op::SetPort { .. }
        | Op::SetMountPoint { .. } => parts::interpret(edits, design, op),
        Op::CreateAttribute { .. }
        | Op::UpdateAttribute { .. }
        | Op::DeleteAttribute { .. }
        | Op::CreateKind { .. }
        | Op::UpdateKind { .. }
        | Op::DeleteKind { .. }
        | Op::RenameAttribute { .. }
        | Op::RenameKind { .. } => defs::interpret(edits, design, op),
        Op::CreateGroundStation { .. }
        | Op::RemoveGroundStation { .. }
        | Op::RenameGroundStation { .. } => ground::interpret(edits, design, op),
        Op::CreateCatalogItem { .. }
        | Op::SetCatalogValue { .. }
        | Op::UnsetCatalogValue { .. }
        | Op::SetCatalogField { .. }
        | Op::RemoveCatalogItem { .. }
        | Op::RenameCatalogItem { .. }
        | Op::MoveCatalogItem { .. } => catalog::interpret(edits, design, op),
        Op::AddCatalogSource { .. }
        | Op::AddLocalCatalog { .. }
        | Op::RemoveCatalogSource { .. } => catalog_source::interpret(edits, design, op),
        Op::RenameNet { .. }
        | Op::RenameHarness { .. }
        | Op::RenameCase { .. }
        | Op::RenameSource { .. }
        | Op::RenameManeuver { .. } => renames::interpret(edits, design, op),
        Op::CreateCommLink { .. } | Op::RemoveCommLink { .. } | Op::RenameCommLink { .. } => {
            link::interpret(edits, design, op)
        }
        Op::SetProjectField { .. } => project::interpret(edits, op),
        Op::SetCase { .. }
        | Op::RemoveCase { .. }
        | Op::SetSource { .. }
        | Op::RemoveSource { .. }
        | Op::MoveSource { .. }
        | Op::SetHarness { .. }
        | Op::RemoveHarness { .. }
        | Op::CreateManeuver { .. }
        | Op::SetManeuver { .. }
        | Op::RemoveManeuver { .. } => records::interpret(edits, design, op),
        Op::CreatePhase { .. }
        | Op::SetPhase { .. }
        | Op::RemovePhase { .. }
        | Op::RenamePhase { .. }
        | Op::SetPhaseField { .. }
        | Op::CopyPhaseOrbit { .. }
        | Op::UnsetPhaseOrbit { .. }
        | Op::UnsetPhaseLaw { .. }
        | Op::SetCasePhase { .. } => phases::interpret(edits, design, op),
        Op::CreateConfiguration { .. }
        | Op::SetConfiguration { .. }
        | Op::RemoveConfiguration { .. }
        | Op::RenameConfiguration { .. }
        | Op::UnsetConfigurationPose { .. } => configurations::interpret(edits, design, op),
        Op::Acknowledge { .. } | Op::Unacknowledge { .. } => {
            acknowledged::interpret(edits, design, op)
        }
        Op::CreateRequirement { .. } | Op::SetRequirement { .. } | Op::RemoveRequirement { .. } => {
            requirements::interpret(edits, design, op)
        }
        _ => others::interpret(edits, design, op),
    }
}
