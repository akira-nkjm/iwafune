//! 根（取付の無い部品）の配置: 根の `[pose]` は機体座標での位置・姿勢で、機体座標の配置は根ではそのまま `[pose]` になる。
//! 手計算で確かめる小さな設計:
//!
//! - R は根（初めは姿勢なし）。R の取付点 P は R の座標で (0.1, 0, 0) m。C は R の P に取付（姿勢は書かない）。
//! - R を (−45, 0, 0) mm に置くと、C は機体の (0.055, 0, 0)。R を z まわり +90° にすると、P は機体の (−0.045, 0.1, 0)。
//! - その状態で C を機体の (0, 0.5, 0) に置くと、取付点に対する位置は Rz(−90°)·((0, 0.5, 0) − (−0.045, 0.1, 0)) = Rz(−90°)·(0.045, 0.4, 0)
//!   = (0.4, −0.045, 0)（Rz(−90°) は (x, y) → (y, −x)）。
//!
//! 姿勢の角度は 6 桁に丸めたクォータニオンでファイルに書くので、回転を含む値は 1e-5 の桁で一致する。

use fune_model::value::Vector3;
use fune_ops::Op;
use fune_service::dto::{ApplyRequest, ConfigurationText, PartsQuery, PlaceText};
use fune_service::{Workspace, edit, query};

fn project() -> (tempfile::TempDir, Workspace) {
    let dir = tempfile::tempdir().unwrap();
    let w = |path: &str, text: &str| {
        let p = dir.path().join(path);
        std::fs::create_dir_all(p.parent().unwrap()).unwrap();
        std::fs::write(p, text).unwrap();
    };
    w("fune.toml", "[project]\nname = \"t\"\n");
    w(
        "parts/R.toml",
        "kind = \"structure\"\n\n[mount_points.P]\nposition = { x = 100.0, y = 0.0, z = 0.0, unit = \"mm\" }\n",
    );
    w(
        "parts/C.toml",
        "kind = \"structure\"\n\n[mount]\non = \"R\"\npoint = \"P\"\n",
    );
    let ws = Workspace::open(dir.path()).unwrap();
    (dir, ws)
}

fn apply(ws: &Workspace, ops: Vec<Op>) {
    edit::apply(
        ws,
        &ApplyRequest {
            ops,
            base_revision: None,
            dry_run: false,
        },
    )
    .unwrap_or_else(|e| panic!("{e}"));
}

fn mm(x: f64, y: f64, z: f64) -> Vector3 {
    Vector3 {
        x,
        y,
        z,
        unit: Some("mm".into()),
        frame: None,
    }
}

fn place_text(part: &str, position: &str, rotation: Option<&str>, cfg: Option<&str>) -> Op {
    edit::place_op(&PlaceText {
        part: part.into(),
        on: None,
        point: None,
        position: Some(position.into()),
        orientation: None,
        rotation: rotation.map(Into::into),
        configuration: cfg.map(Into::into),
    })
    .unwrap()
}

fn body(ws: &Workspace, part: &str, cfg: Option<&str>) -> [f64; 3] {
    query::list_parts(
        ws,
        &PartsQuery {
            configuration: cfg.map(Into::into),
            ..PartsQuery::default()
        },
    )
    .unwrap()
    .into_iter()
    .find(|r| r.id == part)
    .and_then(|r| r.body)
    .unwrap_or_else(|| panic!("{part} has no body pose"))
    .position
}

fn close(a: [f64; 3], b: [f64; 3], eps: f64) {
    for (x, y) in a.iter().zip(b) {
        assert!((x - y).abs() <= eps, "{a:?} != {b:?}");
    }
}

fn local(op: &Op) -> [f64; 3] {
    let Op::Place {
        position: Some(p), ..
    } = op
    else {
        panic!("{op:?}")
    };
    assert_eq!(p.unit.as_deref(), Some("mm"));
    [p.x / 1000.0, p.y / 1000.0, p.z / 1000.0]
}

/// 根の置き方 2 通り（取付先なしの `Place` と機体座標）は、どちらも根の `[pose]` を書く。子は根の姿勢についてくる。
#[test]
fn a_root_is_placed_by_its_own_pose_and_in_body_coordinates() {
    let (dir, ws) = project();
    let file = |p: &str| std::fs::read_to_string(dir.path().join(p)).unwrap();
    // 姿勢なしの根は原点: C は取付点 (0.1, 0, 0)
    close(body(&ws, "R", None), [0.0; 3], 1e-12);
    close(body(&ws, "C", None), [0.1, 0.0, 0.0], 1e-12);
    // 取付先なしの配置（`fune part place R --root --position "-45 0 0 mm"`）が根の `[pose]` を書く
    apply(&ws, vec![place_text("R", "-45 0 0 mm", None, None)]);
    assert!(
        file("parts/R.toml").contains("[pose]"),
        "{}",
        file("parts/R.toml")
    );
    assert!(!file("parts/R.toml").contains("[mount]\n"));
    close(body(&ws, "R", None), [-0.045, 0.0, 0.0], 1e-12);
    close(body(&ws, "C", None), [0.055, 0.0, 0.0], 1e-12);
    // 機体座標の配置: 根では取付点の座標系が機体座標なので、要求した位置がそのまま `[pose]` に書かれる（単位は書かれた mm のまま）
    let op = edit::place_body_op(&ws, "R", &mm(20.0, -10.0, 5.0), None, None).unwrap();
    let Op::Place {
        on,
        point,
        position: Some(p),
        orientation,
        ..
    } = &op
    else {
        panic!("{op:?}")
    };
    assert_eq!((on, point, orientation), (&None, &None, &None));
    assert_eq!(
        (p.x, p.y, p.z, p.unit.as_deref()),
        (20.0, -10.0, 5.0, Some("mm"))
    );
    apply(&ws, vec![op]);
    close(body(&ws, "R", None), [0.02, -0.01, 0.005], 1e-12);
    close(body(&ws, "C", None), [0.12, -0.01, 0.005], 1e-12);
}

/// 根の向きは `[pose]` に書かれ、根に付いた子を機体座標で置くときの逆変換は根の姿勢を通る（手計算はファイルの先頭）。
#[test]
fn placing_a_child_converts_through_the_posed_root() {
    let (_dir, ws) = project();
    apply(
        &ws,
        vec![place_text("R", "-45 0 0 mm", Some("0 0 90 deg"), None)],
    );
    close(body(&ws, "R", None), [-0.045, 0.0, 0.0], 1e-12);
    close(body(&ws, "C", None), [-0.045, 0.1, 0.0], 1e-5);
    // 根を向きごと機体座標で置き直しても、位置は求めた位置そのもの
    let q = edit::rotation_from_text("0 0 90 deg").unwrap();
    let op = edit::place_body_op(&ws, "R", &mm(-45.0, 0.0, 0.0), Some(&q), None).unwrap();
    let Op::Place {
        orientation: Some(written),
        ..
    } = &op
    else {
        panic!("{op:?}")
    };
    assert_eq!((written.w, written.z), (q.w, q.z));
    // C を機体の (0, 0.5, 0) に置く → 取付点に対する位置 (0.4, −0.045, 0)
    let op = edit::place_body_op(&ws, "C", &mm(0.0, 500.0, 0.0), None, None).unwrap();
    close(local(&op), [0.4, -0.045, 0.0], 1e-5);
    apply(&ws, vec![op]);
    close(body(&ws, "C", None), [0.0, 0.5, 0.0], 1e-5);
}

/// 形態は根の姿勢を上書きできる（取付先を書けば付け替えなので拒む）。形態の中の機体座標の配置は形態の `[pose.<根>]` に書き、
/// 基準の部品ファイルは変わらない。
#[test]
fn a_configuration_can_override_a_root() {
    let (dir, ws) = project();
    apply(
        &ws,
        vec![edit::configuration_op(
            &ConfigurationText {
                id: "SHIFTED".into(),
                label: None,
                label_en: None,
                description: None,
            },
            true,
        )],
    );
    let r_file = std::fs::read_to_string(dir.path().join("parts/R.toml")).unwrap();
    apply(
        &ws,
        vec![place_text("R", "200 0 0 mm", None, Some("SHIFTED"))],
    );
    close(body(&ws, "R", Some("SHIFTED")), [0.2, 0.0, 0.0], 1e-12);
    close(body(&ws, "C", Some("SHIFTED")), [0.3, 0.0, 0.0], 1e-12);
    close(body(&ws, "C", None), [0.1, 0.0, 0.0], 1e-12);
    assert_eq!(
        std::fs::read_to_string(dir.path().join("parts/R.toml")).unwrap(),
        r_file
    );
    assert!(
        std::fs::read_to_string(dir.path().join("configurations/SHIFTED.toml"))
            .unwrap()
            .contains("[pose.R]")
    );
    // 形態の中の機体座標の配置
    let op = edit::place_body_op(&ws, "R", &mm(0.0, 300.0, 0.0), None, Some("SHIFTED")).unwrap();
    apply(&ws, vec![op]);
    close(body(&ws, "R", Some("SHIFTED")), [0.0, 0.3, 0.0], 1e-12);
    // 取付先を書く配置は付け替えなので、形態の中では拒む
    let with_mount = Op::Place {
        part: "R".into(),
        on: Some("C".into()),
        point: None,
        position: Some(mm(1.0, 0.0, 0.0)),
        orientation: None,
        configuration: Some("SHIFTED".into()),
    };
    let err = edit::apply(
        &ws,
        &ApplyRequest {
            ops: vec![with_mount],
            base_revision: None,
            dry_run: false,
        },
    )
    .unwrap_err();
    assert!(err.to_string().contains("not its mount"), "{err}");
}

/// 外形の根に位置のずれがあると、`fune check` の診断（service の `Snapshot`）に `outline.root_pose` の警告が付く。
#[test]
fn the_check_warns_about_an_outline_root_with_an_offset() {
    let (dir, ws) = project();
    std::fs::write(
        dir.path().join("parts/R.toml"),
        "kind = \"structure\"\n\n[pose]\nposition = { x = 10.0, y = 0.0, z = 0.0, unit = \"mm\" }\n\n[attr]\nsize = { x = 100.0, y = 100.0, z = 100.0, unit = \"mm\" }\n",
    )
    .unwrap();
    let codes: Vec<_> = ws
        .snapshot()
        .unwrap()
        .diagnostics
        .iter()
        .map(|d| d.diagnostic.code.clone())
        .collect();
    assert!(codes.iter().any(|c| c == "outline.root_pose"), "{codes:?}");
}
