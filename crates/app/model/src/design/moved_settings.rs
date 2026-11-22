//! 基準（と、ミッション期間）が要求へ移った設定のキー。書かれていれば `fune check` が知らせ、`fune setting set` は拒む
//! （読むのは `design::criteria` の引き当てだけ。旧い設定の値は読まない）。

use crate::path::{SettingPath, SettingScope};

/// 基準が要求へ移った（もう設定には置かない）設定のキー。書かれていれば `fune check` が `setting.moved_to_requirement` で知らせ、
/// `fune setting set` は拒む（書き換えは要求で行う。消すのは `fune setting unset` ができる）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MovedSetting {
    pub scope: SettingScope,
    /// パスの区切り（`*` は 1 つの区切りに当たる）
    pub path: &'static [&'static str],
    /// 移した先の要求の書き方（診断の文に入れる）
    pub how: &'static str,
}

/// 基準が要求へ移った設定のキーの表。
pub const MOVED_SETTINGS: &[MovedSetting] = &[
    MovedSetting {
        scope: SettingScope::Budget,
        path: &["mass", "limit"],
        how: "an at_most requirement with verify.budget = \"mass\"",
    },
    MovedSetting {
        scope: SettingScope::Budget,
        path: &["power", "limit"],
        how: "an at_most requirement with verify.budget = \"power\"",
    },
    MovedSetting {
        scope: SettingScope::Budget,
        path: &["data", "limit"],
        how: "an at_most requirement with verify.budget = \"data\"",
    },
    MovedSetting {
        scope: SettingScope::Analysis,
        path: &["power", "dod_max"],
        how: "an at_most requirement with verify.analysis = \"power.energy_balance\" and verify.check = \"dod\"",
    },
    MovedSetting {
        scope: SettingScope::Analysis,
        path: &["comm", "min_margin"],
        how: "an at_least requirement with verify.analysis = \"comm.link_budget\" and verify.check = \"margin\" (every link and station) or \"margin:<link>@<station>\"",
    },
    MovedSetting {
        scope: SettingScope::Analysis,
        path: &["comm", "link", "*", "pfd_limit"],
        how: "an at_most requirement with verify.analysis = \"comm.link_budget\" and verify.check = \"pfd:<link>@<station>\"",
    },
    MovedSetting {
        scope: SettingScope::Analysis,
        path: &["aocs", "max_slew_time"],
        how: "an at_most requirement with verify.analysis = \"aocs.slew_time\" and verify.check = \"slew_time\"",
    },
    MovedSetting {
        scope: SettingScope::Conditions,
        path: &["orbit", "decay", "max_lifetime"],
        how: "an at_most requirement with verify.analysis = \"orbit.decay\" and verify.check = \"max_lifetime\"",
    },
    MovedSetting {
        scope: SettingScope::Conditions,
        path: &["launch", "min_frequency_axial"],
        how: "an at_least requirement with verify.analysis = \"structure.modal_estimate\" and verify.check = \"axial\"",
    },
    MovedSetting {
        scope: SettingScope::Conditions,
        path: &["launch", "min_frequency_lateral"],
        how: "an at_least requirement with verify.analysis = \"structure.modal_estimate\" and verify.check = \"lateral\"",
    },
    MovedSetting {
        scope: SettingScope::Conditions,
        path: &["orbit", "duration"],
        how: "an at_least Time requirement with input = \"mission_duration\" (the mission shall last at least ...)",
    },
];

impl MovedSetting {
    /// 設定のパス `path` が移ったキーなら、その行。
    pub fn of(path: &SettingPath) -> Option<&'static Self> {
        MOVED_SETTINGS.iter().find(|m| {
            m.scope == path.scope
                && m.path.len() == path.segments.len()
                && m.path
                    .iter()
                    .zip(&path.segments)
                    .all(|(p, s)| *p == "*" || p == s)
        })
    }

    /// 診断と拒否の文（英語）。
    pub fn message(&self, path: &SettingPath) -> String {
        format!(
            "`{path}` is a verdict criterion, so a requirement holds it now: use {}. Move the value into that requirement's constraint, then remove this key with `fune setting unset {path}`",
            self.how
        )
    }
}

impl MovedSetting {
    /// 書き込みを拒む文（英語）。基準は要求を編集して変える。
    pub fn refusal(&self, path: &SettingPath) -> String {
        format!(
            "`{path}` is a verdict criterion, so a requirement holds it: edit {} with `fune requirement set` instead",
            self.how
        )
    }
}
