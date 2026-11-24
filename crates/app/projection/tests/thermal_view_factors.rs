//! 内部の放射を形態係数から出す設定（`[view_factors] emissivity`）の射影: 書かなければ入力は従来どおり（何も渡さない）、
//! 書けば放射率と部品の箱だけが解析へ渡る（計算は解析）。解析まで通して、板の間の放射が形態係数の値になること（手計算の式）も確かめる。

use fune_model::Design;
use fune_projection::{Context, MissingInputs, thermal};
use fune_store::FileSet;
use fune_store::read::design_of;
use sat_analysis::spec::Analysis;
use sat_analysis::thermal::network::ThermalCouplingSource;
use sat_analysis::thermal::{ThermalSteady, ThermalSteadyInput};

const ORBIT: &str = include_str!("../../../../fixtures/demo-6u/conditions/orbit.toml");
const AOCS: &str = "[law.NOM]\nlaw = \"sun_pointing\"\naxis = { x = 1.0, y = 0.0, z = 0.0 }\nsecondary = { x = 0.0, y = 0.0, z = 1.0 }\n";
const THERMAL: &str = "contact_conductance = { value = 0.5, unit = \"W/K\" }\n\
    radiative_coupling = { value = 0.002, unit = \"m^2\" }\n\
    orbits = 1\ntime_step = { value = 60.0, unit = \"s\" }\n\n[case.x]\npreset = \"hot\"\n";
const MASS: &str = "mass = { value = 1.0, unit = \"kg\" }\nspecific_heat = { value = 900.0, unit = \"J/(kg*K)\" }\n";
/// 一辺 1 m の容器（中空の殻とみなす）。宇宙へ放射する外面（熱の逃げ道）を 1 つ持つ
const FRAME: &str = "kind = \"structure\"\n\n[attr]\nsize = { x = 1.0, y = 1.0, z = 1.0, unit = \"m\" }\n\
    area = { value = 1.0, unit = \"m^2\" }\nnormal = { x = 0.0, y = 0.0, z = 1.0 }\nabsorptivity = 0.3\nemissivity = 0.8\n";
/// 0.1 m 角、厚さ 0.01 m の基板 2 枚（容器の中。隙間 5 mm で重ねる）
const LOW: &str = "kind = \"structure\"\n\n[mount]\non = \"FRAME\"\n\n[pose]\nposition = { x = 0.0, y = 0.0, z = 0.0 }\n\n[attr]\nsize = { x = 0.1, y = 0.1, z = 0.01, unit = \"m\" }\n";
const HIGH: &str = "kind = \"structure\"\n\n[mount]\non = \"FRAME\"\n\n[pose]\nposition = { x = 0.0, y = 0.0, z = 0.015 }\n\n[attr]\nsize = { x = 0.1, y = 0.1, z = 0.01, unit = \"m\" }\n";

fn input(thermal_extra: &str) -> Result<ThermalSteadyInput, MissingInputs> {
    let mut set = FileSet::new();
    set.insert("fune.toml", "[project]\nname = \"t\"\n");
    set.insert("conditions/orbit.toml", ORBIT);
    set.insert("analysis/aocs.toml", AOCS);
    set.insert(
        "analysis/thermal.toml",
        format!("{THERMAL}\n{thermal_extra}"),
    );
    for (id, text) in [("FRAME", FRAME), ("LOW", LOW), ("HIGH", HIGH)] {
        set.insert(format!("parts/{id}.toml"), format!("{text}{MASS}"));
    }
    let d: Design = design_of(&set);
    let r = fune_model::resolve(&d);
    thermal::steady(&Context::new(&d, &r, None).unwrap())
}

#[test]
fn without_the_setting_nothing_is_passed() {
    assert!(input("").unwrap().network.view_factors.is_none());
}

/// 放射率が無ければ（表だけ）渡さない。書けば放射率と寸法を持つ部品の箱が渡る。
#[test]
fn with_the_emissivity_the_boxes_are_passed() {
    assert!(
        input("[view_factors]\n")
            .unwrap()
            .network
            .view_factors
            .is_none()
    );
    let i = input("[view_factors]\nemissivity = 0.8\n").unwrap();
    let v = i.network.view_factors.as_ref().unwrap();
    assert_eq!(v.emissivity, 0.8);
    let mut ids: Vec<&str> = v.boxes.iter().map(|b| b.id.as_str()).collect();
    ids.sort_unstable();
    assert_eq!(ids, ["FRAME", "HIGH", "LOW"]);
}

/// 解析まで通すと、向き合う面の対 LOW–HIGH は取付の既定値（0.002 m²）でなく、形態係数の値（出どころ `view_factor`）になる。
/// 板の面 0.1 × 0.1 m、隙間 5 mm、ε = 0.8: 向き合う面の `A·F` は Howell C-11 の `F`（`sat_astro::view_factor::parallel_opposed`）から。
#[test]
fn the_analysis_uses_the_view_factor_value() {
    let i = input("[view_factors]\nemissivity = 0.8\n").unwrap();
    let out = ThermalSteady::run(&i).unwrap();
    let pair = out
        .couplings
        .radiative
        .iter()
        .find(|l| l.a == "HIGH" && l.b == "LOW")
        .unwrap();
    assert_eq!(pair.source, ThermalCouplingSource::ViewFactor);
    let a = 0.1 * 0.1;
    let g = a * sat_astro::view_factor::parallel_opposed(0.1, 0.1, 0.005);
    let rho = 1.0 - 0.8;
    let expected = 1.0 / (2.0 * rho / (0.8 * a) + 1.0 / g);
    assert!(
        (pair.exchange_area.value() / expected - 1.0).abs() < 1e-3,
        "{} {expected}",
        pair.exchange_area.value()
    );
}
