//! `fune configuration add|set|unset-pose` に打った引数 → `CreateConfiguration`・`SetConfiguration`・`UnsetConfigurationPose`
//! （`POST /ops/from_command` の `configuration_add`・`configuration_set`）。形態の中への配置は `fune part place --configuration`
//! と同じ `edit::place_op`。**形態の旗の読み方はここだけ**（逆の向き = 編集の欄の初期値は `query::configurations::configuration_text`）。

use super::text::given;
use crate::dto::ConfigurationText;
use fune_ops::Op;

/// 打った引数 → `CreateConfiguration`（`create`）か `SetConfiguration`（部品ごとの姿勢は書かない。`SetConfiguration` は今のものを残す）。
pub fn configuration_op(t: &ConfigurationText, create: bool) -> Op {
    let id = t.id.trim().to_owned();
    let label = given(&t.label).map(str::to_owned);
    let description = given(&t.description).map(str::to_owned);
    if create {
        Op::CreateConfiguration {
            id,
            label,
            description,
        }
    } else {
        Op::SetConfiguration {
            id,
            label,
            description,
        }
    }
}

/// `fune configuration unset-pose <形態> <部品>` → `UnsetConfigurationPose`。
pub fn unset_pose_op(configuration: &str, part: &str) -> Op {
    Op::UnsetConfigurationPose {
        configuration: configuration.trim().to_owned(),
        part: part.trim().to_owned(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::query::configurations::configuration_text;
    use fune_model::design::Configuration;

    /// 空の旗は書かず、編集の欄の初期値で読み直すと同じ見出しの欄になる。
    #[test]
    fn typed_text_becomes_a_configuration_and_back() {
        let t = ConfigurationText {
            id: " STOWED ".into(),
            label: Some("Stowed".into()),
            description: Some(String::new()),
        };
        let Op::CreateConfiguration {
            id,
            label,
            description,
        } = configuration_op(&t, true)
        else {
            panic!("not CreateConfiguration");
        };
        assert_eq!(
            (id.as_str(), label.as_deref(), description),
            ("STOWED", Some("Stowed"), None)
        );
        let c = Configuration {
            id: "STOWED".into(),
            label: Some("Stowed".into()),
            ..Configuration::default()
        };
        assert_eq!(
            configuration_op(&configuration_text(&c), false),
            Op::SetConfiguration {
                id: "STOWED".into(),
                label: Some("Stowed".into()),
                description: None,
            }
        );
    }
}
