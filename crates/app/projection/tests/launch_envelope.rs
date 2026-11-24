//! `structure.launch_fit` の収納外形: 形態が効いていれば全部品の箱の包絡、効いていなければ取付の根の寸法。
//! 小さな設計は手計算（箱 A 0.1 × 0.2 × 0.3 m を原点に、板 B 0.002 × 0.2 × 0.3 m を取付）と比べ、
//! デモは、収納の形態 STOWED の箱から試験の中で包絡を求め直して比べる。

use fune_model::Design;
use fune_projection::configuration_policy::ConfigurationSource;
use fune_projection::{Context, project_json_with_configuration, structure};
use fune_store::FileSet;
use fune_store::read::design_of;
use sat_analysis::spec::{Analysis, Verdict};
use sat_analysis::structure::interference::obb::{OrientedBox, separation};
use sat_analysis::structure::launch_fit::LaunchFitEnvelopeSource;
use sat_analysis::structure::{Interference, LaunchFit, LaunchFitInput, LaunchFitOutput};
use std::collections::BTreeSet;
use std::path::Path;

const FUNE: &str = "[project]\nname = \"t\"\n[[catalog]]\nname = \"local\"\npath = \"catalog\"\n";
/// 許容の外形 0.11 × 0.24 × 0.38 m のディスペンサー
const DISPENSER: &str = "kind = \"dispenser\"\n[attr]\nlaunch_envelope = { x = 110.0, y = 240.0, z = 380.0, unit = \"mm\" }\n";
const LAUNCH: &str = "dispenser = \"local/disp\"\n";
const A: &str = "[attr]\nsize = { x = 0.1, y = 0.2, z = 0.3, unit = \"m\" }\nmass = { value = 4.0, unit = \"kg\" }\n";
/// 基準: A の +y 0.2 m に置く板（A の外）
const B: &str = "[mount]\non = \"A\"\n[pose]\nposition = { x = 0.0, y = 0.2, z = 0.0 }\n[attr]\nsize = { x = 0.002, y = 0.2, z = 0.3, unit = \"m\" }\nmass = { value = 1.0, unit = \"kg\" }\n";
/// 寸法を持たない部品
const C: &str = "[mount]\non = \"A\"\n[attr]\nmass = { value = 0.5, unit = \"kg\" }\n";
/// 収納: B を x = 0.051 m（A の面に重ねる）。x の範囲は A の −0.05 から B の 0.052 まで
const STOWED: &str = "[pose.B]\nposition = { x = 0.051, y = 0.0, z = 0.0 }\n";
/// B を x = 0.07 m に離した別の形態（実行時に明示して、打ち上げ条件より優先することを確かめる）
const FAR: &str = "[pose.B]\nposition = { x = 0.07, y = 0.0, z = 0.0 }\n";
/// B を z まわりに 90° 回して原点に置く形態（B の x・y の長さが入れ替わる）
const TURNED: &str = "[pose.B]\nposition = { x = 0.0, y = 0.0, z = 0.0 }\norientation = { w = 0.7071067811865476, x = 0.0, y = 0.0, z = 0.7071067811865476 }\n";

fn design(launch: &str) -> Design {
    let mut set = FileSet::new();
    set.insert("fune.toml", FUNE);
    set.insert("catalog/disp.toml", DISPENSER);
    set.insert("conditions/launch.toml", launch);
    for (path, text) in [
        ("parts/A.toml", A),
        ("parts/B.toml", B),
        ("parts/C.toml", C),
        ("configurations/STOWED.toml", STOWED),
        ("configurations/FAR.toml", FAR),
        ("configurations/TURNED.toml", TURNED),
        // 打ち上げ条件と違う形態を指すフェーズとケース
        ("phases/OPS.toml", "configuration = \"FAR\"\n"),
        ("cases/ops.toml", "phase = \"OPS\"\n"),
    ] {
        set.insert(path, text);
    }
    design_of(&set)
}

/// `structure.launch_fit` を、実行時の形態の指定とケースを受けて射影して実行する。
fn run(
    d: &Design,
    case: Option<&str>,
    explicit: Option<&str>,
) -> (LaunchFitOutput, Option<String>) {
    let resolved = fune_model::resolve(d);
    let ctx = Context::new(d, &resolved, case).unwrap();
    let (json, used) = project_json_with_configuration(LaunchFit::ID, &ctx, explicit);
    let input: LaunchFitInput = serde_json::from_value(json.unwrap()).unwrap();
    let used = used.map(|(id, source)| format!("{id}:{source:?}"));
    (LaunchFit::run(&input).unwrap(), used)
}

fn sizes(out: &LaunchFitOutput) -> Vec<f64> {
    out.envelope.iter().map(|a| a.size.value()).collect()
}

fn near(got: &[f64], want: &[f64]) {
    assert!(
        got.iter().zip(want).all(|(g, w)| (g - w).abs() < 1e-9),
        "{got:?} != {want:?}"
    );
}

/// 打ち上げ条件が形態を指さなければ、根の寸法 0.1 × 0.2 × 0.3 m のまま（板 B は測らない）。出力に外形の出どころは書かない。
#[test]
fn without_a_launch_configuration_the_root_size_is_used() {
    let d = design(LAUNCH);
    let (out, used) = run(&d, None, None);
    assert_eq!(used, None);
    assert_eq!(out.envelope_source, LaunchFitEnvelopeSource::RootSize);
    near(&sizes(&out), &[0.1, 0.2, 0.3]);
    assert!(out.skipped.is_empty());
    let json = serde_json::to_value(&out).unwrap();
    assert!(json.get("envelope_source").is_none());
}

/// 打ち上げ条件が STOWED を指せば、全部品の箱の包絡: x は A の −0.05 から B の 0.051 + 0.001 = 0.052 までの 0.102 m（中心は +0.001 m）、
/// y は 0.2 m、z は 0.3 m。x の最大は B。寸法の無い C は測らず、一覧にする。
#[test]
fn a_launch_configuration_measures_all_part_boxes() {
    let d = design(&format!("{LAUNCH}configuration = \"STOWED\"\n"));
    let (out, used) = run(&d, None, None);
    assert_eq!(used.as_deref(), Some("STOWED:Launch"));
    assert_eq!(out.envelope_source, LaunchFitEnvelopeSource::Parts);
    near(&sizes(&out), &[0.102, 0.2, 0.3]);
    let x = &out.envelope[0];
    assert_eq!(
        (x.at_min.as_slice(), x.at_max.as_slice()),
        (&["A".to_owned()][..], &["B".to_owned()][..])
    );
    assert!((x.centre_offset.unwrap().value() - 0.001).abs() < 1e-12);
    assert!(out.envelope.iter().all(|a| a.verdict == Verdict::Ok));
    assert_eq!(out.skipped.len(), 1);
    assert_eq!(out.skipped[0].id, "C");
    let message = &out
        .checks
        .iter()
        .find(|c| c.key == "envelope:x")
        .unwrap()
        .message;
    assert!(message.contains("at B"), "{message}");
}

/// 打ち上げ向けの解析は、ケースのフェーズが別の形態（FAR）を指していても打ち上げ条件の形態を使う。
/// 実行時の明示（FAR: B が x = 0.07 なので、x は −0.05 から 0.071 までの 0.121 m）が打ち上げ条件に勝つ。
#[test]
fn launch_configuration_beats_the_phase_and_an_explicit_one_beats_both() {
    let d = design(&format!("{LAUNCH}configuration = \"STOWED\"\n"));
    let (out, used) = run(&d, Some("ops"), None);
    assert_eq!(used.as_deref(), Some("STOWED:Launch"));
    near(&sizes(&out), &[0.102, 0.2, 0.3]);
    let (out, used) = run(&d, Some("ops"), Some("FAR"));
    assert_eq!(used.as_deref(), Some("FAR:Explicit"));
    near(&sizes(&out), &[0.121, 0.2, 0.3]);
    assert_eq!(out.envelope[0].verdict, Verdict::Violation);
    // 打ち上げ条件が形態を指さなければ、ケースのフェーズの形態は読まない（根の寸法）
    let d = design(LAUNCH);
    let (out, used) = run(&d, Some("ops"), None);
    assert_eq!(used, None);
    assert_eq!(out.envelope_source, LaunchFitEnvelopeSource::RootSize);
}

/// 形態で B を z まわりに 90° 回すと、B の 0.002 × 0.2 の辺が入れ替わり、x の範囲は B の ±0.1 m になる（0.2 m）。y は A の 0.2 m のまま。
#[test]
fn a_rotated_part_swaps_its_extents_in_the_envelope() {
    let d = design(&format!("{LAUNCH}configuration = \"TURNED\"\n"));
    let (out, _) = run(&d, None, None);
    near(&sizes(&out), &[0.2, 0.2, 0.3]);
    assert_eq!(out.envelope[0].at_min, ["B"]);
    assert_eq!(out.envelope[0].at_max, ["B"]);
}

fn demo() -> Design {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../../fixtures/demo-6u");
    fune_store::load(&root)
        .expect("the demo should load")
        .design
}

/// 箱の機体座標の範囲 `(最小, 最大)`（8 つの頂点を回して取る。包絡の実装は使わない）。
fn extent(b: &structure::BodyBox) -> ([f64; 3], [f64; 3]) {
    let mut lo = [f64::INFINITY; 3];
    let mut hi = [f64::NEG_INFINITY; 3];
    for i in 0..8 {
        let sign = |k: usize| if (i >> k) & 1 == 1 { 0.5 } else { -0.5 };
        let corner = b.orientation.rotate(sat_astro::math::Vec3::new(
            b.size.x * sign(0),
            b.size.y * sign(1),
            b.size.z * sign(2),
        )) + b.position;
        for (k, v) in [corner.x, corner.y, corner.z].into_iter().enumerate() {
            lo[k] = lo[k].min(v);
            hi[k] = hi[k].max(v);
        }
    }
    (lo, hi)
}

/// デモの STOWED: 包絡は、STOWED の配置の全部品の箱から試験の中で求め直した範囲と同じ（寸法を持たない 2 つは測らない）。
/// 打ち上げ条件が STOWED を指す設計と、実行時に STOWED を明示した設計の結果は同じ。
///
/// 既知: x はディスペンサーの許容に収まらない。太陽センサー SS_MX・SS_PX の箱が両面の外へ出る（−59.0〜56.5 mm = 115.5 mm）ので、
/// 収納したパドルを面に重ねなくても 110 mm を超える。y・z は収まる。判断が付いたら、この断定を直す（#374）。
#[test]
fn the_demo_stowed_envelope_matches_its_part_boxes() {
    let d = demo();
    let resolved = fune_model::resolve(&d);
    let bare = Context::bare(&d, resolved.in_configuration(Some("STOWED")));
    let boxes = structure::boxes(&bare);
    let (mut lo, mut hi) = ([f64::INFINITY; 3], [f64::NEG_INFINITY; 3]);
    for b in &boxes.boxes {
        let (l, h) = extent(b);
        for k in 0..3 {
            lo[k] = lo[k].min(l[k]);
            hi[k] = hi[k].max(h[k]);
        }
    }
    let ctx = Context::new(&d, &resolved, None).unwrap();
    let (out, used) = {
        let (json, used) = project_json_with_configuration(LaunchFit::ID, &ctx, Some("STOWED"));
        let input: LaunchFitInput = serde_json::from_value(json.unwrap()).unwrap();
        (LaunchFit::run(&input).unwrap(), used)
    };
    assert_eq!(used, Some(("STOWED", ConfigurationSource::Explicit)));
    assert_eq!(out.envelope_source, LaunchFitEnvelopeSource::Parts);
    for (k, axis) in out.envelope.iter().enumerate() {
        assert!(
            (axis.size.value() - (hi[k] - lo[k])).abs() < 1e-12,
            "{}",
            axis.axis
        );
        assert!((axis.centre_offset.unwrap().value() - (hi[k] + lo[k]) / 2.0).abs() < 1e-12);
        // 余裕は許容 / 外形 − 1。判定はその符号
        let fits = axis.limit.value() >= axis.size.value();
        assert_eq!(axis.verdict == Verdict::Ok, fits, "{}", axis.axis);
    }
    let verdicts: Vec<Verdict> = out.envelope.iter().map(|a| a.verdict).collect();
    assert_eq!(verdicts, [Verdict::Violation, Verdict::Ok, Verdict::Ok]);
    assert_eq!(out.envelope[0].at_max, ["SAP_PY"]);
    let skipped: Vec<&str> = out.skipped.iter().map(|s| s.id.as_str()).collect();
    assert_eq!(skipped, boxes.unsized_parts);
}

/// デモの STOWED の干渉: 基準で重なっていない組は、STOWED でも重ならない（新しい重なりは無い）。基準にあった SAP_PY–SP_PY
/// （ヒンジ縁の箱の食い込み 1 mm）は、パドルが +X 面へ移って無くなる。
#[test]
fn the_demo_stowed_paddle_adds_no_overlap() {
    let d = demo();
    let resolved = fune_model::resolve(&d);
    let overlaps = |configuration: Option<&str>| -> BTreeSet<(String, String)> {
        let ctx = Context::bare(&d, resolved.in_configuration(configuration));
        let input = structure::interference(&ctx).unwrap();
        Interference::run(&input)
            .unwrap()
            .overlaps
            .into_iter()
            .map(|p| (p.first, p.second))
            .collect()
    };
    let (base, stowed) = (overlaps(None), overlaps(Some("STOWED")));
    assert!(
        stowed.is_subset(&base),
        "new: {:?}",
        stowed.difference(&base)
    );
    let gone: Vec<_> = base.difference(&stowed).collect();
    assert_eq!(gone, [&("SAP_PY".to_owned(), "SP_PY".to_owned())]);
}

/// デモの STOWED のパドルは、+X 面の太陽電池パネルと太陽センサーから正の隙間を持つ。
/// 手計算（mm）: 太陽電池 SP_PX の x は 51.2〜52.8、太陽センサー SS_PX は 46.5〜56.5。収納したパドルの中心面は 50 + 9.2 = 59.2、
/// 厚さ 2 で 58.2〜60.2。セル（厚さ 0.3、パドルの中心面から 2 だけ本体側）は 57.05〜57.35。よってセルと SS_PX の隙間は 57.05 − 56.5 = 0.55、
/// SP_PX との隙間は 57.05 − 52.8 = 4.25、パドルの板と SS_PX は 58.2 − 56.5 = 1.7。
#[test]
fn the_demo_stowed_paddle_clears_the_body_panel_and_the_sun_sensor() {
    let d = demo();
    let resolved = fune_model::resolve(&d);
    let boxes = structure::boxes(&Context::bare(
        &d,
        resolved.in_configuration(Some("STOWED")),
    ));
    let find = |id: &str| {
        let b = boxes.boxes.iter().find(|b| b.id == id).unwrap();
        OrientedBox::from_size(b.position, b.orientation, b.size)
    };
    let gap = |a: &str, b: &str| separation(&find(a), &find(b)).distance;
    for (a, b, mm) in [
        ("CELL_MOD_1", "SS_PX", 0.55),
        ("CELL_MOD_1", "SP_PX", 4.25),
        ("SAP_PY", "SS_PX", 1.7),
        ("SAP_PY", "SP_PX", 5.4),
    ] {
        let g = gap(a, b);
        assert!((g - mm * 1e-3).abs() < 1e-9, "{a} {b}: {g}");
    }
}
