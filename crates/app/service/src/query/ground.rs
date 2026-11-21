//! 地上局 → 表示（ID 順。`GET /design` の `stations`）。局の値は設定（`settings`）で見せる。

use crate::dto::GroundStationView;
use fune_model::design::{Design, Referent};
use fune_model::path::SettingPath;
use fune_store::layout::ground_file;

/// 地上局の一覧（ID 順）。
pub(crate) fn stations(design: &Design) -> Vec<GroundStationView> {
    design
        .conditions
        .ground
        .iter()
        .map(|(id, g)| GroundStationView {
            id: id.clone(),
            label: g.label.clone(),
            file: ground_file(id),
            group: SettingPath::conditions(&["ground", id]).to_string(),
            in_use: !design.references(Referent::Station(id)).is_empty(),
        })
        .collect()
}
