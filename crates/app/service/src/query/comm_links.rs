//! 通信の回線 → 表示（名前順。`GET /design` の `links`）。

use crate::dto::LinkView;
use fune_model::design::{AnalysisSystem, Design};
use fune_model::path::SettingPath;

/// 通信の回線の一覧（名前順。通信の設定が無ければ空）。
pub(crate) fn links(design: &Design) -> Vec<LinkView> {
    design
        .analysis
        .comm
        .iter()
        .flat_map(|c| c.links.iter())
        .map(|(id, l)| LinkView {
            id: id.clone(),
            direction: l.direction,
            part: l.part.clone(),
            band: l.band.clone(),
            file: fune_store::layout::analysis_file(AnalysisSystem::Comm),
            group: SettingPath::analysis(&["comm", "link", id]).to_string(),
        })
        .collect()
}
