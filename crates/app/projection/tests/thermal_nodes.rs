//! 熱の節点は既定で 1 部品 1 節点で、`[nodes]` は部品をまとめる上書き。小さな設計をファイルで組み、節点・熱容量・結合を手計算と比べ、
//! 解析にかけて閉じた 2 節点の緩和を閉じた形と比べる。

use fune_model::Design;
use fune_projection::{Context, MissingInputs, thermal};
use fune_store::FileSet;
use fune_store::read::design_of;
use sat_analysis::spec::{Analysis, AnalysisError};
use sat_analysis::thermal::steady::ThermalSteadyCase;
use sat_analysis::thermal::transient::ThermalInitial;
use sat_analysis::thermal::{
    ThermalExposure, ThermalLoad, ThermalSteady, ThermalSteadyInput, ThermalTransient,
    ThermalTransientInput,
};
use sat_units::typed::Qty;
use std::collections::BTreeMap;

/// 軌道はデモと同じもの（解析の時刻の並びに要る。この設計の熱には効かない: 外面も外部入熱も無い）。
const ORBIT: &str = include_str!("../../../../fixtures/demo-6u/conditions/orbit.toml");
/// 姿勢則は 1 つだけ書く（モードが無い解析が一意に選べるように）
const AOCS: &str = "[law.NOM]\nlaw = \"sun_pointing\"\naxis = { x = 1.0, y = 0.0, z = 0.0 }\nsecondary = { x = 0.0, y = 0.0, z = 1.0 }\n";
const THERMAL: &str = "contact_conductance = { value = 0.5, unit = \"W/K\" }\n\
    orbits = 1\ntime_step = { value = 60.0, unit = \"s\" }\n\n[case.x]\npreset = \"hot\"\n";

fn part(mass: &str, mount: Option<&str>) -> String {
    let mount = mount.map_or(String::new(), |m| format!("[mount]\non = \"{m}\"\n\n"));
    format!(
        "kind = \"structure\"\n\n{mount}[attr]\nmass = {mass}\nspecific_heat = {{ value = 900.0, unit = \"J/(kg*K)\" }}\n"
    )
}

/// A（1 kg）の上に B（0.5 kg）と C（0.5 kg）が付く設計。`nodes` は `[nodes]` の本文。
fn design(nodes: &str) -> Design {
    let kg = |v: f64| format!("{{ value = {v}, unit = \"kg\" }}");
    let mut set = FileSet::new();
    set.insert("fune.toml", "[project]\nname = \"t\"\n");
    set.insert("conditions/orbit.toml", ORBIT);
    set.insert("analysis/aocs.toml", AOCS);
    set.insert("analysis/thermal.toml", format!("{THERMAL}\n{nodes}"));
    set.insert("parts/A.toml", part(&kg(1.0), None));
    set.insert("parts/B.toml", part(&kg(0.5), Some("A")));
    set.insert("parts/C.toml", part(&kg(0.5), Some("A")));
    design_of(&set)
}

fn steady(d: &Design) -> Result<ThermalSteadyInput, MissingInputs> {
    let r = fune_model::resolve(d);
    thermal::steady(&Context::new(d, &r, None).unwrap())
}

fn transient(d: &Design) -> Result<ThermalTransientInput, MissingInputs> {
    let r = fune_model::resolve(d);
    thermal::transient(&Context::new(d, &r, None).unwrap())
}

fn capacities(input: &ThermalSteadyInput) -> Vec<(String, f64)> {
    let n = &input.network.nodes;
    n.iter()
        .map(|n| (n.id.clone(), n.heat_capacity()))
        .collect()
}

/// `[nodes]` が無ければ部品ごとに節点（名前は部品の ID）。熱容量 = 質量 × 比熱: A 1 × 900 = 900、B・C 0.5 × 900 = 450 J/K。
/// 取付 1 つにつき節点対の伝導が 1 つ（B–A、C–A。既定の 0.5 W/K）で、節点の組は取付の数と同じ。
#[test]
fn without_nodes_every_part_is_a_node() {
    let input = steady(&design("")).unwrap();
    assert_eq!(
        capacities(&input),
        [
            ("A".into(), 900.0),
            ("B".into(), 450.0),
            ("C".into(), 450.0)
        ]
    );
    assert!(input.network.nodes.iter().all(|n| n.per_part));
    let mounts = input.network.mounts.as_ref().unwrap();
    assert_eq!(mounts.mounts.len(), 2);
    let cond = mounts.links().unwrap().conductive;
    let pairs: Vec<_> = cond
        .iter()
        .map(|l| (l.a.as_str(), l.b.as_str(), l.conductance.value()))
        .collect();
    assert_eq!(pairs, [("A", "B", 0.5), ("A", "C", 0.5)]);
}

/// B と C を `N` にまとめると、節点は A（部品ごと）と N（450 + 450 = 900 J/K。まとめた）。B・C の取付先はどちらも A なので
/// 並列の伝導が足され、A–N の 1 つの結合は 2 × 0.5 = 1.0 W/K。N の中の取付は無い。
#[test]
fn lumped_and_per_part_nodes_mix_and_parallel_mounts_add() {
    let input = steady(&design("[nodes]\nN = [\"B\", \"C\"]\n")).unwrap();
    assert_eq!(
        capacities(&input),
        [("A".into(), 900.0), ("N".into(), 900.0)]
    );
    let flags: Vec<_> = input.network.nodes.iter().map(|n| n.per_part).collect();
    assert_eq!(flags, [true, false]);
    let cond = input
        .network
        .mounts
        .as_ref()
        .unwrap()
        .links()
        .unwrap()
        .conductive;
    assert_eq!(cond.len(), 1);
    assert_eq!(
        (
            cond[0].a.as_str(),
            cond[0].b.as_str(),
            cond[0].conductance.value()
        ),
        ("A", "N", 1.0)
    );
}

/// 3 部品を 1 つの節点にまとめると、熱容量は 900 + 450 + 450 = 1800 J/K で、節点の間の結合は無い（これまでと同じ）。
#[test]
fn everything_lumped_is_one_node_without_links() {
    let input = steady(&design("[nodes]\nALL = [\"A\", \"B\", \"C\"]\n")).unwrap();
    assert_eq!(capacities(&input), [("ALL".into(), 1800.0)]);
    assert!(!input.network.nodes[0].per_part);
    let links = input.network.mounts.as_ref().unwrap().links().unwrap();
    let (cond, rad) = (links.conductive, links.radiative);
    assert!(cond.is_empty() && rad.is_empty());
}

/// 名前のぶつかり: 節点 `C` が A だけをまとめ、部品 C が `[nodes]` に載っていなければ不足入力（`analysis.thermal.nodes.C`）。
/// 部品 C を別の節点に入れれば解消する。
#[test]
fn a_node_named_like_an_unlisted_part_stops_the_analysis() {
    let err = steady(&design("[nodes]\nC = [\"A\", \"B\"]\n")).unwrap_err();
    let targets: Vec<String> = err
        .into_vec()
        .iter()
        .filter_map(|m| m.target.as_ref().map(ToString::to_string))
        .collect();
    assert_eq!(targets, ["analysis.thermal.nodes.C"]);
    assert!(steady(&design("[nodes]\nC = [\"A\", \"B\", \"C\"]\n")).is_ok());
}

/// 質量が無い部品は、まとめた節点の部品と同じく不足入力（`D.mass`）。比熱を書かない部品は属性の既定値（900 J/(kg·K)）で熱容量を持つ
/// （E は 2 kg で 2 × 900 = 1800 J/K）。質量 0 の部品は熱容量が 0 になり、解析が入力の誤りにする（節点の名前を挙げる）。
#[test]
fn parts_without_mass_or_specific_heat() {
    let files = |parts: &[(&str, &str)]| {
        let mut set = FileSet::new();
        set.insert("fune.toml", "[project]\nname = \"t\"\n");
        set.insert("conditions/orbit.toml", ORBIT);
        set.insert("analysis/aocs.toml", AOCS);
        set.insert("analysis/thermal.toml", format!("{THERMAL}\n"));
        for (id, mass) in parts {
            let attr = mass.is_empty().then(String::new).unwrap_or_else(|| {
                format!("\n[attr]\nmass = {{ value = {mass}, unit = \"kg\" }}\n")
            });
            set.insert(
                format!("parts/{id}.toml"),
                format!("kind = \"structure\"\n{attr}"),
            );
        }
        design_of(&set)
    };
    let err = steady(&files(&[("D", ""), ("E", "2.0")])).unwrap_err();
    let targets: Vec<String> = err
        .into_vec()
        .iter()
        .filter_map(|m| m.target.as_ref().map(ToString::to_string))
        .collect();
    assert_eq!(targets, ["D.mass"]);
    let e_only = steady(&files(&[("E", "2.0")])).unwrap();
    assert_eq!(capacities(&e_only), [("E".into(), 1800.0)]);
    let with_zero = steady(&files(&[("E", "2.0"), ("Z", "0.0")])).unwrap();
    let Err(AnalysisError::InvalidInput(msg)) = ThermalSteady::run(&with_zero) else {
        panic!("a node without heat capacity must be refused");
    };
    assert!(msg.contains("Node Z"), "{msg}");
}

/// 閉じた 2 節点（A 900 J/K、B 450 J/K、間の伝導 G = 0.5 W/K、外面も外部入熱も無い）の緩和。
/// C_A dT_A/dt = G (T_B − T_A)、C_B dT_B/dt = −G (T_B − T_A) より、差 Δ = T_B − T_A は dΔ/dt = −Δ/τ、
/// τ = C_A C_B / ((C_A + C_B) G) = 900 · 450 / (1350 · 0.5) = 600 s。Δ(0) = 20 K なら Δ(t) = 20 e^(−t/600)。
/// 熱容量で重みを付けた平均 M = (C_A T_A + C_B T_B) / (C_A + C_B) は動かず、(900 · 300 + 450 · 320) / 1350 = 306.667 K。
/// したがって T_A = M − Δ · C_B/(C_A+C_B)、T_B = M + Δ · C_A/(C_A+C_B)。
#[test]
fn closed_pair_relaxes_with_the_hand_time_constant() {
    // A と B の 2 部品だけの設計（節点は 2 つ）
    let mut set = FileSet::new();
    set.insert("fune.toml", "[project]\nname = \"t\"\n");
    set.insert("conditions/orbit.toml", ORBIT);
    set.insert("analysis/aocs.toml", AOCS);
    set.insert("analysis/thermal.toml", format!("{THERMAL}\n"));
    set.insert("parts/A.toml", part("{ value = 1.0, unit = \"kg\" }", None));
    set.insert(
        "parts/B.toml",
        part("{ value = 0.5, unit = \"kg\" }", Some("A")),
    );
    let mut input = transient(&design_of(&set)).unwrap();
    assert_eq!(input.network.nodes.len(), 2);
    let temps: BTreeMap<String, Qty<sat_units::typed::Temperature>> = [
        ("A".to_owned(), Qty::from_raw(300.0)),
        ("B".to_owned(), Qty::from_raw(320.0)),
    ]
    .into();
    input.initial = ThermalInitial::PerNode {
        temperatures: temps,
        default: Qty::from_raw(300.0),
    };
    let out = ThermalTransient::run(&input).unwrap();
    let values = |key: &str| {
        out.series
            .channels
            .iter()
            .find(|c| c.key == key)
            .unwrap_or_else(|| panic!("no channel {key}"))
            .values
            .clone()
    };
    let (a, b) = (values("A"), values("B"));
    let (tau, mean) = (600.0_f64, (900.0 * 300.0 + 450.0 * 320.0) / 1350.0);
    assert!(out.series.t.len() > 50);
    for (i, t) in out.series.t.iter().enumerate() {
        let delta = 20.0 * (-t / tau).exp();
        let want_a = mean - delta * 450.0 / 1350.0;
        let want_b = mean + delta * 900.0 / 1350.0;
        assert!(
            (a[i] - want_a).abs() < 1e-4,
            "A at {t} s: {} vs {want_a}",
            a[i]
        );
        assert!(
            (b[i] - want_b).abs() < 1e-4,
            "B at {t} s: {} vs {want_b}",
            b[i]
        );
    }
}

/// 定常: A を 280 K の境界にし、B に 2 W を入れると、B–A の伝導だけが熱の逃げ道なので T_B = T_A + Q/G = 280 + 2/0.5 = 284 K。
#[test]
fn steady_state_of_the_pair_matches_the_hand_solution() {
    let mut set = FileSet::new();
    set.insert("fune.toml", "[project]\nname = \"t\"\n");
    set.insert("conditions/orbit.toml", ORBIT);
    set.insert("analysis/aocs.toml", AOCS);
    set.insert("analysis/thermal.toml", format!("{THERMAL}\n"));
    set.insert("parts/A.toml", part("{ value = 1.0, unit = \"kg\" }", None));
    set.insert(
        "parts/B.toml",
        part("{ value = 0.5, unit = \"kg\" }", Some("A")),
    );
    let mut input = steady(&design_of(&set)).unwrap();
    input.network.nodes[0].fixed_temperature = Some(Qty::from_raw(280.0));
    let ThermalSteadyCase { loads, .. } = &mut input.cases[0];
    loads.push(ThermalLoad {
        count: 1,
        node: "B".into(),
        power: Qty::from_raw(2.0),
        rf_output: None,
        heat_fraction: None,
    });
    assert!(matches!(
        input.cases[0].exposure,
        ThermalExposure::OrbitAverage { .. }
    ));
    let out = ThermalSteady::run(&input).unwrap();
    let t = |n: &str| {
        out.cases[0]
            .nodes
            .iter()
            .find(|x| x.node == n)
            .unwrap()
            .temperature
            .value()
    };
    assert!((t("A") - 280.0).abs() < 1e-9);
    assert!((t("B") - 284.0).abs() < 1e-6, "{}", t("B"));
}
