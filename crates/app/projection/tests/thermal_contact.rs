//! 取付の伝導を箱の接触面積から出す設定（`contact_conductance_per_area`）の射影: 書かなければ入力は従来どおりで、書けば取付ごとの
//! 両端の箱が解析へ渡り、箱を持つ接する取付は `h × A`、寸法の無い部品の取付は 1 取付あたりの値になること。手計算（0.1 × 0.05 m の面）と比べる。

use fune_model::Design;
use fune_projection::{Context, MissingInputs, thermal};
use fune_store::FileSet;
use fune_store::read::design_of;
use sat_analysis::thermal::ThermalSteadyInput;

const ORBIT: &str = include_str!("../../../../fixtures/demo-6u/conditions/orbit.toml");
const AOCS: &str = "[law.NOM]\nlaw = \"sun_pointing\"\naxis = { x = 1.0, y = 0.0, z = 0.0 }\nsecondary = { x = 0.0, y = 0.0, z = 1.0 }\n";
const THERMAL: &str = "contact_conductance = { value = 0.5, unit = \"W/K\" }\n\
    orbits = 1\ntime_step = { value = 60.0, unit = \"s\" }\n\n[case.x]\npreset = \"hot\"\n";
const MASS: &str = "mass = { value = 1.0, unit = \"kg\" }\nspecific_heat = { value = 900.0, unit = \"J/(kg*K)\" }\n";
/// 一辺 0.2 m の立方体（上面は z = 0.1 m）
const BASE: &str =
    "kind = \"structure\"\n\n[attr]\nsize = { x = 0.2, y = 0.2, z = 0.2, unit = \"m\" }\n";
/// 0.1 × 0.05 × 0.1 m の板を BASE の上面に載せる（中心 z = 0.15 m）: 接触面は 0.1 × 0.05 = 0.005 m²
const PLATE: &str = "kind = \"structure\"\n\n[mount]\non = \"BASE\"\n\n[pose]\nposition = { x = 0.0, y = 0.0, z = 0.15 }\n\n[attr]\nsize = { x = 0.1, y = 0.05, z = 0.1, unit = \"m\" }\n";
/// 寸法の無い部品
const BOX: &str = "kind = \"structure\"\n\n[mount]\non = \"BASE\"\n\n[attr]\n";

fn input(thermal_extra: &str) -> Result<ThermalSteadyInput, MissingInputs> {
    let mut set = FileSet::new();
    set.insert("fune.toml", "[project]\nname = \"t\"\n");
    set.insert("conditions/orbit.toml", ORBIT);
    set.insert("analysis/aocs.toml", AOCS);
    set.insert("analysis/thermal.toml", format!("{thermal_extra}{THERMAL}"));
    for (id, text) in [("BASE", BASE), ("PLATE", PLATE), ("BOX", BOX)] {
        set.insert(format!("parts/{id}.toml"), format!("{text}{MASS}"));
    }
    let d: Design = design_of(&set);
    let r = fune_model::resolve(&d);
    thermal::steady(&Context::new(&d, &r, None).unwrap())
}

#[test]
fn without_the_coefficient_nothing_is_passed() {
    let i = input("").unwrap();
    assert!(i.network.mounts.as_ref().unwrap().contact.is_none());
}

#[test]
fn with_the_coefficient_the_boxes_of_each_mount_are_passed() {
    let i =
        input("contact_conductance_per_area = { value = 300.0, unit = \"W/(m^2*K)\" }\n").unwrap();
    let mounts = i.network.mounts.as_ref().unwrap();
    let contact = mounts.contact.as_ref().unwrap();
    assert_eq!(contact.conductance_per_area.value(), 300.0);
    assert_eq!(contact.mounts.len(), mounts.mounts.len());
    let links = mounts.links().unwrap();
    let get = |a: &str, b: &str| {
        links
            .conductive
            .iter()
            .find(|l| l.a == a && l.b == b)
            .unwrap()
    };
    // 板: 300 W/(m²·K) × 0.005 m² = 1.5 W/K。寸法の無い部品: 1 取付あたりの 0.5 W/K
    let plate = get("BASE", "PLATE");
    assert!((plate.conductance.value() - 1.5).abs() < 1e-12);
    assert!((plate.contact.unwrap().area.value() - 0.005).abs() < 1e-15);
    let boxless = get("BASE", "BOX");
    assert_eq!(boxless.conductance.value(), 0.5);
    assert!(boxless.contact.is_none());
    let used = links.contact.unwrap();
    assert_eq!(used.geometric_mounts, 1);
    assert_eq!(used.fallbacks.len(), 1);
    assert_eq!(used.fallbacks[0].part, "BOX");
}
