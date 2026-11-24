//! `structure.cone_clearance` の射影: 担い手の表（スラスタは推力の反対、センサは視線）・半頂角が無い担い手の扱い・
//! 自身の箱と取付先の祖先を判定しないこと・兄弟の検出・形態への追従・距離の上限の設定。小さな設計をファイルで組み、手計算と比べる。
//!
//! 配置（機体の枠 `FRAME` は一辺 2 m の箱で、すべての部品を含む。部品は枠に取り付き、姿勢は恒等）:
//! - `THR`: スラスタ。x = 0.5 m、一辺 0.1 m。`direction` = −x（推力）なので噴流は +x へ、頂点は x = 0.55 m。半頂角 30°
//! - `THR2`: 半頂角を持たないスラスタ（判定せず一覧に残る）
//! - `STT`: 星センサ。y = 0.5 m、一辺 0.1 m。`boresight` = +y、`fov_half_angle` = 10°、頂点は y = 0.55 m
//! - `TARGET`: x = 1.5 m、一辺 0.2 m（x ∈ [1.4, 1.6]）。噴流の軸の上: 頂点から 1.4 − 0.55 = 0.85 m
//! - `BLOCK`: y = 1.2 m、一辺 0.2 m（y ∈ [1.1, 1.3]）。視野の軸の上: 頂点から 1.1 − 0.55 = 0.55 m

use fune_model::Design;
use fune_projection::{Context, structure};
use fune_store::FileSet;
use fune_store::read::design_of;
use sat_analysis::spec::{Analysis, Verdict};
use sat_analysis::structure::cone_clearance::{
    ConeClearance, ConeClearanceOutput, ConeKind, ConeSkipReason,
};

const FRAME: &str = "[attr]\nsize = { x = 2.0, y = 2.0, z = 2.0, unit = \"m\" }\n";
const THR: &str = "kind = \"thruster\"\n\n[mount]\non = \"FRAME\"\n\n[pose]\nposition = { x = 0.5, y = 0.0, z = 0.0 }\n\n[attr]\nsize = { x = 0.1, y = 0.1, z = 0.1, unit = \"m\" }\nplume_half_angle = { value = 30.0, unit = \"deg\" }\n\n[attr.direction]\nx = -1.0\ny = 0.0\nz = 0.0\n";
const THR2: &str = "kind = \"propulsion_thruster\"\n\n[mount]\non = \"FRAME\"\n\n[pose]\nposition = { x = 0.0, y = 0.0, z = 0.5 }\n\n[attr]\nsize = { x = 0.1, y = 0.1, z = 0.1, unit = \"m\" }\n\n[attr.direction]\nx = 0.0\ny = 0.0\nz = -1.0\n";
const STT: &str = "kind = \"star_tracker\"\n\n[mount]\non = \"FRAME\"\n\n[pose]\nposition = { x = 0.0, y = 0.5, z = 0.0 }\n\n[attr]\nsize = { x = 0.1, y = 0.1, z = 0.1, unit = \"m\" }\nfov_half_angle = { value = 10.0, unit = \"deg\" }\n\n[attr.boresight]\nx = 0.0\ny = 1.0\nz = 0.0\n";
const TARGET: &str = "[mount]\non = \"FRAME\"\n\n[pose]\nposition = { x = 1.5, y = 0.0, z = 0.0 }\n\n[attr]\nsize = { x = 0.2, y = 0.2, z = 0.2, unit = \"m\" }\n";
const BLOCK: &str = "[mount]\non = \"FRAME\"\n\n[pose]\nposition = { x = 0.0, y = 1.2, z = 0.0 }\n\n[attr]\nsize = { x = 0.2, y = 0.2, z = 0.2, unit = \"m\" }\n";
/// BLOCK を (−0.8, 0, 0) へ動かす形態（どちらの円錐からも外れる）
const STOWED: &str =
    "label = \"Stowed\"\n\n[pose.BLOCK]\nposition = { x = -0.8, y = 0.0, z = 0.0 }\n";

fn design(extra: &[(&str, &str)]) -> Design {
    let mut set = FileSet::new();
    set.insert("fune.toml", "[project]\nname = \"t\"\n");
    for (path, text) in [
        ("parts/FRAME.toml", FRAME),
        ("parts/THR.toml", THR),
        ("parts/THR2.toml", THR2),
        ("parts/STT.toml", STT),
        ("parts/TARGET.toml", TARGET),
        ("parts/BLOCK.toml", BLOCK),
    ] {
        set.insert(path, text);
    }
    for (path, text) in extra {
        set.insert(*path, *text);
    }
    design_of(&set)
}

fn run(d: &Design, configuration: Option<&str>) -> ConeClearanceOutput {
    let resolved = fune_model::resolve(d);
    let ctx = Context::bare(d, resolved.in_configuration(configuration));
    let input =
        structure::cone_clearance(&ctx).expect("the projection never reports missing input");
    ConeClearance::run(&input).expect("the analysis never fails on obstructions")
}

fn hits<'a>(out: &'a ConeClearanceOutput, carrier: &str) -> Vec<&'a str> {
    out.cones
        .iter()
        .find(|c| c.id == carrier)
        .map(|c| c.obstructions.iter().map(|o| o.part.as_str()).collect())
        .unwrap_or_default()
}

/// スラスタは推力の反対（+x）、センサは視線（+y）へ開く。軸・頂点は箱の面から。FRAME（取付先）・自身の箱は判定せず、兄弟の TARGET・BLOCK を判定する。
/// 噴流は TARGET（軸の上。距離 0.85 m）だけ、視野は BLOCK（軸の上。距離 0.55 m）だけ。違反は無い。
#[test]
fn thruster_uses_minus_direction_and_sensor_uses_boresight() {
    let out = run(&design(&[]), None);
    assert_eq!(
        out.cones.iter().map(|c| c.id.as_str()).collect::<Vec<_>>(),
        ["STT", "THR"]
    );
    let thr = out.cones.iter().find(|c| c.id == "THR").unwrap();
    assert_eq!(thr.kind, ConeKind::Plume);
    assert!((thr.cone_axis.x - 1.0).abs() < 1e-12 && thr.cone_axis.y.abs() < 1e-12);
    assert!((thr.apex.x - 0.55).abs() < 1e-12);
    assert!((thr.half_angle.value() - 30.0_f64.to_radians()).abs() < 1e-12);
    assert_eq!(hits(&out, "THR"), ["TARGET"]);
    let o = &thr.obstructions[0];
    assert!(o.min_angle.value().abs() < 1e-12);
    assert!((o.min_distance.value() - 0.85).abs() < 1e-9);

    let stt = out.cones.iter().find(|c| c.id == "STT").unwrap();
    assert_eq!(stt.kind, ConeKind::FieldOfView);
    assert!((stt.cone_axis.y - 1.0).abs() < 1e-12);
    assert_eq!(hits(&out, "STT"), ["BLOCK"]);
    assert!((stt.obstructions[0].min_distance.value() - 0.55).abs() < 1e-9);

    // 枠（取付先）は数えるが判定せず、自身は数えない
    assert!(out.cones.iter().all(|c| c.parts_in_mount_chain == 1));
    assert!(out.checks.iter().all(|c| c.verdict == Verdict::Warning));
    let keys: Vec<&str> = out.checks.iter().map(|c| c.key.as_str()).collect();
    assert!(keys.contains(&"obstruction:THR:TARGET") && keys.contains(&"obstruction:STT:BLOCK"));
}

/// 半頂角を持たないスラスタは、判定せず理由つきで一覧に残す（不足入力にも、既定の仮定にもしない）。
#[test]
fn a_thruster_without_a_plume_half_angle_is_listed() {
    let out = run(&design(&[]), None);
    assert_eq!(out.skipped_carriers.len(), 1);
    assert_eq!(out.skipped_carriers[0].id, "THR2");
    assert_eq!(out.skipped_carriers[0].reason, ConeSkipReason::NoHalfAngle);
    assert_eq!(out.skipped_carriers[0].kind, ConeKind::Plume);
}

/// 形態を選ぶと、その配置で判定する: BLOCK を動かす形態では視野の遮りが消え、噴流は変わらない。基準の配置は変わらない。
#[test]
fn the_analysis_follows_the_configuration_in_effect() {
    let d = design(&[("configurations/STOWED.toml", STOWED)]);
    let stowed = run(&d, Some("STOWED"));
    assert!(hits(&stowed, "STT").is_empty());
    assert_eq!(hits(&stowed, "THR"), ["TARGET"]);
    assert_eq!(run(&d, None).cones_obstructed, 2);
}

/// `analysis/structure.toml` の `cone_clearance_range` が距離の上限になる（0.8 m なら、頂点から 0.85 m の TARGET は入らない。0.55 m の BLOCK は上限 0.8 m 以内）。
#[test]
fn the_range_setting_is_read() {
    let d = design(&[(
        "analysis/structure.toml",
        "cone_clearance_range = { value = 0.8, unit = \"m\" }\n",
    )]);
    let out = run(&d, None);
    assert!(!out.range_is_default);
    assert!((out.range.value() - 0.8).abs() < 1e-12);
    assert!(hits(&out, "THR").is_empty());
    assert_eq!(hits(&out, "STT"), ["BLOCK"]);
}

/// 部品が無い設計は、担い手も箱も無く、ok の成功（不足入力にしない）。
#[test]
fn an_empty_design_is_an_empty_success() {
    let mut set = FileSet::new();
    set.insert("fune.toml", "[project]\nname = \"t\"\n");
    let d = design_of(&set);
    let out = run(&d, None);
    assert_eq!((out.cones_judged, out.checks[0].verdict), (0, Verdict::Ok));
}
