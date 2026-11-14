//! 共有カタログの検査: `catalog.lock` が無い・lock と fune.toml が食い違う・lock の commit のキャッシュが無い・
//! lock に使っていない記録がある（docs/model.md §12）。状態の判定は `Design::shared_catalog_statuses` が持つ。

use super::Ctx;
use crate::design::{EntityKey, SharedCatalogState, SharedCatalogStatus};

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
                "catalog.lock の `{}` は fune.toml の共有カタログ（[[catalog]] git = …）にありません。`fune catalog fetch` を実行すると lock から外します",
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
                "共有カタログ `{name}` の commit が catalog.lock にありません。`fune catalog fetch` を実行して取得し、catalog.lock をコミットしてください"
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
                        l.rev.as_deref().unwrap_or("既定のブランチ")
                    )
                })
                .unwrap_or_default();
            (
                "catalog.lock_mismatch",
                format!(
                    "共有カタログ `{name}` の git・rev が catalog.lock の記録（{locked}）と違います。`fune catalog update {name}` を実行して lock を合わせてください"
                ),
            )
        }
        SharedCatalogState::NotFetched => {
            let commit = s.commit.as_deref().unwrap_or_default();
            let short = commit.get(..SHORT_COMMIT).unwrap_or(commit);
            (
                "catalog.not_fetched",
                format!(
                    "共有カタログ `{name}` の commit `{short}` がまだ取得されていません（.fune/ のキャッシュに無い）。`fune catalog fetch` を実行してください"
                ),
            )
        }
    })
}
