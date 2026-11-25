//! 通信の回線の一覧（`GET /design` の `links`）。デモの設計の回線と一致する（名前順）。
use fune_model::design::LinkDirection;
use fune_service::Workspace;
use std::path::Path;

#[test]
fn the_demo_lists_its_links_in_name_order() {
    let ws =
        Workspace::open(Path::new(env!("CARGO_MANIFEST_DIR")).join("../../../fixtures/demo-6u"))
            .unwrap();
    let view = fune_service::query::design_view(&ws).unwrap();
    let ids: Vec<&str> = view.links.iter().map(|l| l.id.as_str()).collect();
    assert_eq!(ids, ["s_down", "uhf_down", "uhf_up"]);
    let s = &view.links[0];
    assert_eq!(s.direction, LinkDirection::Down);
    assert_eq!((s.part.as_str(), s.band.as_str()), ("STX", "s_band"));
    assert_eq!(s.file, "analysis/comm.toml");
    assert_eq!(s.group, "analysis.comm.link.s_down");
    assert_eq!(view.links[2].direction, LinkDirection::Up);
}
