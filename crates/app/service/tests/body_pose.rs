//! 機体座標の位置・姿勢（`PartRow.body`）と取付の根。
use fune_service::{Workspace, query};
use std::path::Path;

fn demo() -> Workspace {
    Workspace::open(Path::new(env!("CARGO_MANIFEST_DIR")).join("../../../fixtures/demo-6u"))
        .expect("demo")
}

/// 機体座標の位置・姿勢は、取付のある部品と取付の根（`BUS_FRAME`。姿勢を書かなければ原点・恒等）に付き、未配置の部品には付かない。
#[test]
fn body_pose_covers_mounted_parts_and_the_mount_root() {
    let ws = demo();
    let design = query::design_view(&ws).unwrap();
    let part = |id: &str| design.parts.iter().find(|p| p.id == id).unwrap();
    let root = part("BUS_FRAME");
    assert!(root.mount.is_none());
    let body = root.body.as_ref().expect("root body pose");
    assert!(body.root);
    assert_eq!(body.position, [0.0, 0.0, 0.0]);
    assert_eq!(body.orientation, [1.0, 0.0, 0.0, 0.0]);
    let panel = part("PANEL_PX").body.as_ref().expect("mounted body pose");
    assert!(!panel.root);
    assert!((panel.position[0] - 0.05).abs() < 1e-12);
    for p in design
        .parts
        .iter()
        .filter(|p| p.mount.is_none() && p.id != "BUS_FRAME")
    {
        assert!(p.body.is_none(), "{} is not placed", p.id);
    }
    // 根も機体座標で動かせる: 書く姿勢は取付先を持たない `Place`（位置はそのまま根の `[pose]`）
    let op = fune_service::edit::place_body_op(
        &ws,
        "BUS_FRAME",
        &fune_model::value::Vector3 {
            x: 1.0,
            y: 0.0,
            z: 0.0,
            unit: Some("mm".into()),
            frame: None,
        },
        None,
        None,
    )
    .unwrap();
    let fune_ops::Op::Place {
        on,
        point,
        position,
        ..
    } = op
    else {
        panic!("{op:?}")
    };
    assert_eq!((on, point), (None, None));
    assert_eq!(
        position.map(|p| (p.x, p.unit)),
        Some((1.0, Some("mm".into())))
    );
}
