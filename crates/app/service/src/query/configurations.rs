//! 形態 → 表示（ID 順。`Design::configurations` は ID の順）。姿勢の中身は部品の表示（`GET /parts?configuration=`）で読む。

use super::views::file_of;
use crate::dto::{ConfigurationText, ConfigurationView};
use fune_model::design::{Configuration, Design, EntityKey};

/// 形態の一覧（ID 順）。
pub(crate) fn configurations(design: &Design) -> Vec<ConfigurationView> {
    design
        .configurations
        .values()
        .map(|c| view(design, c))
        .collect()
}

fn view(design: &Design, c: &Configuration) -> ConfigurationView {
    let id = c.id.as_str();
    ConfigurationView {
        id: id.to_owned(),
        label: c.label.clone().unwrap_or_else(|| id.to_owned()),
        description: c.description.clone(),
        overridden: c.pose.keys().map(ToString::to_string).collect(),
        text: configuration_text(c),
        file: file_of(design, EntityKey::Configuration(id.to_owned())),
    }
}

/// 今の形態 → 打った引数（`edit::configuration_op` で読み直すと同じ見出しの欄になる。GUI の編集の欄の初期値）。
pub(crate) fn configuration_text(c: &Configuration) -> ConfigurationText {
    ConfigurationText {
        id: c.id.to_string(),
        label: c.label.clone(),
        description: c.description.clone(),
    }
}
