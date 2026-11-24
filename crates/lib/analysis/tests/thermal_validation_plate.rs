//! 熱解析の検証（5）: 板を面内の格子の節点に分けたときの温度分布（`ThermalNetwork::plates`）。
//! 記号: k 熱伝導率 [W/(m·K)]、t 板厚、w 板の幅、Δ 升の寸法（板の長さ方向）、N 升の数、Q 流す熱 [W]。

mod common;

use common::*;
use sat_analysis::spec::{Analysis, Verdict};
use sat_analysis::structure::interference::InterferenceBox;
use sat_analysis::thermal::network::{
    ThermalMounts, ThermalNode, ThermalPlate, ThermalPlateAttachment,
};
use sat_analysis::thermal::steady::{ThermalSteadyCase, ThermalSteadyOutput};
use sat_analysis::thermal::{
    ThermalConductiveLink, ThermalMarginPolicy, ThermalNetwork, ThermalSteady, ThermalSteadyInput,
    ThermalTransient, ThermalTransientInput,
};
use sat_astro::math::{Quat, Vec3};
use sat_units::typed::Qty;

const K: f64 = 167.0; // アルミ合金の教科書的な値
const LEN: f64 = 0.4; // 板の長さ（x）
const WIDTH: f64 = 0.1; // 板の幅（y）
const THICK: f64 = 0.004; // 板厚（z。最も薄い軸 = 法線 → 面内は u = x, v = y）

fn bx(id: &str, centre: [f64; 3], size: [f64; 3]) -> InterferenceBox {
    InterferenceBox {
        id: id.to_owned(),
        count: 1,
        position: Vec3::new(centre[0], centre[1], centre[2]),
        orientation: Quat::IDENTITY,
        size: Vec3::new(size[0], size[1], size[2]),
        mount_ancestors: Vec::new(),
    }
}

fn plate(cells: [u32; 2], attachments: Vec<ThermalPlateAttachment>) -> ThermalPlate {
    ThermalPlate {
        part: "P".to_owned(),
        cells,
        conductivity: Qty::from_raw(K),
        frame: bx("P", [0.0; 3], [LEN, WIDTH, THICK]),
        attachments,
    }
}

/// 板の +z の面に載る小さい箱（x 方向の位置 `x`）。
fn on_top(node: &str, x: f64) -> ThermalPlateAttachment {
    ThermalPlateAttachment {
        node: node.to_owned(),
        footprint: bx(node, [x, 0.0, THICK / 2.0 + 0.01], [0.04, WIDTH, 0.02]),
    }
}

fn case(loads: Vec<sat_analysis::thermal::ThermalLoad>) -> ThermalSteadyCase {
    ThermalSteadyCase {
        id: "case".to_owned(),
        label: String::new(),
        environment: env(SOLAR, 0.0),
        exposure: fixed_sun(false),
        loads,
        mode_loads: None,
        heaters: Vec::new(),
    }
}

fn run(net: ThermalNetwork, c: ThermalSteadyCase) -> ThermalSteadyOutput {
    ThermalSteady::run(&ThermalSteadyInput {
        network: net,
        cases: vec![c],
        margin: ThermalMarginPolicy::default(),
    })
    .unwrap()
}

fn temp(out: &ThermalSteadyOutput, node: &str) -> f64 {
    out.cases[0]
        .nodes
        .iter()
        .find(|n| n.node == node)
        .unwrap_or_else(|| panic!("no node {node}"))
        .temperature
        .value()
}

/// 熱源 H（10 W）が板の左端の升に、300 K の温度一定の S が右端の升に載る 4 × 1 の板（放射なし）。
fn chain(ranges: bool) -> ThermalNetwork {
    let mut p = al_node("P", 0.1);
    if ranges {
        p.parts[0].ranges.operating = Some(sat_analysis::thermal::ThermalTempRange {
            min: Qty::from_raw(250.0),
            max: Qty::from_raw(320.0),
        });
    }
    ThermalNetwork {
        plates: vec![plate([4, 1], vec![on_top("H", -0.15), on_top("S", 0.15)])],
        mounts: Some(ThermalMounts {
            mounts: vec![["H".into(), "P".into()], ["S".into(), "P".into()]],
            conductance: Qty::from_raw(1000.0),
            exchange_area: None,
            contact: None,
        }),
        nodes: vec![
            p,
            al_node("H", 0.1),
            node_with("S", 0.1, CP_AL, Some(300.0)),
        ],
        conductive: Vec::new(),
        radiative: Vec::new(),
        enclosure: None,
        view_factors: None,
        surfaces: Vec::new(),
        shadowing: None,
    }
}

/// 手計算: 長さ L = 0.4 m を N = 4 升に分けると Δ = 0.1 m。隣り合う升の伝導は
/// G = k t w / Δ = 167 · 0.004 · 0.1 / 0.1 = 0.668 W/K。左端（H の下）から右端（S の下）の升まで 3 区間（中心間 3Δ = 0.3 m）を
/// 10 W が流れるので、ΔT = Q · 3Δ / (k t w) = 10 · 0.3 / (167 · 0.004 · 0.1) = 3 / 0.0668 = 44.91 K。
/// 取付は箱の真下の升だけに付く（取付の伝導 1000 W/K は他の升に配られない）ので、途中の升は直線的に下がる。
#[test]
fn steady_drop_along_the_plate_is_q_l_over_k_t_w() {
    let out = run(chain(false), case(vec![load("H", 10.0)]));
    let (t0, t3) = (temp(&out, "P[0,0]"), temp(&out, "P[3,0]"));
    let expected = 10.0 * 3.0 * 0.1 / (K * THICK * WIDTH);
    assert!((expected - 44.91).abs() < 0.01);
    assert!(
        (t0 - t3 - expected).abs() < 1e-6,
        "{} vs {expected}",
        t0 - t3
    );
    // 直線的: 各区間が Q / G = 14.97 K
    let step = 10.0 / (K * THICK * WIDTH / 0.1);
    for (a, b) in [
        ("P[0,0]", "P[1,0]"),
        ("P[1,0]", "P[2,0]"),
        ("P[2,0]", "P[3,0]"),
    ] {
        assert!((temp(&out, a) - temp(&out, b) - step).abs() < 1e-6);
    }
    // 右端の升の下は S（300 K）に 1000 W/K でつながる: 10 W で 0.01 K
    assert!((t3 - 300.0 - 10.0 / 1000.0).abs() < 1e-6);
}

/// 範囲の判定は部品の名前で 1 つ（key は `range_high:P:operating`）。上限は最も高い升（左端）、下限は最も低い升（右端）で
/// 判定し、判定文に升を書く。左端は 300 + 0.01 + 44.91 ≈ 345 K で上限 320 K を超えて違反、右端は 300.01 K で下限 250 K に余裕。
#[test]
fn range_checks_use_the_hottest_and_coldest_cells() {
    let out = run(chain(true), case(vec![load("H", 10.0)]));
    let high = out
        .checks
        .iter()
        .find(|c| c.key == "range_high:P:operating")
        .unwrap();
    let low = out
        .checks
        .iter()
        .find(|c| c.key == "range_low:P:operating")
        .unwrap();
    assert_eq!(high.verdict, Verdict::Violation);
    assert!(
        high.message.contains("hottest cell P[0,0]"),
        "{}",
        high.message
    );
    assert_eq!(low.verdict, Verdict::Ok);
    assert!(
        low.message.contains("coldest cell P[3,0]"),
        "{}",
        low.message
    );
    // 升ごとの key は出ない
    assert!(out.checks.iter().all(|c| !c.key.contains('[')));
    // 結果は升ごとに並び、板の部品と位置を持つ
    let cell = out.cases[0]
        .nodes
        .iter()
        .find(|n| n.node == "P[2,0]")
        .unwrap()
        .cell
        .clone()
        .unwrap();
    assert_eq!((cell.part.as_str(), cell.i, cell.j), ("P", 2, 0));
}

/// 外面（α 0.5・ε 0.8、0.04 m²）と内部発熱 5 W を持つ板（`cells` があれば分ける）と、板に明示の伝導 0.7 W/K で結ばれた箱 B。
fn heated_plate(cells: Option<[u32; 2]>) -> ThermalNetwork {
    ThermalNetwork {
        plates: cells
            .map(|c| vec![plate(c, Vec::new())])
            .unwrap_or_default(),
        mounts: None,
        nodes: vec![al_node("P", 0.1), al_node("B", 0.2)],
        conductive: vec![ThermalConductiveLink {
            a: "P".to_owned(),
            b: "B".to_owned(),
            conductance: Qty::from_raw(0.7),
        }],
        radiative: Vec::new(),
        enclosure: None,
        view_factors: None,
        surfaces: vec![
            surface("top", "P", 0.04, Vec3::X, 0.5, 0.8),
            surface("bottom", "P", 0.04, -Vec3::X, 0.5, 0.8),
        ],
        shadowing: None,
    }
}

fn sunlit() -> ThermalSteadyCase {
    let mut c = case(vec![load("P", 5.0), load("B", 2.0)]);
    c.exposure = fixed_sun(true);
    c
}

/// 一様な入熱・一様な放射・面積で配る結合だけなら、分けても温度は変わらない（どの升も分けない場合の温度に等しい）。
#[test]
fn a_uniform_case_is_not_changed_by_subdividing() {
    let whole = run(heated_plate(None), sunlit());
    for cells in [[3, 2], [8, 8], [1, 5]] {
        let split = run(heated_plate(Some(cells)), sunlit());
        for n in &split.cases[0].nodes {
            if n.cell.is_some() {
                let t = temp(&whole, "P");
                assert!(
                    (n.temperature.value() - t).abs() < 1e-6,
                    "{cells:?} {}",
                    n.node
                );
            }
        }
        assert!((temp(&split, "B") - temp(&whole, "B")).abs() < 1e-6);
    }
}

/// 1 × 1 に分けても、分けない場合と同じ結果（升は板の全体）。
#[test]
fn one_by_one_reproduces_the_undivided_result() {
    let whole = run(heated_plate(None), sunlit());
    let split = run(heated_plate(Some([1, 1])), sunlit());
    assert_eq!(temp(&split, "P[0,0]"), temp(&whole, "P"));
    assert_eq!(temp(&split, "B"), temp(&whole, "B"));
}

/// 熱量保存（非定常）: ∫(Q_in − Q_rad) dt = Σ C (T_end − T_0)。板（4 × 3 升）の左端の升に載る H に 10 W を入れ、
/// 板の外面（α 0.5・ε 0.8、0.04 m²）が升の面積の割合で太陽光を吸収・放射する。日照だけ（β 75°）600 s、標本 0.1 s（升の時定数が数秒なので、求積の誤差を許容より小さくする）、
/// 入熱も放射も滑らかなのでシンプソン則。許容は貯えた熱の 1e-9 倍（既存の `transient_conserves_energy` と同じ）。
#[test]
fn transient_conserves_energy_with_a_subdivided_plate() {
    let mut net = chain(false);
    net.plates[0].cells = [4, 3];
    net.nodes[2] = al_node("S", 0.1); // 温度一定の節点を外す（保存を数える）
    net.mounts.as_mut().unwrap().mounts.truncate(1);
    net.surfaces = vec![surface("top", "P", 0.04, Vec3::X, 0.5, 0.8)];
    // 14 節点の合計の熱容量が大きいので、積分の許容誤差（既定の絶対 1e-6 K が熱容量 180 J/K で 2e-4 J）を締めて、
    // 保存の誤差を積分器の許容ではなく求積と丸めだけにする
    let out = ThermalTransient::run(&ThermalTransientInput {
        network: net,
        timeline: timeline(600.0, 0.1, 75.0),
        environment: env(SOLAR, 0.0),
        phases: constant_phase(vec![load("H", 10.0)]),
        heaters: Vec::new(),
        initial: uniform(250.0),
        max_step: None,
        rel_tol: Some(1e-12),
        abs_tol: Some(Qty::from_raw(1e-10)),
        margin: ThermalMarginPolicy::default(),
    })
    .unwrap();
    let t = &out.series.t;
    let absorbed = values(&out.series, "absorbed");
    let q_in: Vec<f64> = absorbed.iter().map(|q| q + 10.0).collect();
    let cells: Vec<&str> = out
        .nodes
        .iter()
        .filter(|n| n.cell.is_some())
        .map(|n| n.node.as_str())
        .collect();
    assert_eq!(cells.len(), 12);
    // 放射: 各升の ε σ (A / 12) T⁴
    let mut q_rad = 0.0;
    let mut stored = 0.0;
    for id in &cells {
        let v = values(&out.series, id);
        let t4: Vec<f64> = v.iter().map(|x| x.powi(4)).collect();
        q_rad += 0.8 * sat_astro::constants::SIGMA_SB * (0.04 / 12.0) * simpson(t, &t4);
        stored += 0.1 / 12.0 * CP_AL * (v[v.len() - 1] - v[0]);
    }
    let h = values(&out.series, "H");
    stored += 0.1 * CP_AL * (h[h.len() - 1] - h[0]);
    let err = simpson(t, &q_in) - q_rad - stored;
    assert!(err.abs() < 1e-9 * stored.abs(), "err {err} of {stored}");
}

/// 板の部品の節点は 1 つの `ThermalNode`（`plates` で名指した節点）から作る。名指した節点が無い・まとめた節点・
/// 升の数が範囲外・板でない箱は入力の誤り。
#[test]
fn invalid_plates_are_refused() {
    let try_run = |f: &dyn Fn(&mut ThermalNetwork)| {
        let mut net = heated_plate(Some([2, 2]));
        f(&mut net);
        ThermalSteady::run(&ThermalSteadyInput {
            network: net,
            cases: vec![case(vec![])],
            margin: ThermalMarginPolicy::default(),
        })
        .unwrap_err()
        .to_string()
    };
    assert!(try_run(&|n| n.plates[0].part = "NOPE".into()).contains("no node of that name"));
    assert!(try_run(&|n| n.plates[0].cells = [9, 2]).contains("1 to 8 cells"));
    assert!(try_run(&|n| n.plates[0].cells = [2, 0]).contains("1 to 8 cells"));
    assert!(
        try_run(&|n| n.plates[0].frame.size = Vec3::new(0.2, 0.1, 0.05)).contains("not plate-like")
    );
    assert!(
        try_run(&|n| {
            let extra = n.nodes[0].parts[0].clone();
            n.nodes[0]
                .parts
                .push(sat_analysis::thermal::network::ThermalNodePart {
                    id: "Q".into(),
                    ..extra
                });
        })
        .contains("only a node of this one part")
    );
    let _: &ThermalNode = &heated_plate(None).nodes[0];
}

/// 刻みの警告: 4 × 1 の板の升は C = 0.1 kg · 900 / 4 = 22.5 J/K。端の升は面内の伝導 0.668 W/K 1 本と取付 1000 W/K
/// （H か S が付く升）なので、内側の升は 0.668 · 2 = 1.336 W/K で時定数 22.5 / 1.336 = 16.8 s（取付の付く端の升はもっと速い）。
/// 刻み 60 s の 10 % = 6 s を下回る升（取付の 1000 W/K が付く端の升: 22.5 / 1000 ≈ 0.02 s）があるので警告が出る。
/// 板に取付が無く内側の升だけなら 16.8 s > 6 s で警告は出ない。
#[test]
fn a_fast_cell_warns_about_the_step() {
    let run_with = |net: ThermalNetwork| {
        transient(
            net,
            timeline(600.0, 60.0, 75.0),
            env(SOLAR, 0.0),
            constant_phase(vec![load("H", 10.0)]),
            uniform(300.0),
        )
    };
    let out = run_with(chain(false));
    let w = out.checks.iter().find(|c| c.key == "plate_step:P").unwrap();
    assert_eq!(w.verdict, Verdict::Warning);
    assert!(w.message.contains("Use fewer cells"), "{}", w.message);
    let mut net = chain(false);
    net.mounts = None;
    net.plates[0].attachments.clear();
    net.conductive = vec![ThermalConductiveLink {
        a: "H".into(),
        b: "P".into(),
        conductance: Qty::from_raw(0.1),
    }];
    net.nodes[2] = al_node("S", 0.1);
    let out = run_with(net);
    assert!(out.checks.iter().all(|c| c.key != "plate_step:P"));
}
