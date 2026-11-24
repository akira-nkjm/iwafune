//! `structure.interference` の射影: 部品の箱を取付の連鎖から機体座標に置き、寸法の無い部品は一覧にするだけで不足入力にしないこと。
//! 小さな設計をファイルで組み、手計算（立方体の重なり）と比べる。形態を選んだ設計では、その配置で判定する。

use fune_model::Design;
use fune_projection::{Context, structure};
use fune_store::FileSet;
use fune_store::read::design_of;
use sat_analysis::spec::{Analysis, Verdict};
use sat_analysis::structure::Interference;

const FRAME: &str = "[attr]\nsize = { x = 1.0, y = 1.0, z = 1.0, unit = \"m\" }\n";
/// FRAME の中の一辺 0.2 m の箱。x = 0.3 m → x の範囲 0.2〜0.4 m
const B: &str = "[mount]\non = \"FRAME\"\n\n[pose]\nposition = { x = 0.3, y = 0.0, z = 0.0 }\n\n[attr]\nsize = { x = 0.2, y = 0.2, z = 0.2, unit = \"m\" }\n";
/// 同じく FRAME の中の一辺 0.2 m の箱。x = 0.45 m → 0.35〜0.55 m（B と x が 0.05 m 重なる）
const C: &str = "[mount]\non = \"FRAME\"\n\n[pose]\nposition = { x = 0.45, y = 0.0, z = 0.0 }\n\n[attr]\nsize = { x = 0.2, y = 0.2, z = 0.2, unit = \"m\" }\n";
/// 寸法の無い部品
const D: &str = "[mount]\non = \"FRAME\"\n";
/// C を x = 0.7 m（0.6〜0.8 m）へ動かす形態: B との隙間は 0.2 m
const FAR: &str = "label = \"Far\"\n\n[pose.C]\nposition = { x = 0.7, y = 0.0, z = 0.0 }\n";

fn design(extra: &[(&str, &str)]) -> Design {
    let mut set = FileSet::new();
    set.insert("fune.toml", "[project]\nname = \"t\"\n");
    for (path, text) in [
        ("parts/FRAME.toml", FRAME),
        ("parts/B.toml", B),
        ("parts/C.toml", C),
        ("parts/D.toml", D),
    ] {
        set.insert(path, text);
    }
    for (path, text) in extra {
        set.insert(*path, *text);
    }
    design_of(&set)
}

fn run(d: &Design, configuration: Option<&str>) -> sat_analysis::structure::InterferenceOutput {
    let resolved = fune_model::resolve(d);
    let ctx = Context::bare(d, resolved.in_configuration(configuration));
    let input = structure::interference(&ctx).expect("the projection never reports missing input");
    Interference::run(&input).expect("the analysis never fails on overlaps")
}

/// 箱は寸法を持つ 3 部品（FRAME・B・C）、D は一覧。FRAME は B・C の取付の祖先なので判定しない（2 組）。
/// B–C は x が 0.4 − 0.35 = 0.05 m 重なり（y・z は 0.2 m）、深さ 0.05 m、向きは B → C の +x。警告で、成功。
#[test]
fn boxes_come_from_the_mount_chain_and_the_overlap_is_a_warning() {
    let d = design(&[]);
    let resolved = fune_model::resolve(&d);
    let boxes = structure::boxes(&Context::bare(&d, &resolved));
    let ids: Vec<&str> = boxes.boxes.iter().map(|b| b.id.as_str()).collect();
    assert_eq!(ids, ["B", "C", "FRAME"]);
    assert_eq!(boxes.unsized_parts, ["D"]);
    assert_eq!(boxes.boxes[0].mount_ancestors, ["FRAME"]);

    let out = run(&d, None);
    assert_eq!(
        (out.parts_judged, out.pairs_judged, out.pairs_in_mount_chain),
        (3, 1, 2)
    );
    assert_eq!(out.overlaps.len(), 1);
    let p = &out.overlaps[0];
    assert_eq!((p.first.as_str(), p.second.as_str()), ("B", "C"));
    assert!((p.depth.value() - 0.05).abs() < 1e-12);
    assert!((p.separation_axis.x - 1.0).abs() < 1e-12);
    assert_eq!(out.skipped.len(), 1);
    assert_eq!(out.skipped[0].id, "D");
    assert!(out.checks.iter().all(|c| c.verdict == Verdict::Warning));
}

/// 形態 FAR では C が離れ、重なりが無い（B–C の隙間 0.6 − 0.4 = 0.2 m）。基準の配置は変わらない。
#[test]
fn the_analysis_follows_the_configuration_in_effect() {
    let d = design(&[("configurations/FAR.toml", FAR)]);
    let far = run(&d, Some("FAR"));
    assert!(far.overlaps.is_empty());
    assert!((far.min_clearance.unwrap().gap.value() - 0.2).abs() < 1e-12);
    assert_eq!(far.checks[0].verdict, Verdict::Ok);
    assert_eq!(run(&d, None).overlaps.len(), 1);
}

/// 寸法を持つ部品が 2 つに満たない設計は、重なり無しの成功（不足入力にしない）。
#[test]
fn fewer_than_two_boxes_is_an_empty_success() {
    let mut set = FileSet::new();
    set.insert("fune.toml", "[project]\nname = \"t\"\n");
    set.insert("parts/FRAME.toml", FRAME);
    set.insert("parts/D.toml", "");
    let d = design_of(&set);
    let out = run(&d, None);
    assert!(out.overlaps.is_empty());
    assert_eq!((out.parts_judged, out.pairs_judged), (1, 0));
    assert_eq!(out.skipped.len(), 1);
}

/// `analysis/structure.toml` の `interference_tolerance` が許容差になる（0.06 m なら深さ 0.05 m は報告しない）。
#[test]
fn the_tolerance_setting_is_read() {
    let d = design(&[(
        "analysis/structure.toml",
        "interference_tolerance = { value = 60.0, unit = \"mm\" }\n",
    )]);
    let out = run(&d, None);
    assert!((out.tolerance.value() - 0.06).abs() < 1e-12);
    assert!(out.overlaps.is_empty());
}
