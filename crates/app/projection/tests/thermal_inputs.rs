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

/// 明示の結合（`[conductive.*]`・`[radiative.*]`）と外殻（`[enclosure]`）は、解析の入力にそのまま渡る
/// （置き換えと優先順位は解析が決める）。箱の寸法は機体の根（BUS_FRAME）の外形（100 × 226 × 366 mm）。
#[test]
fn explicit_couplings_and_enclosure_reach_the_analysis() {
    let mut design = demo();
    let t = design.analysis.thermal.as_mut().unwrap();
    t.radiative.entry("PANEL_PX".into()).or_default().insert(
        "PANEL_MX".into(),
        fune_model::value::Quantity::new(0.02, "m^2"),
    );
    let resolved = fune_model::resolve(&design);
    let ctx = Context::new(&design, &resolved, None).unwrap();
    let net = fune_projection::thermal::steady(&ctx).unwrap().network;
    let strap = net
        .conductive
        .iter()
        .find(|l| (l.a.as_str(), l.b.as_str()) == ("BAT", "PANEL_MZ"))
        .unwrap();
    // #472: デモの電池は断熱した箱で、機体と PANEL_MZ への結合を細くした（0.8 → 0.03 W/K）
    assert_eq!(strap.conductance.value(), 0.03);
    assert_eq!(net.radiative.len(), 1);
    assert_eq!(net.radiative[0].exchange_area.value(), 0.02);
    let e = net.enclosure.unwrap();
    assert_eq!(e.emissivity, 0.85);
    let size = e.size.unwrap();
    let mm = |q: sat_units::typed::Qty<sat_units::typed::Length>| (q.value() * 1000.0).round();
    assert_eq!((mm(size.x), mm(size.y), mm(size.z)), (100.0, 226.0, 366.0));
}

/// 外殻は `[enclosure] emissivity` を書いたときだけ。書かなければ従来どおり（解析へ渡さない）。
/// 結合の量の次元が違えば不足入力に出す（黙って捨てない）。
#[test]
fn enclosure_is_off_without_emissivity_and_bad_units_are_missing() {
    let mut design = demo();
    design.analysis.thermal.as_mut().unwrap().enclosure = None;
    let resolved = fune_model::resolve(&design);
    let ctx = Context::new(&design, &resolved, None).unwrap();
    let net = fune_projection::thermal::steady(&ctx).unwrap().network;
    assert!(net.enclosure.is_none());

    let t = design.analysis.thermal.as_mut().unwrap();
    t.conductive.get_mut("BAT").unwrap().insert(
        "PANEL_MZ".into(),
        fune_model::value::Quantity::new(0.8, "m^2"),
    );
    let resolved = fune_model::resolve(&design);
    let ctx = Context::new(&design, &resolved, None).unwrap();
    let Err(missing) = fune_projection::thermal::steady(&ctx) else {
        panic!("a conductance in m^2 was accepted");
    };
    assert!(
        missing
            .into_vec()
            .iter()
            .any(|m| m.path.contains("thermal.conductive.BAT.PANEL_MZ"))
    );
}

/// `setpoint` を持つヒータ（HTR_BAT・HTR_CAM）は `ThermalHeater` として渡り、平均電力の発熱の源（解析の入力）には入らない
/// （二重に数えない）。オンの電力は `power` の `on` の値、設定温度は SI（K）。モード表示用の `loads` には残る。
/// `setpoint` を消せば、ヒータは従来どおりモードの発熱の源になる。
#[test]
fn thermostat_heaters_are_passed_as_heaters_not_loads() {
    let design = demo();
    let resolved = fune_model::resolve(&design);
    let ctx = Context::new(&design, &resolved, None).unwrap();
    let input = fune_projection::thermal::steady(&ctx).unwrap();
    let case = input.cases.iter().find(|c| c.id == "cold").unwrap();
    let h = case.heaters.iter().find(|h| h.id == "HTR_BAT").unwrap();
    assert_eq!(h.node, "BAT");
    // #472: 電池の性能範囲（10〜35 °C に不確かさ 5 K と警告幅 5 K）に合わせ、設定温度を 5〜10 °C から 20〜25 °C にした
    assert!((h.on_below.value() - 293.15).abs() < 1e-9);
    assert!((h.off_above.value() - 298.15).abs() < 1e-9);
    assert_eq!(h.power.value(), 2.0);
    assert_eq!(case.heaters.len(), 2);
    let analysis_parts = &case.mode_loads.as_ref().unwrap().parts;
    assert!(analysis_parts.iter().all(|p| !p.id.starts_with("HTR_")));
    let s = design.analysis.thermal.as_ref().unwrap();
    let display = fune_projection::thermal::loads(&ctx, s, Some("SAFE")).unwrap();
    assert!(display.parts.iter().any(|p| p.id == "HTR_BAT"));
    // setpoint の無いヒータは、これまでどおりモードの発熱の源（ヒータとしては渡さない）
    let mut plain = demo();
    plain
        .parts
        .get_mut("HTR_BAT")
        .unwrap()
        .attr
        .remove("setpoint");
    let resolved = fune_model::resolve(&plain);
    let ctx = Context::new(&plain, &resolved, None).unwrap();
    let input = fune_projection::thermal::steady(&ctx).unwrap();
    let case = &input.cases[0];
    assert!(case.heaters.iter().all(|h| h.id != "HTR_BAT"));
    assert!(
        case.mode_loads
            .as_ref()
            .unwrap()
            .parts
            .iter()
            .any(|p| p.id == "HTR_BAT")
    );
}

/// オンの電力が無いヒータは、黙って外さず不足入力に `<部品>.power` を出す。
#[test]
fn heater_without_on_power_is_reported_as_missing() {
    let mut design = demo();
    design
        .parts
        .get_mut("HTR_CAM")
        .unwrap()
        .attr
        .remove("power");
    let resolved = fune_model::resolve(&design);
    let ctx = Context::new(&design, &resolved, None).unwrap();
    let Err(missing) = fune_projection::thermal::steady(&ctx) else {
        panic!("the input was built although a heater has no power");
    };
    assert!(missing.into_vec().iter().any(|m| m.path == "HTR_CAM.power"));
}
