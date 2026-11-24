//! `structure.cone_clearance` の射影: アンテナのビーム（担い手の表の 1 行）。ボアサイトが軸、ビーム幅の半分が半頂角。
//! 小さな設計をファイルで組み、手計算と比べる。
//!
//! 配置（機体の枠 `FRAME` は一辺 2 m の箱で、部品は枠に取り付き、姿勢は恒等）:
//! - `ANT`: アンテナ。z = 0.5 m、一辺 0.1 m（z ∈ [0.45, 0.55]）。`boresight` = +z、`beamwidth` = 40° なので半頂角 20°。頂点は z = 0.55 m
//! - 遮る箱はどれも一辺 0.2 m（z ∈ [1.4, 1.6]）、x の中心だけが違う。頂点から箱の面までの z は 1.4 − 0.55 = 0.85 m、遠い面まで 1.6 − 0.55 = 1.05 m

use fune_model::Design;
use fune_projection::{Context, structure};
use fune_store::FileSet;
use fune_store::read::design_of;
use sat_analysis::spec::{Analysis, Verdict};
use sat_analysis::structure::cone_clearance::{
    ConeClearance, ConeClearanceOutput, ConeKind, ConeSkipReason,
};

const FRAME: &str = "[attr]\nsize = { x = 2.0, y = 2.0, z = 2.0, unit = \"m\" }\n";
const ANT: &str = "kind = \"antenna\"\n\n[mount]\non = \"FRAME\"\n\n[pose]\nposition = { x = 0.0, y = 0.0, z = 0.5 }\n\n[attr]\nsize = { x = 0.1, y = 0.1, z = 0.1, unit = \"m\" }\nantenna_gain = { value = 5.0, unit = \"dBi\" }\nbeamwidth = { value = 40.0, unit = \"deg\" }\n\n[attr.boresight]\nx = 0.0\ny = 0.0\nz = 1.0\n";
/// ビーム幅が無いアンテナ
const ANT_NO_BEAMWIDTH: &str = "kind = \"antenna\"\n\n[mount]\non = \"FRAME\"\n\n[pose]\nposition = { x = 0.5, y = 0.0, z = 0.0 }\n\n[attr]\nsize = { x = 0.1, y = 0.1, z = 0.1, unit = \"m\" }\nantenna_gain = { value = 5.0, unit = \"dBi\" }\n\n[attr.boresight]\nx = 1.0\ny = 0.0\nz = 0.0\n";
/// ボアサイトが無いアンテナ（全方向）
const ANT_OMNI: &str = "kind = \"antenna\"\n\n[mount]\non = \"FRAME\"\n\n[pose]\nposition = { x = -0.5, y = 0.0, z = 0.0 }\n\n[attr]\nsize = { x = 0.1, y = 0.1, z = 0.1, unit = \"m\" }\nantenna_gain = { value = 0.0, unit = \"dBi\" }\nbeamwidth = { value = 120.0, unit = \"deg\" }\n";

fn block(x: f64) -> String {
    format!(
        "[mount]\non = \"FRAME\"\n\n[pose]\nposition = {{ x = {x}, y = 0.0, z = 1.5 }}\n\n[attr]\nsize = {{ x = 0.2, y = 0.2, z = 0.2, unit = \"m\" }}\n"
    )
}

fn design(blocks: &[(&str, f64)], extra: &[(&str, &str)]) -> Design {
    let mut set = FileSet::new();
    set.insert("fune.toml", "[project]\nname = \"t\"\n");
    for (path, text) in [("parts/FRAME.toml", FRAME), ("parts/ANT.toml", ANT)] {
        set.insert(path, text);
    }
    for (id, x) in blocks {
        set.insert(format!("parts/{id}.toml"), block(*x));
    }
    for (path, text) in extra {
        set.insert(*path, *text);
    }
    design_of(&set)
}

fn run(d: &Design) -> ConeClearanceOutput {
    let resolved = fune_model::resolve(d);
    let ctx = Context::bare(d, resolved.in_configuration(None));
    let input =
        structure::cone_clearance(&ctx).expect("the projection never reports missing input");
    ConeClearance::run(&input).expect("the analysis never fails on obstructions")
}

/// 軸の上の箱は当たる（軸の角度 0、頂点から面まで 0.85 m）。ビーム幅 40° の半分 = 半頂角 20°、軸はボアサイト（+z）、頂点は箱の面（z = 0.55 m）。
#[test]
fn a_part_on_the_boresight_is_inside_the_beam() {
    let out = run(&design(&[("HIT", 0.0)], &[]));
    let ant = out.cones.iter().find(|c| c.id == "ANT").unwrap();
    assert_eq!(ant.kind, ConeKind::Beam);
    assert!((ant.half_angle.value() - 20.0_f64.to_radians()).abs() < 1e-12);
    assert!((ant.cone_axis.z - 1.0).abs() < 1e-12);
    assert!((ant.apex.z - 0.55).abs() < 1e-12);
    assert_eq!(ant.obstructions.len(), 1);
    let o = &ant.obstructions[0];
    assert_eq!(o.part, "HIT");
    assert!(o.min_angle.value().abs() < 1e-12);
    assert!((o.min_distance.value() - 0.85).abs() < 1e-9);
    let check = out.checks.iter().find(|c| c.key == "obstruction:ANT:HIT");
    assert_eq!(check.unwrap().verdict, Verdict::Warning);
    assert!(check.unwrap().label.starts_with("Antenna beam of ANT"));
}

/// 軸からずれた箱の境界。x ∈ [0.3, 0.5] の箱: 軸から最も近い点は x = 0.3・z = 1.6（頂点から 1.05 m）で、
/// 角度 atan(0.3 / 1.05) = 15.9° < 20° なので当たる。x ∈ [0.4, 0.6] の箱は atan(0.4 / 1.05) = 20.85° > 20° なので当たらない。
#[test]
fn an_off_axis_box_is_judged_against_the_half_angle() {
    let out = run(&design(&[("NEAR", 0.4), ("FAR", 0.5)], &[]));
    let ant = out.cones.iter().find(|c| c.id == "ANT").unwrap();
    let hits: Vec<&str> = ant.obstructions.iter().map(|o| o.part.as_str()).collect();
    assert_eq!(hits, ["NEAR"]);
    let angle = (0.3_f64 / 1.05).atan();
    assert!((ant.obstructions[0].min_angle.value() - angle).abs() < 1e-9);
    assert!(angle < 20.0_f64.to_radians() && (0.4_f64 / 1.05).atan() > 20.0_f64.to_radians());
}

/// ビーム幅が無いアンテナ、ボアサイトが無い（全方向の）アンテナは、判定せず理由つきで一覧に残す（不足入力にしない）。
#[test]
fn an_antenna_without_a_beamwidth_or_boresight_is_listed_not_judged() {
    let d = design(
        &[("HIT", 0.0)],
        &[
            ("parts/ANT_NO_BEAMWIDTH.toml", ANT_NO_BEAMWIDTH),
            ("parts/ANT_OMNI.toml", ANT_OMNI),
        ],
    );
    let out = run(&d);
    assert_eq!(
        out.cones.iter().map(|c| c.id.as_str()).collect::<Vec<_>>(),
        ["ANT"]
    );
    let skipped: Vec<_> = out
        .skipped_carriers
        .iter()
        .map(|s| (s.id.as_str(), s.kind, s.reason))
        .collect();
    assert_eq!(
        skipped,
        [
            (
                "ANT_NO_BEAMWIDTH",
                ConeKind::Beam,
                ConeSkipReason::NoHalfAngle
            ),
            ("ANT_OMNI", ConeKind::Beam, ConeSkipReason::NoAxis),
        ]
    );
}

/// 既存の承知の仕組みで、アンテナの所見も承知できる（担い手が先、部品が後）。承知済みは出力に残り、判定は ok。
#[test]
fn an_antenna_finding_can_be_acknowledged() {
    let acknowledged = "[[acknowledged]]\nfinding = \"cone_clearance\"\nfirst = \"ANT\"\nsecond = \"HIT\"\nreason = \"Radome is transparent\"\n";
    let d = design(
        &[("HIT", 0.0)],
        &[("analysis/structure.toml", acknowledged)],
    );
    let out = run(&d);
    let o = &out.cones[0].obstructions[0];
    assert!(o.acknowledged);
    assert_eq!(o.reason.as_deref(), Some("Radome is transparent"));
    let check = out.checks.iter().find(|c| c.key == "obstruction:ANT:HIT");
    assert_eq!(check.unwrap().verdict, Verdict::Ok);
}
