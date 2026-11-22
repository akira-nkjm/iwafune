//! 基準が要求へ移った設定のキー（`design::criteria::MOVED_SETTINGS`）が、まだ書かれていることの知らせ。
//! 読まれない値が黙って残らないよう error にする（移し先は診断の文が言う）。

use super::Ctx;
use crate::design::{EntityKey, MovedSetting, SettingEntry};
use crate::path::{SettingPath, SettingScope};

pub(super) fn check_moved_settings(ctx: &mut Ctx, entries: &[SettingEntry]) {
    for entry in entries.iter().filter(|e| e.value.is_some()) {
        let Some(moved) = MovedSetting::of(&entry.path) else {
            continue;
        };
        // 位置の索引は、バジェットは配分のファイル、conditions と analysis は先頭 2 つの区切り（`analysis.power` など）のファイル
        let segments = &entry.path.segments;
        let (entity, keys) = if entry.path.scope == SettingScope::Budget {
            (EntityKey::Allocation, segments.as_slice())
        } else {
            let prefix = SettingPath {
                scope: entry.path.scope,
                segments: segments[..1].to_vec(),
            };
            (EntityKey::Setting(prefix), &segments[1..])
        };
        let keys: Vec<&str> = keys.iter().map(String::as_str).collect();
        let message = moved.message(&entry.path);
        ctx.error(
            "setting.moved_to_requirement",
            &entity,
            &keys,
            Some(entry.path.clone().into()),
            message,
        );
    }
}
