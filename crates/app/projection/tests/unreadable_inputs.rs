//! 書かれているのに読めない入力は、黙って無いことにせず、対象を名指しした不足入力になる（#367）。
//! 書かれていない任意の入力は、これまでどおり無いまま（不足にしない）。

use fune_model::value::Quantity;
use fune_model::{Design, Resolved};
use fune_projection::{Context, ProjectionError, project_json};
use std::path::Path;

fn demo() -> Design {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../../fixtures/demo-6u");
    fune_store::load(&root)
        .expect("the demo should load")
        .design
}

/// 解析 `id` を射影して、不足入力のパスを返す（組み立てられれば空）。
fn missing_paths(design: &Design, id: &str) -> Vec<String> {
    let resolved: Resolved = fune_model::resolve(design);
    let ctx = Context::new(design, &resolved, None).unwrap();
    match project_json(id, &ctx) {
        Ok(_) => Vec::new(),
        Err(ProjectionError::Missing(m)) => m.into_vec().into_iter().map(|x| x.path).collect(),
        Err(e) => panic!("{id}: {e}"),
    }
}

/// 読めない単位と、正でない比（dB にできない）。
fn unreadable() -> [Quantity; 2] {
    [Quantity::new(0.5, "furlongs"), Quantity::new(0.0, "")]
}

#[test]
fn the_demo_projects_without_missing_inputs() {
    let design = demo();
    for id in [
        "comm.link_budget",
        "comm.data_throughput",
        "structure.static_load",
    ] {
        assert_eq!(missing_paths(&design, id), Vec::<String>::new(), "{id}");
    }
}

/// 任意の損失（衛星側・大気・偏波・ポインティング・実装）が読めなければ不足入力。
#[test]
fn an_unreadable_optional_loss_is_a_missing_input_naming_it() {
    for bad in unreadable() {
        for field in [
            "line_loss",
            "atmospheric_loss",
            "polarization_loss",
            "pointing_loss",
            "implementation_loss",
        ] {
            let mut design = demo();
            let link = design
                .analysis
                .comm
                .as_mut()
                .unwrap()
                .links
                .get_mut("s_down")
                .unwrap();
            let slot = match field {
                "line_loss" => &mut link.line_loss,
                "atmospheric_loss" => &mut link.atmospheric_loss,
                "polarization_loss" => &mut link.polarization_loss,
                "pointing_loss" => &mut link.pointing_loss,
                _ => &mut link.implementation_loss,
            };
            *slot = Some(bad.clone());
            let path = format!("analysis.comm.link.s_down.{field}");
            let missing = missing_paths(&design, "comm.link_budget");
            assert!(missing.contains(&path), "{path} with {bad:?}: {missing:?}");
        }
    }
}

#[test]
fn an_unreadable_ground_line_loss_is_a_missing_input_naming_it() {
    for bad in unreadable() {
        let mut design = demo();
        let ground = design.conditions.ground.get_mut("TOKYO_GS").unwrap();
        ground.links.get_mut("s_band").unwrap().line_loss = Some(bad.clone());
        let missing = missing_paths(&design, "comm.link_budget");
        assert!(
            missing
                .iter()
                .any(|p| p == "conditions.ground.TOKYO_GS.link.s_band.line_loss"),
            "{bad:?}: {missing:?}"
        );
    }
}

/// 損失を書かなければ（任意）、不足にならない。
#[test]
fn an_absent_optional_loss_stays_absent() {
    let mut design = demo();
    let link = design
        .analysis
        .comm
        .as_mut()
        .unwrap()
        .links
        .get_mut("s_down")
        .unwrap();
    link.pointing_loss = None;
    link.polarization_loss = None;
    assert!(missing_paths(&design, "comm.link_budget").is_empty());
}

/// 地上局の G/T が正でなければ、省略したことにせず不足入力にする。
#[test]
fn a_non_positive_g_over_t_is_a_missing_input() {
    let mut design = demo();
    let ground = design.conditions.ground.get_mut("TOKYO_GS").unwrap();
    ground.links.get_mut("s_band").unwrap().g_over_t = Some(Quantity::new(0.0, ""));
    let missing = missing_paths(&design, "comm.link_budget");
    assert!(
        missing
            .iter()
            .any(|p| p == "conditions.ground.TOKYO_GS.link.s_band.g_over_t"),
        "{missing:?}"
    );
}

/// データ量の解析: 下りの所要 Eb/N0 が読めなければ、局を黙って落とさず不足入力にする。
#[test]
fn an_unreadable_ebn0_required_is_a_missing_input_for_data_throughput() {
    let mut design = demo();
    let comm = design.analysis.comm.as_mut().unwrap();
    // デモは `data_downlink` を書かないので、送信量に数える下りは名前順の先頭（`s_down`）
    let name = "s_down";
    comm.links.get_mut(name).unwrap().ebn0_required = Some(Quantity::new(10.5, "furlongs"));
    let path = format!("analysis.comm.link.{name}.ebn0_required");
    let missing = missing_paths(&design, "comm.data_throughput");
    assert!(missing.contains(&path), "{missing:?}");
    let missing = missing_paths(&design, "comm.link_budget");
    assert!(missing.contains(&path), "{missing:?}");
}

/// 準静的荷重の倍数の単位が読めなければ、汎用の「入力を作れない」ではなく、書いた欄を名指しする。
#[test]
fn an_unreadable_load_factor_unit_names_the_field() {
    for axial in [true, false] {
        let mut design = demo();
        let launch = design.conditions.launch.as_mut().unwrap();
        let bad = Some(Quantity::new(6.0, "furlongs"));
        let path = if axial {
            launch.load_axial = bad;
            "conditions.launch.load_axial"
        } else {
            launch.load_lateral = bad;
            "conditions.launch.load_lateral"
        };
        let missing = missing_paths(&design, "structure.static_load");
        assert!(missing.iter().any(|p| p == path), "{path}: {missing:?}");
    }
}
