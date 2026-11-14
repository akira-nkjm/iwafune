//! 読んだ対象 → どのカードのどの行か。部品の属性は部品のカード（まとめるのは `group`）、設定は設定ファイルのカード（`query::settings::split_setting`）、
//! モードはモードのカード、ケースのフィールドはケースのカード。

use fune_model::Target;
use fune_model::path::{ParamPath, SettingPath};

/// カードの鍵（並びは部品 → 設定 → モード → ケース）。
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub(super) enum CardKey {
    /// 部品（1 つか、同じ役割で同じ読まれ方の部品をまとめたもの。ID 順）
    Part(Vec<String>),
    /// 設定ファイルのまとまり（`conditions.orbit`）とファイル（`conditions/orbit.toml`）
    Setting(SettingPath, String),
    Modes,
    Case(String),
}

impl CardKey {
    /// カードの ID（図の中で一意。Web は分解しない）。
    pub(super) fn node_id(&self) -> String {
        match self {
            CardKey::Part(ids) if ids.len() == 1 => format!("part:{}", ids[0]),
            CardKey::Part(ids) => format!("parts:{}", ids.join("+")),
            CardKey::Setting(p, _) => format!("setting:{p}"),
            CardKey::Modes => "modes".to_owned(),
            CardKey::Case(id) => format!("case:{id}"),
        }
    }
}

/// 読んだ対象 → (カード, カードの中の行の ID)。カードにしない対象（ほかの解析など）は None。
pub(super) fn locate(t: &Target) -> Option<(CardKey, String)> {
    match t {
        Target::Param(ParamPath::Attr { part, attr, .. }) => {
            Some((CardKey::Part(vec![part.clone()]), attr.clone()))
        }
        Target::Param(ParamPath::Field { part, field }) => {
            Some((CardKey::Part(vec![part.clone()]), field.clone()))
        }
        Target::Param(ParamPath::Setting(path)) => {
            let (group, keys, file) = crate::query::settings::split_setting(path);
            let file = file.unwrap_or_else(|| group.to_string());
            Some((CardKey::Setting(group, file), keys.join(".")))
        }
        Target::Mode { id, .. } => Some((CardKey::Modes, id.clone())),
        Target::Case { id, field } => Some((CardKey::Case(id.clone()), field.clone()?)),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn settings_group_by_file_and_parts_by_id() {
        let dod = Target::from(SettingPath::analysis(&["power", "dod_max"]));
        assert_eq!(
            locate(&dod),
            Some((
                CardKey::Setting(
                    SettingPath::analysis(&["power"]),
                    "analysis/power.toml".to_owned()
                ),
                "dod_max".to_owned()
            ))
        );
        let law = Target::from(SettingPath::analysis(&["aocs", "law", "NOMINAL"]));
        assert_eq!(locate(&law).map(|(_, r)| r), Some("law.NOMINAL".to_owned()));
        let area = Target::attr("SP_PX", "area");
        assert_eq!(
            locate(&area),
            Some((CardKey::Part(vec!["SP_PX".to_owned()]), "area".to_owned()))
        );
        assert_eq!(
            locate(&Target::Analysis {
                id: "power.mode_load".to_owned()
            }),
            None
        );
    }
}
