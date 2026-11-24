//! 板の分け方（`[subdivide.<部品>]`）の射影: 升の数・箱・材料の熱伝導率・取付で結ばれた相手の箱が解析へ渡ること。
//! 足りないもの（`u`・`v`・材料・熱伝導率）は不足入力になること。

use fune_model::Design;
use fune_model::design::ThermalSubdivideSettings;
use fune_projection::{Context, MissingInputs, thermal};
use fune_store::FileSet;
use fune_store::read::design_of;
use sat_analysis::thermal::ThermalSteadyInput;
use sat_analysis::thermal::network::MAX_CELLS;

const ORBIT: &str = include_str!("../../../../fixtures/demo-6u/conditions/orbit.toml");
const AOCS: &str = "[law.NOM]\nlaw = \"sun_pointing\"\naxis = { x = 1.0, y = 0.0, z = 0.0 }\nsecondary = { x = 0.0, y = 0.0, z = 1.0 }\n";
const MASS: &str = "mass = { value = 1.0, unit = \"kg\" }\nspecific_heat = { value = 900.0, unit = \"J/(kg*K)\" }\n";
const MATERIAL: &str =
    "[material.alloy]\nthermal_conductivity = { value = 167.0, unit = \"W/(m*K)\" }\n";
/// 0.3 × 0.2 × 0.003 m の板（材料 alloy）と、その上面（z = 0.0015 m）に載る 0.05 m の箱
const PLATE: &str = "kind = \"structure\"\n\n[attr]\nmaterial = \"alloy\"\nsize = { x = 0.3, y = 0.2, z = 0.003, unit = \"m\" }\n";
const BOX: &str = "kind = \"structure\"\n\n[mount]\non = \"PLATE\"\n\n[pose]\nposition = { x = 0.0, y = 0.0, z = 0.0265 }\n\n[attr]\nsize = { x = 0.05, y = 0.05, z = 0.05, unit = \"m\" }\n";

fn input(thermal_extra: &str, material: &str) -> Result<ThermalSteadyInput, MissingInputs> {
    let mut set = FileSet::new();
    set.insert("fune.toml", "[project]\nname = \"t\"\n");
    set.insert("conditions/orbit.toml", ORBIT);
    set.insert("analysis/aocs.toml", AOCS);
    set.insert("analysis/structure.toml", material);
    set.insert(
        "analysis/thermal.toml",
        format!(
            "contact_conductance = {{ value = 0.5, unit = \"W/K\" }}\n{thermal_extra}\n[case.x]\npreset = \"hot\"\n"
        ),
    );
    for (id, text) in [("PLATE", PLATE), ("BOX", BOX)] {
        set.insert(format!("parts/{id}.toml"), format!("{text}{MASS}"));
    }
    let d: Design = design_of(&set);
    let r = fune_model::resolve(&d);
    thermal::steady(&Context::new(&d, &r, None).unwrap())
}

#[test]
fn nothing_is_passed_without_subdivide() {
    let i = input("orbits = 1\n", MATERIAL).unwrap();
    assert!(i.network.plates.is_empty());
}

#[test]
fn a_subdivided_plate_is_passed_with_its_box_material_and_neighbours() {
    let i = input("orbits = 1\n[subdivide.PLATE]\nu = 3\nv = 2\n", MATERIAL).unwrap();
    let [p] = i.network.plates.as_slice() else {
        panic!("{:?}", i.network.plates);
    };
    assert_eq!((p.part.as_str(), p.cells), ("PLATE", [3, 2]));
    assert_eq!(p.conductivity.value(), 167.0);
    assert!((p.frame.size.z - 0.003).abs() < 1e-15);
    // 箱は板に取り付く（板の取付先は無い）
    let nodes: Vec<_> = p.attachments.iter().map(|a| a.node.as_str()).collect();
    assert_eq!(nodes, ["BOX"]);
}

#[test]
fn missing_cells_and_conductivity_are_missing_inputs() {
    let missing = |extra: &str, material: &str| -> Vec<String> {
        input(extra, material)
            .unwrap_err()
            .into_vec()
            .into_iter()
            .map(|m| m.path)
            .collect()
    };
    let only_u = missing("[subdivide.PLATE]\nu = 3\n", MATERIAL);
    assert!(
        only_u
            .iter()
            .any(|t| t == "analysis.thermal.subdivide.PLATE.v"),
        "{only_u:?}"
    );
    let no_k = missing("[subdivide.PLATE]\nu = 3\nv = 2\n", "[material.alloy]\n");
    assert!(
        no_k.iter()
            .any(|t| t == "analysis.structure.material.alloy.thermal_conductivity"),
        "{no_k:?}"
    );
}

/// 設定の上限（fune-model）と解析の上限（sat-analysis）は同じ値（依存の向きで共有できないので、両方が見えるここで確かめる）。
#[test]
fn the_cell_limit_matches_the_analysis() {
    assert_eq!(ThermalSubdivideSettings::MAX_CELLS as usize, MAX_CELLS);
}
