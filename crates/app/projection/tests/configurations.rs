//! 形態（`configurations/<ID>.toml`）: 解決は形態ごとの解決済みの設計を作り、取付の姿勢だけが上書きで変わる。
//! 小さな設計をファイルで組み、機体座標の姿勢（`pose::body_pose`）と質量特性（`structure::summary`）を手計算と比べる。
//! 形態の無い設計は基準だけで、数値は変わらない。

use fune_model::{Design, Resolved};
use fune_projection::{Context, MissingInputs, pose, structure};
use fune_store::FileSet;
use fune_store::read::design_of;
use sat_astro::math::Vec3;

const A: &str = "[attr]\nmass = { value = 4.0, unit = \"kg\" }\n";
/// 基準: A の (0, 0.2, 0) m に 1 kg
const B: &str = "[mount]\non = \"A\"\n\n[pose]\nposition = { x = 0.0, y = 0.2, z = 0.0 }\norientation = { w = 0.707107, x = 0.0, y = 0.0, z = 0.707107 }\n\n[attr]\nmass = { value = 1.0, unit = \"kg\" }\n";
/// B の +x に (0.1, 0, 0) m
const C: &str = "[mount]\non = \"B\"\n\n[pose]\nposition = { x = 0.1, y = 0.0, z = 0.0 }\n\n[attr]\nmass = { value = 0.0, unit = \"kg\" }\n";

fn design(extra: &[(&str, &str)]) -> Design {
    let mut set = FileSet::new();
    set.insert("fune.toml", "[project]\nname = \"t\"\n");
    set.insert("parts/A.toml", A);
    set.insert("parts/B.toml", B);
    for (path, text) in extra {
        set.insert(*path, *text);
    }
    design_of(&set)
}

fn near(v: Vec3, x: f64, y: f64, z: f64) {
    let d = (v - Vec3::new(x, y, z)).norm();
    assert!(d < 1e-9, "{v:?} != ({x}, {y}, {z})");
}

fn body(resolved: &Resolved, part: &str) -> (Vec3, sat_astro::math::Quat) {
    pose::body_pose(resolved, part).expect("part")
}

/// B を (0.05, 0, 0) m に置き、向きは書かない（基準の向きを引き継がない）形態
const MOVED: &str = "label = \"Moved\"\n\n[pose.B]\nposition = { x = 0.05, y = 0.0, z = 0.0 }\n";

#[test]
fn a_design_without_configurations_resolves_to_the_base_only() {
    let d = design(&[]);
    let r = fune_model::resolve(&d);
    assert!(r.configurations.is_empty());
    assert!(std::ptr::eq(r.in_configuration(None), &r));
    assert!(std::ptr::eq(r.in_configuration(Some("ANY")), &r));
}

#[test]
fn unknown_and_absent_ids_give_the_base() {
    let d = design(&[("configurations/MOVED.toml", MOVED)]);
    let r = fune_model::resolve(&d);
    assert!(std::ptr::eq(r.in_configuration(None), &r));
    assert!(std::ptr::eq(r.in_configuration(Some("NOPE")), &r));
    assert!(!std::ptr::eq(r.in_configuration(Some("MOVED")), &r));
    // 基準そのものは形態を書いても変わらない
    near(body(&r, "B").0, 0.0, 0.2, 0.0);
}

/// 基準: 重心 y = 0.2 / 5 = 0.04 m、点質量の慣性 μd² = (4·1/5)(0.2)² = 0.032 kg·m²（重心まわり。最大の主慣性モーメント）。
/// 上書き: B が (0.05, 0, 0) m → 重心 x = 0.05 / 5 = 0.01 m、y = 0、μd² = 0.8 · 0.05² = 0.002 kg·m²。
#[test]
fn mass_properties_follow_the_overridden_pose() {
    let d = design(&[("configurations/MOVED.toml", MOVED)]);
    let r = fune_model::resolve(&d);
    let summary = |cfg: Option<&str>| {
        let ctx = Context::bare(&d, r.in_configuration(cfg));
        let mut m = MissingInputs::new();
        structure::summary(&ctx, &mut m).expect("summary")
    };
    let base = summary(None);
    near(base.center_of_mass, 0.0, 0.04, 0.0);
    assert!((base.principal_moments.z - 0.032).abs() < 1e-12);
    assert!(base.principal_moments.x.abs() < 1e-12);
    let moved = summary(Some("MOVED"));
    near(moved.center_of_mass, 0.01, 0.0, 0.0);
    assert!((moved.principal_moments.z - 0.002).abs() < 1e-12);
    assert!(moved.principal_moments.x.abs() < 1e-12);
    // 総質量は変わらない
    assert_eq!(base.total_mass.value(), moved.total_mass.value());
}

#[test]
fn a_child_follows_its_overridden_mount() {
    let d = design(&[("parts/C.toml", C), ("configurations/MOVED.toml", MOVED)]);
    let r = fune_model::resolve(&d);
    // 基準: B は y = 0.2 に +90° (z まわり) で付き、C は B の +x に 0.1 → 機体の +y に 0.1 だけ進んだ y = 0.3
    near(body(&r, "C").0, 0.0, 0.3, 0.0);
    // 形態: B は (0.05, 0, 0) で向きは恒等 → C は x = 0.15
    near(
        body(r.in_configuration(Some("MOVED")), "C").0,
        0.15,
        0.0,
        0.0,
    );
}

#[test]
fn an_orientation_override_turns_plus_x_into_plus_y() {
    let file = "[pose.B]\norientation = { w = 0.707107, x = 0.0, y = 0.0, z = 0.707107 }\n";
    // 基準の B は向きを書かない版に差し替えて、形態だけが 90° を持つようにする
    let plain_b = B.replace(
        "orientation = { w = 0.707107, x = 0.0, y = 0.0, z = 0.707107 }\n",
        "",
    );
    let d = design(&[
        ("parts/B.toml", &plain_b),
        ("configurations/TURNED.toml", file),
    ]);
    let r = fune_model::resolve(&d);
    let x = Vec3::new(1.0, 0.0, 0.0);
    let base = body(&r, "B").1.rotate(x);
    near(base, 1.0, 0.0, 0.0);
    let turned = body(r.in_configuration(Some("TURNED")), "B").1.rotate(x);
    // クォータニオンは 6 桁の丸め（w = z = 0.707107）なので 1e-6 の桁で一致する
    assert!(
        (turned - Vec3::new(0.0, 1.0, 0.0)).norm() < 1e-5,
        "{turned:?}"
    );
}

#[test]
fn the_override_replaces_the_whole_pose_and_drops_the_base_orientation() {
    // 基準の B は +90° を持つ。形態は位置だけを書くので、向きは恒等になる（キーごとの合成はしない）
    let d = design(&[("configurations/MOVED.toml", MOVED)]);
    let r = fune_model::resolve(&d);
    let x = Vec3::new(1.0, 0.0, 0.0);
    assert!((body(&r, "B").1.rotate(x) - Vec3::new(0.0, 1.0, 0.0)).norm() < 1e-5);
    near(
        body(r.in_configuration(Some("MOVED")), "B").1.rotate(x),
        1.0,
        0.0,
        0.0,
    );
}

#[test]
fn unlisted_parts_keep_the_base() {
    let d = design(&[("configurations/MOVED.toml", MOVED)]);
    let r = fune_model::resolve(&d);
    near(
        body(r.in_configuration(Some("MOVED")), "A").0,
        0.0,
        0.0,
        0.0,
    );
}
