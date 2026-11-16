//! 射影が読んだものの記録（`ReadLog`）: デモの電源の解析が、入力に使う部品の属性・設定・条件・モードを記録すること。
//! 記録しても入力は変わらないこと。

use fune_model::Target;
use fune_model::path::SettingPath;
use fune_projection::{Context, ReadLog, project_json};
use std::collections::BTreeSet;
use std::path::Path;

fn demo() -> (fune_model::Design, fune_model::Resolved) {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../../fixtures/demo-6u");
    let snap = fune_store::load(&root).expect("the demo should load");
    let resolved = fune_model::resolve(&snap.design);
    (snap.design, resolved)
}

/// 解析 `id` の射影が読んだもの（ケースなし）。
fn reads(id: &str) -> BTreeSet<Target> {
    let (design, resolved) = demo();
    let log = ReadLog::new();
    let ctx = Context::new(&design, &resolved, None)
        .unwrap()
        .with_reads(Some(&log));
    let _ = project_json(id, &ctx);
    log.into_targets()
}

fn power(keys: &[&str]) -> Target {
    SettingPath::analysis(&["power"]).child(keys).into()
}

fn orbit(key: &str) -> Target {
    SettingPath::conditions(&["orbit", key]).into()
}

#[test]
fn orbit_generation_reads_solar_panel_values_and_orbit() {
    let r = reads("power.orbit_generation");
    for attr in [
        "area",
        "cell_efficiency",
        "packing_factor",
        "eol_factor",
        "normal",
    ] {
        assert!(
            r.contains(&Target::attr("SP_PX", attr)),
            "SP_PX.{attr} was not read: {r:?}"
        );
    }
    assert!(r.contains(&power(&["conversion_efficiency"])));
    assert!(r.contains(&orbit("altitude")));
    assert!(r.contains(&orbit("epoch")));
    // 電池・負荷の値は読まない
    assert!(!r.contains(&Target::attr("BAT", "capacity")));
    assert!(!r.contains(&power(&["dod_max"])));
}

#[test]
fn energy_balance_and_sizing_read_battery_capacity_and_power_settings() {
    for id in ["power.energy_balance", "power.sizing"] {
        let r = reads(id);
        for attr in ["capacity", "voltage"] {
            assert!(r.contains(&Target::attr("BAT", attr)), "{id}: BAT.{attr}");
        }
        assert!(r.contains(&power(&["dod_max"])), "{id}: dod_max");
        assert!(r.contains(&power(&["sequence"])), "{id}: sequence");
        assert!(r.contains(&Target::mode("NOMINAL", None)), "{id}: mode");
        assert!(r.contains(&Target::attr("SP_PX", "area")), "{id}: panel");
    }
    assert!(reads("power.sizing").contains(&orbit("duration")));
    assert!(!reads("power.energy_balance").contains(&orbit("duration")));
}

#[test]
fn mode_load_reads_power_of_loads_and_modes_only() {
    let r = reads("power.mode_load");
    assert!(r.iter().any(|t| matches!(t, Target::Mode { .. })));
    let attrs: BTreeSet<&str> = r
        .iter()
        .filter_map(|t| match t {
            Target::Param(fune_model::path::ParamPath::Attr { attr, .. }) => Some(attr.as_str()),
            _ => None,
        })
        .collect();
    assert_eq!(attrs, BTreeSet::from(["power"]));
    assert!(
        !r.iter()
            .any(|t| matches!(t, Target::Param(fune_model::path::ParamPath::Setting(_))))
    );
}

/// 記録してもしなくても、組み立てた入力は同じ。
#[test]
fn recording_does_not_change_inputs() {
    let (design, resolved) = demo();
    for e in fune_projection::entries() {
        let plain = Context::new(&design, &resolved, None).unwrap();
        let log = ReadLog::new();
        let logged = Context::new(&design, &resolved, None)
            .unwrap()
            .with_reads(Some(&log));
        let a = project_json(e.id, &plain).map_err(|e| e.to_string());
        let b = project_json(e.id, &logged).map_err(|e| e.to_string());
        assert_eq!(a, b, "{}", e.id);
    }
}
