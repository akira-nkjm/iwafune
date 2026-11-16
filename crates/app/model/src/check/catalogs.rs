//! 共有カタログの検査: `catalog.lock` が無い・lock と fune.toml が食い違う・lock の commit のキャッシュが無い・
//! lock に使っていない記録がある（docs/model.md §12）。状態の判定は `Design::shared_catalog_statuses` が持つ。

use super::Ctx;
use crate::design::{EntityKey, SharedCatalogState, SharedCatalogStatus};

/// 取得（`fune catalog fetch`・`update`）で直る状態の error のコード。設計の誤りではないので、編集の計画（fune-ops）は
/// 新しく出ても編集を拒まない（共有カタログを足した直後は lock に記録が無い）。
pub const FETCH_STATE_CODES: &[&str] = &[
    "catalog.not_locked",
    "catalog.lock_mismatch",
    "catalog.not_fetched",
];

/// commit を表示するときの桁数。
const SHORT_COMMIT: usize = 12;

/// 共有カタログの lock とキャッシュの検査。診断は fune.toml の `[[catalog]]` の行に付ける。
pub(super) fn check_shared_catalogs(ctx: &mut Ctx) {
    let design = ctx.design;
    for status in design.shared_catalog_statuses() {
        let Some((code, message)) = problem(ctx, &status) else {
            continue;
        };
        ctx.error(code, &EntityKey::Project, &["catalog"], None, message);
    }
    for unused in design.unused_lock_entries() {
        ctx.warning(
            "catalog.lock_unused",
            &EntityKey::Project,
            &["catalog"],
            None,
            format!(
                "`{}` in catalog.lock is not a shared catalog in fune.toml ([[catalog]] git = …). Run `fune catalog fetch` to remove it from the lock",
                unused.name
            ),
        );
    }
}

/// 状態 → (コード, 次にすることが分かる文)。読み込めていれば None。
fn problem(ctx: &Ctx, s: &SharedCatalogStatus) -> Option<(&'static str, String)> {
    let name = &s.name;
    Some(match s.state {
        SharedCatalogState::Ready => return None,
        SharedCatalogState::NotLocked => (
            "catalog.not_locked",
            format!(
                "The commit of shared catalog `{name}` is not in catalog.lock. Run `fune catalog fetch` to fetch it, then commit catalog.lock"
            ),
        ),
        SharedCatalogState::LockMismatch => {
            let locked = ctx
                .design
                .catalog_lock
                .as_ref()
                .and_then(|l| l.get(name))
                .map(|l| {
                    format!(
                        "{} @ {}",
                        l.git,
                        l.rev.as_deref().unwrap_or("default branch")
                    )
                })
                .unwrap_or_default();
            (
                "catalog.lock_mismatch",
                format!(
                    "The git and rev of shared catalog `{name}` differ from the record in catalog.lock ({locked}). Run `fune catalog update {name}` to update the lock"
                ),
            )
        }
        SharedCatalogState::NotFetched => {
            let commit = s.commit.as_deref().unwrap_or_default();
            let short = commit.get(..SHORT_COMMIT).unwrap_or(commit);
            (
                "catalog.not_fetched",
                format!(
                    "Commit `{short}` of shared catalog `{name}` has not been fetched (not in the .fune/ cache). Run `fune catalog fetch`"
                ),
            )
        }
    })
}
