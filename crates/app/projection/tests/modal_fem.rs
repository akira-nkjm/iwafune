//! `structure.modal_fem` の射影: 小さな設計をファイルで組み、手で組んだ解析の入力と比べる。
//! 板 1 枚は手で組んだ入力（第 2 段階の JSON の入力と同じ形）の結果と一致し、搭載部品は振動数を下げ、
//! 拘束が無ければ語彙つきで足りない入力になり、パネルに載らない部品は最も近いパネルにつなぐ。

use fune_model::Design;
use fune_model::design::launch::attr;
use fune_projection::{Context, ProjectionError, project_typed};
use fune_store::FileSet;
use fune_store::read::design_of;
use sat_analysis::spec::Analysis;
use sat_analysis::structure::ModalFem;
use sat_analysis::structure::modal_fem::{
    ModalFemAxis, ModalFemFace, ModalFemInput, ModalFemOutput, ModalFemPanel, ModalFemSupport,
};
use sat_units::typed::{Length, MassDensity, Pressure, Qty};

const FUNE: &str = "[project]\nname = \"t\"\n[[catalog]]\nname = \"local\"\npath = \"catalog\"\n";
const MATERIAL: &str = "[material.al]\ndensity = { value = 2700.0, unit = \"kg/m^3\" }\nyoungs_modulus = { value = 70.0, unit = \"GPa\" }\npoisson_ratio = 0.3\n";
/// 0.2 × 0.3 m、厚さ 2 mm の板を原点に置く（法線は z）。質量は密度 × 板厚 × 面積と同じ 0.324 kg にしない（`mass` を書かない）
const PLATE: &str = "kind = \"panel\"\n[attr]\nsize = { x = 0.2, y = 0.3, z = 0.002, unit = \"m\" }\nthickness = { value = 0.002, unit = \"m\" }\nmaterial = \"al\"\n";
const CLAMP_X: &str = "launch_constraint = \"base_fixed\"\nlaunch_constraint_face = \"-x\"\n";

fn design(files: &[(&str, &str)]) -> Design {
    let mut set = FileSet::new();
    set.insert("fune.toml", FUNE);
    set.insert("analysis/structure.toml", MATERIAL);
    for (path, text) in files {
        set.insert(*path, *text);
    }
    design_of(&set)
}

fn project(d: &Design) -> Result<ModalFemInput, ProjectionError> {
    let resolved = fune_model::resolve(d);
    let ctx = Context::bare(d, &resolved);
    project_typed::<ModalFem>(&ctx)
}

fn input(d: &Design) -> ModalFemInput {
    project(d).unwrap_or_else(|e| panic!("{e}"))
}

fn run(i: &ModalFemInput) -> ModalFemOutput {
    ModalFem::run(i).expect("the model solves")
}

fn missing_paths(d: &Design) -> Vec<String> {
    match project(d) {
        Err(ProjectionError::Missing(m)) => m.into_vec().into_iter().map(|x| x.path).collect(),
        other => panic!("expected missing inputs, got {:?}", other.map(|_| ())),
    }
}

/// 手で組んだ入力（板 1 枚。0.2 × 0.3 m、厚さ 2 mm、アルミ、x の負の面を固定）。
fn plate_by_hand() -> ModalFemInput {
    let m = Qty::<Length>::from_raw;
    ModalFemInput {
        panels: vec![ModalFemPanel {
            id: "P".to_owned(),
            center: [m(0.0), m(0.0), m(0.0)],
            normal: ModalFemAxis::Z,
            size: [m(0.2), m(0.3)],
            thickness: m(0.002),
            youngs_modulus: Qty::<Pressure>::from_raw(70.0e9),
            poisson_ratio: 0.3,
            density: Qty::<MassDensity>::from_raw(2700.0),
            mass: None,
        }],
        lumped: vec![],
        skipped_panels: vec![],
        distributed_mass: None,
        launch_axis: ModalFemAxis::Z,
        support: ModalFemSupport::BaseFixed {
            face: ModalFemFace::MinusX,
        },
        mesh_size: None,
        mode_count: None,
        merge_tolerance: None,
        drilling_factor: None,
    }
}

fn frequencies(o: &ModalFemOutput) -> Vec<f64> {
    o.modes.iter().map(|m| m.frequency.value()).collect()
}

/// 板 1 枚の設計の射影は、手で組んだ入力と同じ（JSON で全欄が一致）。解析の結果も、振動数まで同じ。
#[test]
fn one_plate_projects_to_the_hand_built_input() {
    let d = design(&[("parts/P.toml", PLATE), ("conditions/launch.toml", CLAMP_X)]);
    let projected = input(&d);
    let by_hand = plate_by_hand();
    assert_eq!(
        serde_json::to_value(&projected).unwrap(),
        serde_json::to_value(&by_hand).unwrap()
    );
    assert_eq!(frequencies(&run(&projected)), frequencies(&run(&by_hand)));
    // 質量は板の密度 × 板厚 × 面積 = 2700 × 0.002 × 0.06 = 0.324 kg（`mass` を書いていない）
    assert!((run(&projected).total_mass.value() - 0.324).abs() < 1e-9);
}

/// 板に載せた部品（取付先が板）は、板に剛につながって振動数を下げる。つなぎ先は名前で決まる（`automatic` が偽）。
#[test]
fn a_part_mounted_on_the_plate_lowers_the_frequency() {
    let m = "[mount]\non = \"P\"\n[pose]\nposition = { x = 0.1, y = 0.15, z = 0.0 }\n[attr]\nmass = { value = 0.2, unit = \"kg\" }\n";
    let bare = design(&[("parts/P.toml", PLATE), ("conditions/launch.toml", CLAMP_X)]);
    let loaded = design(&[
        ("parts/P.toml", PLATE),
        ("parts/M.toml", m),
        ("conditions/launch.toml", CLAMP_X),
    ]);
    let (a, b) = (run(&input(&bare)), run(&input(&loaded)));
    assert!(frequencies(&b)[0] < frequencies(&a)[0]);
    assert_eq!(b.attachments.len(), 1);
    assert_eq!(
        (
            b.attachments[0].id.as_str(),
            b.attachments[0].panel.as_str()
        ),
        ("M", "P")
    );
    assert!(!b.attachments[0].automatic);
    assert!((b.total_mass.value() - (0.324 + 0.2)).abs() < 1e-9);
}

/// 拘束が無ければ、`launch_constraint` が足りない入力になる（黙った既定値は置かない）。
#[test]
fn a_missing_constraint_is_a_missing_input() {
    let d = design(&[("parts/P.toml", PLATE)]);
    assert_eq!(
        missing_paths(&d),
        ["conditions.launch.launch_constraint".to_owned()]
    );
}

/// 面が要る拘束（`tabs`）に面が無ければ、面が足りない入力になる。`rails` は面が要らない。
#[test]
fn a_face_is_needed_only_by_constraints_that_use_one() {
    let tabs = design(&[
        ("parts/P.toml", PLATE),
        ("conditions/launch.toml", "launch_constraint = \"tabs\"\n"),
    ]);
    assert_eq!(
        missing_paths(&tabs),
        ["conditions.launch.launch_constraint_face".to_owned()]
    );
    let rails = design(&[
        ("parts/P.toml", PLATE),
        ("conditions/launch.toml", "launch_constraint = \"rails\"\n"),
    ]);
    assert!(matches!(input(&rails).support, ModalFemSupport::Rails));
}

/// 優先は `launch.toml` → ディスペンサー → 打ち上げ機（属性ごと）。品の列挙の値は属性の値から読む。
#[test]
fn the_constraint_priority_is_launch_toml_then_dispenser_then_launcher() {
    let disp = "kind = \"dispenser\"\n[attr]\nlaunch_constraint = \"rails\"\n";
    let lv = "kind = \"launcher\"\n[attr]\nlaunch_constraint = \"base_fixed\"\nlaunch_constraint_face = \"-z\"\n";
    let files = |launch: &'static str| {
        vec![
            ("parts/P.toml", PLATE),
            ("catalog/disp.toml", disp),
            ("catalog/lv.toml", lv),
            ("conditions/launch.toml", launch),
        ]
    };
    let both = "dispenser = \"local/disp\"\nlauncher = \"local/lv\"\n";
    assert!(matches!(
        input(&design(&files(both))).support,
        ModalFemSupport::Rails
    ));
    let only_lv = "launcher = \"local/lv\"\n";
    assert!(matches!(
        input(&design(&files(only_lv))).support,
        ModalFemSupport::BaseFixed {
            face: ModalFemFace::MinusZ
        }
    ));
    let written =
        "dispenser = \"local/disp\"\nlauncher = \"local/lv\"\nlaunch_constraint = \"free\"\n";
    assert!(matches!(
        input(&design(&files(written))).support,
        ModalFemSupport::Free
    ));
}

/// 面は、種類を出した出所だけから読む: ディスペンサーが `tabs` で面を書かず、打ち上げ機が `-z` を書いていても、
/// 打ち上げ機の面は混ぜず、面が足りない入力になる（出所の名前が理由に出る）。
#[test]
fn the_face_comes_only_from_the_source_of_the_kind() {
    let disp = "kind = \"dispenser\"\n[attr]\nlaunch_constraint = \"tabs\"\n";
    let lv = "kind = \"launcher\"\n[attr]\nlaunch_constraint = \"base_fixed\"\nlaunch_constraint_face = \"-z\"\n";
    let d = design(&[
        ("parts/P.toml", PLATE),
        ("catalog/disp.toml", disp),
        ("catalog/lv.toml", lv),
        (
            "conditions/launch.toml",
            "dispenser = \"local/disp\"\nlauncher = \"local/lv\"\n",
        ),
    ]);
    match project(&d) {
        Err(ProjectionError::Missing(m)) => {
            let list = m.into_vec();
            assert_eq!(list.len(), 1, "{list:?}");
            assert_eq!(list[0].path, "conditions.launch.launch_constraint_face");
            assert!(
                list[0].reason.contains("dispenser local/disp"),
                "{}",
                list[0].reason
            );
        }
        other => panic!("expected a missing face, got {:?}", other.map(|_| ())),
    }
}

/// `launch.toml` の種類と、そこに書いた面は、カタログの値より先に、組で効く。
#[test]
fn launch_toml_kind_and_its_own_face_win_over_the_catalog() {
    let lv = "kind = \"launcher\"\n[attr]\nlaunch_constraint = \"base_fixed\"\nlaunch_constraint_face = \"-z\"\n";
    let d = design(&[
        ("parts/P.toml", PLATE),
        ("catalog/lv.toml", lv),
        (
            "conditions/launch.toml",
            "launcher = \"local/lv\"\nlaunch_constraint = \"tabs\"\nlaunch_constraint_face = \"+x\"\n",
        ),
    ]);
    assert!(matches!(
        input(&d).support,
        ModalFemSupport::Tabs {
            face: ModalFemFace::PlusX
        }
    ));
    // launch.toml が種類だけを書けば、面はカタログから借りず（組は同じ出所）、面が必要な種類なら足りない入力
    let d = design(&[
        ("parts/P.toml", PLATE),
        ("catalog/lv.toml", lv),
        (
            "conditions/launch.toml",
            "launcher = \"local/lv\"\nlaunch_constraint = \"tabs\"\n",
        ),
    ]);
    assert_eq!(
        missing_paths(&d),
        ["conditions.launch.launch_constraint_face".to_owned()]
    );
}

/// 属性定義の語彙は、解析が意味を持つ綴りと同じ（拘束の種類・面）。
#[test]
fn the_attribute_vocabulary_matches_the_analysis() {
    let d = Design::default();
    let symbols = |id: &str| d.attribute(id).expect("defined").symbols.clone();
    assert_eq!(
        symbols(attr::CONSTRAINT),
        ModalFemSupport::LAUNCH_KINDS.map(str::to_owned)
    );
    assert_eq!(
        symbols(attr::CONSTRAINT_FACE),
        ModalFemFace::ALL.map(|f| f.as_str().to_owned())
    );
}

const FRAME: &str = "kind = \"structure\"\n[attr]\nmass = { value = 0.3, unit = \"kg\" }\n";
/// 2 枚目の板: 同じ大きさの板を x = 0.6 m に離して置く（板 P とは辺を共有しない。質量は `mass` で 0.25 kg）
const FAR_PLATE: &str = "kind = \"panel\"\n[pose]\nposition = { x = 0.6, y = 0.0, z = 0.0 }\n[attr]\nsize = { x = 0.2, y = 0.3, z = 0.002, unit = \"m\" }\nthickness = { value = 0.002, unit = \"m\" }\nmaterial = \"al\"\nmass = { value = 0.25, unit = \"kg\" }\n";

fn two_plates<'a>(extra: &[(&'a str, &'a str)]) -> Design {
    let mut files: Vec<(&'a str, &'a str)> = vec![
        ("parts/P.toml", PLATE),
        ("parts/Q.toml", FAR_PLATE),
        ("parts/F.toml", FRAME),
        ("conditions/launch.toml", CLAMP_X),
    ];
    files.extend_from_slice(extra);
    design(&files)
}

/// パネルに載らない部品（フレームに付いた部品・取付の無い部品）は最も近いパネルにつなぐ（出力に `automatic` で出る）。
/// フレームは質量だけを全体に配る（`distributed_mass`）。パネルの `mass` は質量行列だけの値になる。
#[test]
fn a_part_off_the_panels_ties_to_the_nearest_panel() {
    let near_q = "[mount]\non = \"F\"\n[pose]\nposition = { x = 0.55, y = 0.0, z = 0.01 }\n[attr]\nmass = { value = 0.1, unit = \"kg\" }\nsize = { x = 0.04, y = 0.04, z = 0.04, unit = \"m\" }\n";
    let d = two_plates(&[("parts/N.toml", near_q)]);
    let i = input(&d);
    assert_eq!(i.lumped.len(), 1);
    assert_eq!(i.lumped[0].panel, None);
    assert_eq!(
        i.lumped[0].footprint.map(|f| f.map(|x| x.value())),
        Some([0.04; 3])
    );
    assert_eq!(i.distributed_mass.map(|m| m.value()), Some(0.3));
    assert_eq!(i.panels[1].mass.map(|m| m.value()), Some(0.25));
    let out = run(&i);
    let a = &out.attachments[0];
    assert_eq!(
        (a.id.as_str(), a.panel.as_str(), a.automatic),
        ("N", "Q", true)
    );
    // 質量は成長余裕の無い素の質量の和: 板 P（密度 × 板厚 × 面積 0.324）+ 板 Q（mass 0.25）+ 部品 0.1 + フレーム 0.3
    assert!((out.total_mass.value() - (0.324 + 0.25 + 0.1 + 0.3)).abs() < 1e-9);
}

/// 設定 `[modal].tie` は、取付先も距離も上書きしてパネルを決める。パネルでないものを指せば足りない入力にする。
#[test]
fn the_tie_setting_overrides_the_attachment() {
    let n = "[mount]\non = \"F\"\n[pose]\nposition = { x = 0.55, y = 0.0, z = 0.01 }\n[attr]\nmass = { value = 0.1, unit = \"kg\" }\n";
    let tie = |panel: &str| format!("{MATERIAL}\n[modal.tie]\nN = \"{panel}\"\n");
    let ok = tie("P");
    let d = two_plates(&[("parts/N.toml", n), ("analysis/structure.toml", &ok)]);
    assert_eq!(input(&d).lumped[0].panel.as_deref(), Some("P"));
    let out = run(&input(&d));
    assert_eq!(out.attachments[0].panel, "P");
    assert!(!out.attachments[0].automatic);

    let bad = tie("F");
    let d = two_plates(&[("parts/N.toml", n), ("analysis/structure.toml", &bad)]);
    assert_eq!(
        missing_paths(&d),
        ["analysis.structure.modal.tie.N".to_owned()]
    );
}

/// 設定 `[modal]` の要素の大きさ・許容差が入力に入る（書かなければ解析の既定のまま）。
#[test]
fn the_modal_settings_are_read() {
    let d = two_plates(&[]);
    let i = input(&d);
    assert!(i.mesh_size.is_none() && i.merge_tolerance.is_none());
    let text = format!(
        "{MATERIAL}\n[modal]\nmesh_size = {{ value = 20.0, unit = \"mm\" }}\njoint_tolerance = {{ value = 2.0, unit = \"mm\" }}\n"
    );
    let d = two_plates(&[("analysis/structure.toml", &text)]);
    let i = input(&d);
    assert_eq!(i.mesh_size.map(|x| x.value()), Some(0.02));
    assert_eq!(i.merge_tolerance.map(|x| x.value()), Some(0.002));
    assert!((run(&i).model.mesh_size.value() - 0.02).abs() < 1e-12);
}

/// 機体の軸に沿わないパネル（z まわりに 30° 回した板）は、理由つきで飛ばし、質量は集中質量にする。
#[test]
fn a_skewed_panel_is_skipped_with_a_reason_and_keeps_its_mass() {
    let skew = "kind = \"panel\"\n[pose]\nposition = { x = 0.6, y = 0.0, z = 0.0 }\norientation = { w = 0.9659258262890683, x = 0.0, y = 0.0, z = 0.25881904510252074 }\n[attr]\nsize = { x = 0.2, y = 0.3, z = 0.002, unit = \"m\" }\nthickness = { value = 0.002, unit = \"m\" }\nmaterial = \"al\"\nmass = { value = 0.4, unit = \"kg\" }\n";
    let d = design(&[
        ("parts/P.toml", PLATE),
        ("parts/S.toml", skew),
        ("conditions/launch.toml", CLAMP_X),
    ]);
    let i = input(&d);
    assert_eq!(i.panels.len(), 1);
    assert_eq!(i.skipped_panels.len(), 1);
    assert_eq!(i.skipped_panels[0].id, "S");
    assert!(i.skipped_panels[0].reason.contains("not aligned"));
    assert_eq!(i.lumped[0].id, "S");
    assert_eq!(i.lumped[0].footprint, None);
    let out = run(&i);
    assert_eq!(out.skipped_panels.len(), 1);
    assert!((out.total_mass.value() - (0.324 + 0.4)).abs() < 1e-9);
}

/// 打ち上げの解析は打ち上げ条件の形態で部品を置く: 展開した姿勢（x = 1 m）の部品が、収納の形態では板の上に来る。
#[test]
fn parts_are_placed_in_the_launch_configuration() {
    let m = "[mount]\non = \"P\"\n[pose]\nposition = { x = 1.0, y = 0.0, z = 0.0 }\n[attr]\nmass = { value = 0.2, unit = \"kg\" }\n";
    let stowed = "[pose.M]\nposition = { x = 0.05, y = 0.0, z = 0.0 }\n";
    let launch = format!("{CLAMP_X}configuration = \"STOWED\"\n");
    let d = design(&[
        ("parts/P.toml", PLATE),
        ("parts/M.toml", m),
        ("configurations/STOWED.toml", stowed),
        ("conditions/launch.toml", &launch),
    ]);
    let resolved = fune_model::resolve(&d);
    let ctx = Context::new(&d, &resolved, None).unwrap();
    let i = project_typed::<ModalFem>(&ctx).unwrap_or_else(|e| panic!("{e}"));
    assert_eq!(i.lumped[0].position[0].value(), 0.05);
    // 形態を選ばない文脈（基準の配置）では展開した位置
    let base = input(&design(&[
        ("parts/P.toml", PLATE),
        ("parts/M.toml", m),
        ("conditions/launch.toml", CLAMP_X),
    ]));
    assert_eq!(base.lumped[0].position[0].value(), 1.0);
}

/// パネルが 1 枚も無い設計は、パネルの部品が足りない入力になる（計算の失敗にしない）。
#[test]
fn a_design_without_panels_misses_the_panel_part() {
    let d = design(&[("conditions/launch.toml", CLAMP_X)]);
    let paths = missing_paths(&d);
    assert!(paths.iter().any(|p| p.contains("panel")), "{paths:?}");
}
