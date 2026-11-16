//! 熱の入力の射影（3 種の温度範囲・RF 出力・発熱の場所・表面材・プリセット・解析マージン）をデモで確かめる。

use fune_model::design::ThermalPresetKind;
use fune_projection::Context;
use sat_analysis::thermal::ThermalPreset;
use std::path::Path;

fn demo() -> fune_model::Design {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../../fixtures/demo-6u");
    fune_store::load(&root)
        .expect("the demo should load")
        .design
}

/// 温度範囲は 保管 = `temp_survival`、動作 = `temp_op`、性能 = `temp_perf` から読む（BAT のカタログ品は 3 種とも持つ）。
#[test]
fn three_ranges_come_from_the_part_attributes() {
    let design = demo();
    let resolved = fune_model::resolve(&design);
    let ctx = Context::new(&design, &resolved, None).unwrap();
    let input = fune_projection::thermal::steady(&ctx).unwrap();
    let bat = input.network.nodes.iter().find(|n| n.id == "BAT").unwrap();
    let part = bat.parts.iter().find(|p| p.id == "BAT").unwrap();
    let c = |k: f64| k - 273.15;
    let r = part.ranges;
    assert_eq!(r.storage.map(|r| c(r.min.value()).round()), Some(-20.0));
    assert_eq!(r.operating.map(|r| c(r.max.value()).round()), Some(45.0));
    assert_eq!(r.performance.map(|r| c(r.min.value()).round()), Some(10.0));
}

/// 表面材を書いた PANEL_PX は部品の α/ε を使わず、表面材の面積割合と α/ε を渡す。書いていない面は部品の α/ε のまま。
/// 環境条件のプリセットと解析マージンも設定から渡る（値の既定は解析が持つ）。
#[test]
fn coatings_presets_and_margin_reach_the_analysis() {
    let design = demo();
    let resolved = fune_model::resolve(&design);
    let ctx = Context::new(&design, &resolved, None).unwrap();
    let input = fune_projection::thermal::steady(&ctx).unwrap();
    let px = input
        .network
        .surfaces
        .iter()
        .find(|s| s.id == "PANEL_PX")
        .unwrap();
    assert_eq!((px.absorptivity, px.emissivity), (None, None));
    let shares: Vec<(&str, f64)> = px
        .coatings
        .iter()
        .map(|c| (c.id.as_str(), c.area_fraction))
        .collect();
    assert_eq!(shares, [("black_paint", 0.14), ("white_paint", 0.86)]);
    let (a, e) = px.optics().unwrap();
    assert!((a - (0.86 * 0.2 + 0.14 * 0.92)).abs() < 1e-12);
    assert!((e - (0.86 * 0.85 + 0.14 * 0.86)).abs() < 1e-12);
    let my = input
        .network
        .surfaces
        .iter()
        .find(|s| s.id == "PANEL_MY")
        .unwrap();
    assert!(my.coatings.is_empty() && my.absorptivity.is_some());
    let preset = |id: &str| {
        input
            .cases
            .iter()
            .find(|c| c.id == id)
            .unwrap()
            .environment
            .preset
    };
    assert_eq!(preset("hot"), Some(ThermalPreset::Hot));
    assert_eq!(preset("cold_nominal"), Some(ThermalPreset::Cold));
    assert_eq!(input.margin.uncertainty.map(|u| u.value()), Some(5.0));
    assert!(input.margin.warning_band.is_none());
    assert_eq!(ThermalPresetKind::ALL.len(), 3);
}

/// RF 出力はモードの発熱から引かれる（S バンド送信機は tx で 9.5 W・RF 2 W → 7.5 W）。`[heat_node]` に書くと
/// 発熱の節点が変わり、節点に入っていない部品も発熱の源になる。
#[test]
fn rf_output_and_heat_node() {
    let mut design = demo();
    let thermal = design.analysis.thermal.as_mut().unwrap();
    thermal.heat_node.insert("STX".into(), "PANEL_MZ".into());
    let resolved = fune_model::resolve(&design);
    let ctx = Context::new(&design, &resolved, None).unwrap();
    let s = design.analysis.thermal.as_ref().unwrap();
    let loads = fune_projection::thermal::loads(&ctx, s, Some("DOWNLINK")).unwrap();
    let stx = loads.parts.iter().find(|p| p.id == "STX").unwrap();
    assert_eq!(stx.node, "PANEL_MZ");
    assert_eq!(stx.rf_output.get("tx").map(|q| q.value()), Some(2.0));
    // DOWNLINK では STX は常に tx（modes/DOWNLINK.toml）
    assert_eq!(design.modes["DOWNLINK"].states["STX"].state, "tx");
    let (heat, _) = loads.evaluate_parts().unwrap();
    let got = heat
        .iter()
        .find(|(id, _)| id == "STX")
        .unwrap()
        .1
        .unwrap()
        .value();
    assert!((got - 7.5).abs() < 1e-9, "{got}");
}

/// 節点に入る部品の質量が無ければ、黙って節点から外さず不足入力に `<部品>.mass` を出す。
#[test]
fn node_part_without_mass_is_reported_as_missing() {
    let mut design = demo();
    let bat = design.parts.get_mut("BAT").unwrap();
    bat.attr.remove("mass");
    bat.catalog = None;
    let resolved = fune_model::resolve(&design);
    let ctx = Context::new(&design, &resolved, None).unwrap();
    let Err(missing) = fune_projection::thermal::steady(&ctx) else {
        panic!("the input was built although a part has no mass");
    };
    assert!(missing.into_vec().iter().any(|m| m.path == "BAT.mass"));
}
