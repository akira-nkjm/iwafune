//! 形態の中の機体座標での配置（`place_body_op` の `configuration`）と、形態ごとの部品の表示（`PartRow.mount`・`PartRow.body`）。
//! 手計算で確かめる小さな設計:
//!
//! - R は取付の根（原点・恒等）。A は R に取付（姿勢は書かない）。A の取付点 P は A の座標で (0, 0.2, 0) m、Z まわり +90°。
//! - B は A の P に取付。基準の姿勢は位置 (0, 0, 0) m（取付点に重なる）。
//!
//! Rz(θ) は (x, y) → (x cosθ − y sinθ, x sinθ + y cosθ)。+90° は (x, y) → (−y, x)、−90° は (y, −x)、±180° は (−x, −y)。
//! 四元数は 1/√2 を倍精度で書く（0.707107 に丸めると読み戻しが 1e-7 ずれるので、手計算と厳密に比べるため）。

use fune_model::value::{Quaternion, Vector3};
use fune_service::dto::{ApplyRequest, PartRow, PartViewRequest, PartsQuery};
use fune_service::{Workspace, edit, query};

const H: f64 = std::f64::consts::FRAC_1_SQRT_2;

fn project() -> (tempfile::TempDir, Workspace) {
    let dir = tempfile::tempdir().unwrap();
    let w = |path: &str, text: &str| {
        let p = dir.path().join(path);
        std::fs::create_dir_all(p.parent().unwrap()).unwrap();
        std::fs::write(p, text).unwrap();
    };
    w("fune.toml", "[project]\nname = \"t\"\n");
    w("parts/R.toml", "kind = \"structure\"\n");
    w(
        "parts/A.toml",
        &format!(
            "kind = \"structure\"\n\n[mount]\non = \"R\"\n\n[mount_points.P]\nposition    = {{ x = 0.0, y = 0.2, z = 0.0, unit = \"m\" }}\norientation = {{ w = {H}, x = 0.0, y = 0.0, z = {H} }}\n"
        ),
    );
    w(
        "parts/B.toml",
        "kind = \"structure\"\n\n[mount]\non = \"A\"\npoint = \"P\"\n\n[pose]\nposition = { x = 0.0, y = 0.0, z = 0.0, unit = \"m\" }\n",
    );
    let ws = Workspace::open(dir.path()).unwrap();
    (dir, ws)
}

fn m(x: f64, y: f64, z: f64) -> Vector3 {
    Vector3 {
        x,
        y,
        z,
        unit: Some("m".into()),
        frame: None,
    }
}

fn apply(ws: &Workspace, ops: Vec<fune_ops::Op>) {
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

fn rows(ws: &Workspace, configuration: Option<&str>) -> Vec<PartRow> {
    query::list_parts(
        ws,
        &PartsQuery {
            configuration: configuration.map(Into::into),
            ..PartsQuery::default()
        },
    )
    .unwrap()
}

fn body(ws: &Workspace, part: &str, configuration: Option<&str>) -> [f64; 3] {
    rows(ws, configuration)
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

fn place_op(ws: &Workspace, orientation: Option<&Quaternion>, cfg: Option<&str>) -> fune_ops::Op {
    edit::place_body_op(ws, "B", &m(0.3, 0.7, 0.1), orientation, cfg).unwrap()
}

fn written(op: &fune_ops::Op) -> ([f64; 3], Option<[f64; 4]>) {
    let fune_ops::Op::Place {
        position,
        orientation,
        ..
    } = op
    else {
        panic!("{op:?}")
    };
    let p = position.as_ref().unwrap();
    (
        [p.x, p.y, p.z],
        orientation.as_ref().map(|q| [q.w, q.x, q.y, q.z]),
    )
}

/// 手計算 1（A は基準のまま）:
/// B の取付点 P の機体座標は、A が R に重なる（恒等）ので、原点 (0, 0.2, 0)、姿勢 Rz(+90°)。
/// 機体座標の位置 (0.3, 0.7, 0.1) の取付点に対する位置は Rz(−90°)·((0.3, 0.7, 0.1) − (0, 0.2, 0)) = Rz(−90°)·(0.3, 0.5, 0.1)
///   = (0.5, −0.3, 0.1)。姿勢を指定しなければ、書かれた姿勢（無し）のまま。
#[test]
fn body_placement_in_a_configuration_matches_the_hand_calculation() {
    let (dir, ws) = project();
    apply(
        &ws,
        vec![edit::configuration_op(
            &fune_service::dto::ConfigurationText {
                id: "STOWED".into(),
                label: Some("Stowed".into()),
                description: None,
            },
            true,
        )],
    );
    let b_file = std::fs::read_to_string(dir.path().join("parts/B.toml")).unwrap();
    let op = place_op(&ws, None, Some("STOWED"));
    let (p, q) = written(&op);
    assert_eq!((p, q), ([0.5, -0.3, 0.1], None));
    apply(&ws, vec![op]);
    // 形態の表示は要求した機体座標に読み戻る（取付点の姿勢が厳密なので丸めの誤差は無い）
    close(body(&ws, "B", Some("STOWED")), [0.3, 0.7, 0.1], 1e-12);
    // 基準は動かない: B のファイルはそのまま、基準の機体座標は取付点の位置 (0, 0.2, 0)
    assert_eq!(
        std::fs::read_to_string(dir.path().join("parts/B.toml")).unwrap(),
        b_file
    );
    close(body(&ws, "B", None), [0.0, 0.2, 0.0], 1e-12);
    // 形態が上書きしている部品の印
    let overridden = |cfg: Option<&str>, part: &str| {
        rows(&ws, cfg)
            .into_iter()
            .find(|r| r.id == part)
            .and_then(|r| r.mount)
            .map(|m| m.overridden)
    };
    assert_eq!(overridden(Some("STOWED"), "B"), Some(true));
    assert_eq!(overridden(Some("STOWED"), "A"), Some(false));
    assert_eq!(overridden(None, "B"), Some(false));
    // 形態の取付点に対する位置の表示は上書きの値
    let b = rows(&ws, Some("STOWED"))
        .into_iter()
        .find(|r| r.id == "B")
        .unwrap();
    let mount = b.mount.unwrap();
    assert_eq!(mount.position, [0.5, -0.3, 0.1]);
    assert_eq!(mount.authored_position.unwrap().y, -0.3);
    // 1 部品の表示も同じ
    let view = query::part_view(
        &ws,
        "B",
        &PartViewRequest {
            configuration: Some("STOWED".into()),
            ..PartViewRequest::default()
        },
    )
    .unwrap();
    close(view.part.body.unwrap().position, [0.3, 0.7, 0.1], 1e-12);
}

/// 手計算 2（A も形態で上書き）: A の姿勢を位置 (1, 0, 0) m・Rz(+90°) にする。
/// A の機体座標は (1, 0, 0)・Rz(+90°)。P の機体座標は (1, 0, 0) + Rz(+90°)·(0, 0.2, 0) = (1, 0, 0) + (−0.2, 0, 0) = (0.8, 0, 0)、
/// 姿勢は Rz(+90°)·Rz(+90°) = Rz(180°)。
/// 同じ機体座標の位置 (0.3, 0.7, 0.1) の取付点に対する位置は Rz(−180°)·((0.3, 0.7, 0.1) − (0.8, 0, 0)) = Rz(−180°)·(−0.5, 0.7, 0.1)
///   = (0.5, −0.7, 0.1)。1 つ目（0.5, −0.3, 0.1）とは違い、読み戻しは要求した位置。
#[test]
fn an_overridden_parent_changes_the_mount_local_pose() {
    let (_dir, ws) = project();
    apply(
        &ws,
        vec![fune_ops::Op::CreateConfiguration {
            id: "STOWED".into(),
            label: None,
            description: None,
        }],
    );
    let first = place_op(&ws, None, Some("STOWED"));
    assert_eq!(written(&first).0, [0.5, -0.3, 0.1]);
    apply(&ws, vec![first]);
    // A の上書き（A の取付は R のまま）
    apply(
        &ws,
        vec![fune_ops::Op::Place {
            part: "A".into(),
            on: None,
            point: None,
            position: Some(m(1.0, 0.0, 0.0)),
            orientation: Some(Quaternion {
                w: H,
                x: 0.0,
                y: 0.0,
                z: H,
            }),
            configuration: Some("STOWED".into()),
        }],
    );
    // B の上書き（取付点に対する (0.5, −0.3, 0.1)）は A に付いて動く: (0.8, 0, 0) + Rz(180°)·(0.5, −0.3, 0.1) = (0.3, 0.3, 0.1)
    close(body(&ws, "B", Some("STOWED")), [0.3, 0.3, 0.1], 1e-12);
    let second = place_op(&ws, None, Some("STOWED"));
    assert_eq!(written(&second).0, [0.5, -0.7, 0.1]);
    apply(&ws, vec![second]);
    close(body(&ws, "B", Some("STOWED")), [0.3, 0.7, 0.1], 1e-12);
    // 基準は A の上書きの影響を受けない
    close(body(&ws, "A", None), [0.0, 0.0, 0.0], 1e-12);
    close(body(&ws, "A", Some("STOWED")), [1.0, 0.0, 0.0], 1e-12);
}

/// 姿勢も指定する: 機体座標の姿勢が恒等なら、取付点に対する姿勢は取付点の姿勢の逆 Rz(−90°) = (0.707107, 0, 0, −0.707107)
/// （6 桁に丸めて書く）。読み戻しの姿勢の誤差は、その丸め（1e-6）以内。
#[test]
#[allow(clippy::approx_constant)] // ファイルに 6 桁で書く値（0.707107）を確かめる
fn a_body_orientation_is_written_as_the_inverse_of_the_mount_frame() {
    let (_dir, ws) = project();
    apply(
        &ws,
        vec![fune_ops::Op::CreateConfiguration {
            id: "STOWED".into(),
            label: None,
            description: None,
        }],
    );
    let op = place_op(&ws, Some(&Quaternion::IDENTITY), Some("STOWED"));
    let (p, q) = written(&op);
    assert_eq!(p, [0.5, -0.3, 0.1]);
    assert_eq!(q, Some([0.707107, 0.0, 0.0, -0.707107]));
    apply(&ws, vec![op]);
    close(body(&ws, "B", Some("STOWED")), [0.3, 0.7, 0.1], 1e-6);
    let b = rows(&ws, Some("STOWED"))
        .into_iter()
        .find(|r| r.id == "B")
        .unwrap();
    let o = b.body.unwrap().orientation;
    close([o[1], o[2], o[3]], [0.0, 0.0, 0.0], 1e-6);
    assert!((o[0] - 1.0).abs() <= 1e-6, "{o:?}");
}

/// 上書きを消すと基準に戻り、無い形態・上書きの無い部品は拒む。
#[test]
fn reset_returns_to_the_base_and_unknown_configurations_are_refused() {
    let (dir, ws) = project();
    apply(
        &ws,
        vec![fune_ops::Op::CreateConfiguration {
            id: "STOWED".into(),
            label: Some("Stowed".into()),
            description: None,
        }],
    );
    apply(&ws, vec![place_op(&ws, None, Some("STOWED"))]);
    apply(&ws, vec![edit::unset_pose_op("STOWED", "B")]);
    close(body(&ws, "B", Some("STOWED")), [0.0, 0.2, 0.0], 1e-12);
    assert_eq!(
        std::fs::read_to_string(dir.path().join("configurations/STOWED.toml")).unwrap(),
        "label = \"Stowed\"\n"
    );
    let again = edit::apply(
        &ws,
        &ApplyRequest {
            ops: vec![edit::unset_pose_op("STOWED", "B")],
            base_revision: None,
            dry_run: false,
        },
    );
    assert!(again.is_err());
    // 無い形態は、読み取りも機体座標の配置も拒む（無い ID を基準として読まない）
    let unknown = PartsQuery {
        configuration: Some("NOPE".into()),
        ..PartsQuery::default()
    };
    assert!(query::list_parts(&ws, &unknown).is_err());
    assert!(edit::place_body_op(&ws, "B", &m(0.3, 0.7, 0.1), None, Some("NOPE")).is_err());
    // 根は機体座標の配置の対象にならない（形態の中でも）
    assert!(
        edit::place_body_op(&ws, "R", &m(0.3, 0.7, 0.1), None, Some("STOWED"))
            .unwrap_err()
            .to_string()
            .contains("mount root")
    );
}
