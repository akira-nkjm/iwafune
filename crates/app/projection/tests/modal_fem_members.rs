//! `structure.modal_fem` の射影（斜めのパネルとフレームの部材）: 小さな設計をファイルで組み、入力と解析の出力を確かめる。
//! 軸に沿わないパネルは姿勢つきでモデルになり、部材の断面を持つフレームは箱の辺の部材になって、フレームに付いた部品は部材につながる。

use fune_model::Design;
use fune_projection::{Context, ProjectionError, project_typed};
use fune_store::FileSet;
use fune_store::read::design_of;
use sat_analysis::spec::Analysis;
use sat_analysis::structure::ModalFem;
use sat_analysis::structure::modal_fem::{ModalFemAxis, ModalFemInput, ModalFemOutput};

const FUNE: &str = "[project]\nname = \"t\"\n[[catalog]]\nname = \"local\"\npath = \"catalog\"\n";
const MATERIAL: &str = "[material.al]\ndensity = { value = 2700.0, unit = \"kg/m^3\" }\nyoungs_modulus = { value = 70.0, unit = \"GPa\" }\npoisson_ratio = 0.3\n";
/// 0.2 × 0.3 m、厚さ 2 mm の板を原点に置く（法線は z）
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

/// 機体の軸に沿わないパネル（z まわりに 30° 回した板）は、姿勢つきの斜めのパネルとしてモデルにする（飛ばさない）。
/// 質量は板の `mass`（0.4 kg）で、ほかの板と節点を共有しなければ、つながりのない辺として出力に出る。
#[test]
fn a_skewed_panel_is_modelled_with_its_pose() {
    let skew = "kind = \"panel\"\n[pose]\nposition = { x = 0.6, y = 0.0, z = 0.0 }\norientation = { w = 0.9659258262890683, x = 0.0, y = 0.0, z = 0.25881904510252074 }\n[attr]\nsize = { x = 0.2, y = 0.3, z = 0.002, unit = \"m\" }\nthickness = { value = 0.002, unit = \"m\" }\nmaterial = \"al\"\nmass = { value = 0.4, unit = \"kg\" }\n";
    let d = design(&[
        ("parts/P.toml", PLATE),
        ("parts/S.toml", skew),
        ("conditions/launch.toml", CLAMP_X),
    ]);
    let i = input(&d);
    assert_eq!(i.panels.len(), 2);
    assert!(i.skipped_panels.is_empty() && i.lumped.is_empty());
    let s = i
        .panels
        .iter()
        .find(|p| p.id == "S")
        .expect("the skewed panel");
    assert_eq!(s.normal, ModalFemAxis::Z);
    assert_eq!(s.size.map(|x| x.value()), [0.2, 0.3]);
    assert_eq!(s.center.map(|x| x.value()), [0.6, 0.0, 0.0]);
    let q = s.orientation.expect("a pose");
    assert!(
        (q[0] - 0.9659258262890683).abs() < 1e-12 && (q[3] - 0.25881904510252074).abs() < 1e-12
    );
    // 軸に沿った板は姿勢を持たない
    assert!(
        i.panels
            .iter()
            .find(|p| p.id == "P")
            .is_some_and(|p| p.orientation.is_none())
    );
    let out = run(&i);
    assert!(out.skipped_panels.is_empty());
    assert_eq!(out.skewed_panels.len(), 1);
    assert_eq!(
        (
            out.skewed_panels[0].id.as_str(),
            out.skewed_panels[0].unjoined_edges
        ),
        ("S", 4)
    );
    assert!((out.total_mass.value() - (0.324 + 0.4)).abs() < 1e-9);
}

/// 部材の断面（`member_width`、あれば `member_wall`）を持つフレームは、箱・材料・断面・質量が入力の `frames` に入り、
/// 質量は全体に配らない（`distributed_mass` に入らない）。断面の無いフレームは今までどおり質量だけ。断面に必要な箱が無ければ足りない入力。
#[test]
fn a_frame_with_a_member_section_is_projected_with_its_box() {
    let framed = "kind = \"structure\"\n[attr]\nsize = { x = 0.1, y = 0.2, z = 0.3, unit = \"m\" }\nmaterial = \"al\"\nmember_width = { value = 6.0, unit = \"mm\" }\nmember_wall = { value = 1.0, unit = \"mm\" }\nmass = { value = 0.3, unit = \"kg\" }\n";
    let d = design(&[
        ("parts/P.toml", PLATE),
        ("parts/F.toml", framed),
        ("conditions/launch.toml", CLAMP_X),
    ]);
    let i = input(&d);
    assert_eq!(i.distributed_mass, None);
    let f = &i.frames[0];
    assert_eq!(f.id, "F");
    assert_eq!(f.size.map(|x| x.value()), [0.1, 0.2, 0.3]);
    assert_eq!(f.center.map(|x| x.value()), [0.0; 3]);
    assert_eq!(
        (f.youngs_modulus.value(), f.poisson_ratio, f.mass.value()),
        (70.0e9, 0.3, 0.3)
    );
    assert_eq!(f.section.width.value(), 0.006);
    assert_eq!(f.section.wall.map(|x| x.value()), Some(0.001));
    // 板の辺が箱の辺に乗らないので、部材は 1 本も無く、質量は全体に配る（質量は保たれる）
    let out = run(&i);
    assert_eq!(out.frames[0].members, 0);
    assert!((out.total_mass.value() - (0.324 + 0.3)).abs() < 1e-9);
    // 断面だけで箱が無い → 足りない入力
    let no_box = "kind = \"structure\"\n[attr]\nmaterial = \"al\"\nmember_width = { value = 6.0, unit = \"mm\" }\nmass = { value = 0.3, unit = \"kg\" }\n";
    let d = design(&[
        ("parts/P.toml", PLATE),
        ("parts/F.toml", no_box),
        ("conditions/launch.toml", CLAMP_X),
    ]);
    assert!(missing_paths(&d).iter().any(|p| p.contains("size")));
    // 肉厚だけで幅が無い → 足りない入力
    let wall_only = "kind = \"structure\"\n[attr]\nmember_wall = { value = 1.0, unit = \"mm\" }\nmass = { value = 0.3, unit = \"kg\" }\n";
    let d = design(&[
        ("parts/P.toml", PLATE),
        ("parts/F.toml", wall_only),
        ("conditions/launch.toml", CLAMP_X),
    ]);
    assert!(missing_paths(&d).iter().any(|p| p.contains("member_width")));
}

/// 箱形の構体（6 枚のパネル + 部材のあるフレーム）: フレームに付いた部品は、パネルでなくフレームの部材につながる
/// （出力の取り付けに辺が出る）。部品がパネルに載っていれば、そのパネル（フレームより先）。
#[test]
fn parts_on_a_frame_with_members_tie_to_its_edges() {
    let panel = |x: f64, y: f64, z: f64, size: [f64; 3]| {
        format!(
            "kind = \"panel\"\n[pose]\nposition = {{ x = {x}, y = {y}, z = {z} }}\n[attr]\nsize = {{ x = {}, y = {}, z = {}, unit = \"m\" }}\nthickness = {{ value = 0.0015, unit = \"m\" }}\nmaterial = \"al\"\n",
            size[0], size[1], size[2]
        )
    };
    let faces = [
        ("PX", panel(0.05, 0.0, 0.0, [0.0015, 0.2, 0.3])),
        ("NX", panel(-0.05, 0.0, 0.0, [0.0015, 0.2, 0.3])),
        ("PY", panel(0.0, 0.1, 0.0, [0.1, 0.0015, 0.3])),
        ("NY", panel(0.0, -0.1, 0.0, [0.1, 0.0015, 0.3])),
        ("PZ", panel(0.0, 0.0, 0.15, [0.1, 0.2, 0.0015])),
        ("NZ", panel(0.0, 0.0, -0.15, [0.1, 0.2, 0.0015])),
    ];
    let framed = "kind = \"structure\"\n[attr]\nsize = { x = 0.1, y = 0.2, z = 0.3, unit = \"m\" }\nmaterial = \"al\"\nmember_width = { value = 6.0, unit = \"mm\" }\nmass = { value = 0.3, unit = \"kg\" }\n";
    let on_frame = "[mount]\non = \"F\"\n[pose]\nposition = { x = 0.0, y = 0.06, z = 0.05 }\n[attr]\nmass = { value = 0.5, unit = \"kg\" }\n";
    let on_panel = "[mount]\non = \"PX\"\n[pose]\nposition = { x = 0.0, y = 0.0, z = 0.0 }\n[attr]\nmass = { value = 0.2, unit = \"kg\" }\n";
    let mut files: Vec<(String, String)> = faces
        .iter()
        .map(|(id, text)| (format!("parts/{id}.toml"), text.clone()))
        .collect();
    files.push(("parts/F.toml".to_owned(), framed.to_owned()));
    files.push(("parts/A.toml".to_owned(), on_frame.to_owned()));
    files.push(("parts/B.toml".to_owned(), on_panel.to_owned()));
    files.push(("conditions/launch.toml".to_owned(), CLAMP_X.to_owned()));
    let refs: Vec<(&str, &str)> = files
        .iter()
        .map(|(a, b)| (a.as_str(), b.as_str()))
        .collect();
    let i = input(&design(&refs));
    let (a, b) = (
        i.lumped.iter().find(|l| l.id == "A").expect("A"),
        i.lumped.iter().find(|l| l.id == "B").expect("B"),
    );
    assert_eq!((a.panel.as_deref(), a.frame.as_deref()), (None, Some("F")));
    assert_eq!((b.panel.as_deref(), b.frame.as_deref()), (Some("PX"), None));
    let out = run(&i);
    assert_eq!(
        (out.frames[0].edges, out.frames[0].unmodelled_edges.len()),
        (12, 0)
    );
    assert!(
        out.attachments
            .iter()
            .filter(|r| r.id == "A")
            .all(|r| r.panel == "F" && r.edge.is_some())
    );
    let b_rows: Vec<_> = out.attachments.iter().filter(|r| r.id == "B").collect();
    assert_eq!(
        (
            b_rows.len(),
            b_rows[0].panel.as_str(),
            b_rows[0].edge.as_deref()
        ),
        (1, "PX", None)
    );
    let panels = 2.0 * 2700.0 * 0.0015 * (0.2 * 0.3 + 0.1 * 0.3 + 0.1 * 0.2);
    assert!((out.total_mass.value() - (panels + 0.3 + 0.5 + 0.2)).abs() < 1e-9);
}
