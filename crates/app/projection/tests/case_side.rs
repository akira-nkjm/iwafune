//! 解析ケースの `side`（#162）: ケースが `max` / `min` を指すと、値が持つ `max` / `min` を読む。無ければ nominal、ケースが無ければ nominal。

use fune_model::design::UncertaintySide;
use fune_projection::Context;
use sat_analysis::power::mode_load::{ModeLoad, ModeLoadOutput};
use std::path::Path;

fn demo() -> (fune_model::Design, fune_model::Resolved) {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../../fixtures/demo-6u");
    let design = fune_store::load(&root)
        .expect("the demo should load")
        .design;
    let resolved = fune_model::resolve(&design);
    (design, resolved)
}

/// 姿勢制御ホイールの `on` の消費電力はデモで 0.6 W（min 0.5 W・max 0.75 W）。ケースの側で読む値が変わる。
fn rw_on_power(case: Option<&str>) -> f64 {
    let (design, resolved) = demo();
    let ctx = Context::new(&design, &resolved, case).unwrap();
    let part = &resolved.parts["RW_X"];
    let values =
        fune_projection::parts::state_values::<sat_units::typed::Power>(&ctx, part, "power")
            .expect("the wheel has a power table");
    values.by_state["on"].value()
}

/// どのケースの側でも、読む値は `max` / `min` があればそれ、無い値は nominal。ケースが無ければ nominal。
#[test]
fn state_values_follow_the_case_side() {
    assert_eq!(rw_on_power(None), 0.6);
    assert_eq!(rw_on_power(Some("eclipse_worst")), 0.75); // side = max
    assert_eq!(rw_on_power(Some("cold_safe")), 0.5); // side = min
}

/// 値が `max` を持たない状態（`idle` は nominal だけ）は、`side = max` でも nominal のまま。
#[test]
fn values_without_the_bound_stay_nominal() {
    let (design, resolved) = demo();
    let ctx = Context::new(&design, &resolved, Some("eclipse_worst")).unwrap();
    assert_eq!(ctx.side(), Some(UncertaintySide::Max));
    let part = &resolved.parts["RW_X"];
    let v = fune_projection::parts::state_values::<sat_units::typed::Power>(&ctx, part, "power")
        .unwrap();
    assert_eq!(v.by_state["idle"].value(), 0.18);
}

/// 範囲（`HTR_*` の `setpoint = { min, max }`）は不確かさではない。`side` で変わらず、スカラーとしても読まれない。
#[test]
fn ranges_are_not_uncertainty() {
    let (design, resolved) = demo();
    let ctx = Context::new(&design, &resolved, Some("eclipse_worst")).unwrap();
    let heater = &resolved.parts["HTR_BAT"];
    assert!(matches!(
        ctx.attr(heater, "setpoint"),
        Some(fune_model::resolve::SiValue::Range { .. })
    ));
    assert_eq!(ctx.scalar(heater, "setpoint"), None);
}

fn mode_total(case: Option<&str>, mode: &str) -> f64 {
    let (design, resolved) = demo();
    let ctx = Context::new(&design, &resolved, case).unwrap();
    let input = fune_projection::power::mode_load(&ctx).unwrap();
    let out: ModeLoadOutput = sat_analysis::catalog::run_typed::<ModeLoad>(&input).unwrap();
    out.modes
        .iter()
        .find(|m| m.id == mode)
        .unwrap()
        .total
        .value()
}

/// 部品の電力が `max` を持つと、`side = max` のケースだけ負荷が上がる。手計算: NOMINAL モードは 3 台のホイールが `on` なので、
/// 平均の合計は 3 × (0.75 − 0.6) = 0.45 W 増える（成長余裕なしの合計）。nominal のケース・ケース無しは変わらない。
#[test]
fn max_side_raises_the_mode_load_by_the_hand_calculation() {
    let base = mode_total(None, "NOMINAL");
    let max = mode_total(Some("eclipse_worst"), "NOMINAL");
    assert!((max - base - 0.45).abs() < 1e-9, "{base} -> {max}");
    // `side` を nominal にしたケースは、ケース無しと同じ。
    let (mut design, resolved) = demo();
    design.cases.get_mut("eclipse_worst").unwrap().side = Some(UncertaintySide::Nominal);
    let ctx = Context::new(&design, &resolved, Some("eclipse_worst")).unwrap();
    let input = fune_projection::power::mode_load(&ctx).unwrap();
    let out = sat_analysis::catalog::run_typed::<ModeLoad>(&input).unwrap();
    let nominal = out.modes.iter().find(|m| m.id == "NOMINAL").unwrap();
    assert!((nominal.total.value() - base).abs() < 1e-12);
}
