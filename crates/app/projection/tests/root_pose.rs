//! 根（取付の無い部品）の `[pose]` は機体座標での位置と向き。小さな設計をファイルで組み、機体座標の姿勢
//! （`pose::body_pose`）・取付点の座標系（`pose::mount_frame`）・逆変換（`pose::body_to_mount_local`）・質量特性・
//! 形態の上書き・外形の消費者への影響（`outline`）を手計算と比べる。姿勢を書かない根は今までどおり原点・恒等。

use fune_model::{Design, Resolved};
use fune_projection::{Context, MissingInputs, outline, pose, structure};
use fune_store::FileSet;
use fune_store::read::design_of;
use sat_astro::math::{Quat, Vec3};

/// 根 R: 機体座標の (−0.045, 0, 0) m。取付点 P は R の (0.1, 0, 0) m
fn root(extra_pose: &str) -> String {
    format!(
        "[pose]\nposition = {{ x = -0.045, y = 0.0, z = 0.0 }}\n{extra_pose}\n[mount_points.P]\nposition = {{ x = 0.1, y = 0.0, z = 0.0 }}\n\n[attr]\nmass = {{ value = 1.0, unit = \"kg\" }}\n"
    )
}
/// R の取付点 P に付けた子（姿勢は書かない）
const C: &str =
    "[mount]\non = \"R\"\npoint = \"P\"\n\n[attr]\nmass = { value = 1.0, unit = \"kg\" }\n";
/// 90°（z まわり）の向き
const TURN_Z: &str =
    "orientation = { w = 0.7071067811865476, x = 0.0, y = 0.0, z = 0.7071067811865476 }\n";

fn design(files: &[(&str, &str)]) -> Design {
    let mut set = FileSet::new();
    set.insert("fune.toml", "[project]\nname = \"t\"\n");
    for (path, text) in files {
        set.insert(*path, *text);
    }
    design_of(&set)
}

fn near(v: Vec3, x: f64, y: f64, z: f64) {
    let d = (v - Vec3::new(x, y, z)).norm();
    assert!(d < 1e-9, "{v:?} != ({x}, {y}, {z})");
}

fn body(resolved: &Resolved, part: &str) -> (Vec3, Quat) {
    pose::body_pose(resolved, part).expect("part")
}

/// R の位置は書いた (−0.045, 0, 0)、C は取付点 (0.1, 0, 0) の分だけ +x: (0.055, 0, 0)。
#[test]
fn a_root_pose_is_its_body_position_and_children_follow() {
    let d = design(&[("parts/R.toml", &root("")), ("parts/C.toml", C)]);
    let r = fune_model::resolve(&d);
    near(body(&r, "R").0, -0.045, 0.0, 0.0);
    near(body(&r, "C").0, 0.055, 0.0, 0.0);
    // 向きを書かない根は恒等
    near(body(&r, "R").1.rotate(Vec3::X), 1.0, 0.0, 0.0);
}

/// R を z まわり +90° にすると、取付点 (0.1, 0, 0) は R の座標の +x なので機体の +y 方向: C は (−0.045, 0.1, 0)。
/// C 自身の姿勢は R の姿勢を引き継ぎ、C の +x は機体の +y。
#[test]
fn a_rotated_root_carries_its_children() {
    let d = design(&[("parts/R.toml", &root(TURN_Z)), ("parts/C.toml", C)]);
    let r = fune_model::resolve(&d);
    near(body(&r, "R").0, -0.045, 0.0, 0.0);
    near(body(&r, "C").0, -0.045, 0.1, 0.0);
    near(body(&r, "C").1.rotate(Vec3::X), 0.0, 1.0, 0.0);
}

/// 根の取付点の座標系は機体座標: 機体座標の配置を根の `[pose]` に戻す逆変換は恒等の座標系を通る
/// （そのまま返る）。取付のある C は今までどおり、R の姿勢（位置・回転）を通して戻る。
#[test]
fn the_mount_frame_of_a_root_is_the_body_frame() {
    let d = design(&[("parts/R.toml", &root(TURN_Z)), ("parts/C.toml", C)]);
    let r = fune_model::resolve(&d);
    let (origin, q) = pose::mount_frame(&r, "R").unwrap();
    near(origin, 0.0, 0.0, 0.0);
    near(q.rotate(Vec3::X), 1.0, 0.0, 0.0);
    let want = Quat::from_wxyz([1.0, 0.0, 0.0, 0.0]);
    let (p, q) = pose::body_to_mount_local(&r, "R", Vec3::new(0.3, -0.2, 0.1), want).unwrap();
    near(p, 0.3, -0.2, 0.1);
    near(q.rotate(Vec3::X), 1.0, 0.0, 0.0);
    // C を機体の (0.0, 0.5, 0.0) に置く: 取付点は機体の (−0.045, 0.1, 0)・座標系は R と同じ（z まわり +90°）。
    // 差 (0.045, 0.4, 0) を −90° 回す（(x, y) → (y, −x)）と (0.4, −0.045, 0)
    let (p, _) = pose::body_to_mount_local(&r, "C", Vec3::new(0.0, 0.5, 0.0), want).unwrap();
    near(p, 0.4, -0.045, 0.0);
}

const POINT: &str = "[attr]\nmass = { value = 1.0, unit = \"kg\" }\n";

/// 根が 2 つ: z = ±0.1 m の 1 kg の点質量。重心は原点、Ixx = Iyy = 2 · 1 · 0.1² = 0.02 kg·m²、Izz = 0。
/// 姿勢を読まなければ両方が原点に重なり、慣性は 0 になる。
#[test]
fn several_roots_sit_at_their_own_places() {
    let at = |z: f64| format!("[pose]\nposition = {{ x = 0.0, y = 0.0, z = {z} }}\n\n{POINT}");
    let d = design(&[("parts/R1.toml", &at(0.1)), ("parts/R2.toml", &at(-0.1))]);
    let r = fune_model::resolve(&d);
    let ctx = Context::bare(&d, &r);
    let mut m = MissingInputs::new();
    let s = structure::summary(&ctx, &mut m).expect("summary");
    near(s.center_of_mass, 0.0, 0.0, 0.0);
    assert!((s.inertia.m[0][0] - 0.02).abs() < 1e-12, "{:?}", s.inertia);
    assert!((s.inertia.m[1][1] - 0.02).abs() < 1e-12);
    assert!(s.inertia.m[2][2].abs() < 1e-12);
}

/// 姿勢を書かない根は原点・恒等（今までと同じ）で、連鎖に根の項は積まれない。
#[test]
fn a_root_without_a_pose_stays_at_the_origin() {
    let d = design(&[("parts/R.toml", POINT), ("parts/C.toml", C)]);
    let r = fune_model::resolve(&d);
    assert!(r.parts["R"].root_pose.is_none());
    near(body(&r, "R").0, 0.0, 0.0, 0.0);
    assert!(r.pose_chain("R").is_empty());
    // C の連鎖は C の取付（取付点・自分の姿勢）の 2 項だけ（`R` に取付点 P が無いので恒等）
    assert_eq!(r.pose_chain("C").len(), 2);
}

/// 形態の `[pose.R]` は根の機体座標での姿勢を、その形態だけ置き換える。木全体が動き、基準は動かない。
#[test]
fn a_configuration_overrides_a_root_and_moves_the_whole_tree() {
    let moved = "[pose.R]\nposition = { x = 0.2, y = 0.0, z = 0.0 }\n";
    let d = design(&[
        ("parts/R.toml", &root("")),
        ("parts/C.toml", C),
        ("configurations/MOVED.toml", moved),
    ]);
    let r = fune_model::resolve(&d);
    let cfg = r.in_configuration(Some("MOVED"));
    near(body(&r, "R").0, -0.045, 0.0, 0.0);
    near(body(cfg, "R").0, 0.2, 0.0, 0.0);
    near(body(cfg, "C").0, 0.3, 0.0, 0.0);
    near(body(&r, "C").0, 0.055, 0.0, 0.0);
    // 根の姿勢を上書きしても、形態に上書きの無い根は基準のまま
    let d = design(&[
        ("parts/R.toml", &root("")),
        (
            "configurations/OTHER.toml",
            "[pose.C]\nposition = { x = 1.0, y = 0.0, z = 0.0 }\n",
        ),
        ("parts/C.toml", C),
    ]);
    let r = fune_model::resolve(&d);
    near(
        body(r.in_configuration(Some("OTHER")), "R").0,
        -0.045,
        0.0,
        0.0,
    );
}

fn outline_codes(d: &Design) -> Vec<String> {
    let r = fune_model::resolve(d);
    outline::check(d, &r).into_iter().map(|x| x.code).collect()
}

const SIZED: &str = "[attr]\nmass = { value = 1.0, unit = \"kg\" }\nsize = { x = 0.1, y = 0.2, z = 0.3, unit = \"m\" }\n";

fn sized_root(pose: &str) -> String {
    format!("{pose}\n{SIZED}")
}

/// 外形の根（取付を持たず size を持つ部品）の姿勢: 位置がずれているか 90° の倍数でない回転のときだけ `outline.root_pose` を警告する。
/// 姿勢を書かない・向きだけが 90° の倍数のときは出ない。
#[test]
fn the_outline_root_warns_only_when_consumers_would_misread_its_pose() {
    let code = |pose: &str| outline_codes(&design(&[("parts/R.toml", &sized_root(pose))]));
    assert!(code("").is_empty());
    let offset = code("[pose]\nposition = { x = 0.01, y = 0.0, z = 0.0 }\n");
    assert_eq!(offset, ["outline.root_pose"]);
    let turned45 = code(
        "[pose]\norientation = { w = 0.9238795325112867, x = 0.0, y = 0.0, z = 0.3826834323650898 }\n",
    );
    assert_eq!(turned45, ["outline.root_pose"]);
    // 90° の倍数の回転だけ（位置はそのまま原点）。寸法を入れ替えるので警告しない
    assert!(code(&format!("[pose]\n{TURN_Z}")).is_empty());
    // 外形でない根（size が無い）は見ない
    let d = design(&[("parts/R.toml", &root(""))]);
    assert!(outline_codes(&d).is_empty());
}

/// 外形の根が z まわり 90° なら、機体の軸に沿った寸法は x と y が入れ替わる（z はそのまま）。回転が無ければ size のまま。
/// 軸に沿わない回転は size のまま（警告が知らせる）。
#[test]
fn the_outline_extents_follow_a_quarter_turn() {
    let ext = |pose: &str| {
        let d = design(&[("parts/R.toml", &sized_root(pose))]);
        let r = fune_model::resolve(&d);
        outline::body_extents(&r, "R", [0.1, 0.2, 0.3])
    };
    assert_eq!(ext(""), [0.1, 0.2, 0.3]);
    let turned = ext(&format!("[pose]\n{TURN_Z}"));
    for (got, want) in turned.iter().zip([0.2, 0.1, 0.3]) {
        assert!((got - want).abs() < 1e-9, "{turned:?}");
    }
    let skew = ext(
        "[pose]\norientation = { w = 0.9238795325112867, x = 0.0, y = 0.0, z = 0.3826834323650898 }\n",
    );
    assert_eq!(skew, [0.1, 0.2, 0.3]);
}
