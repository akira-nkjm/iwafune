//! 小さなプロジェクトを組んで、検証の診断のコードと重さを確かめる。

use fune_model::{Diagnostic, Severity, check};
use fune_store::FileSet;
use fune_store::read::design_of;

const PROJECT: &str = "[project]\nname = \"t\"\n";

fn diags(files: &[(&str, &str)]) -> Vec<Diagnostic> {
    let mut set = FileSet::new();
    set.insert("fune.toml", PROJECT);
    for (p, t) in files {
        set.insert(*p, *t);
    }
    check(&design_of(&set))
}

fn has(d: &[Diagnostic], code: &str, severity: Severity) -> bool {
    d.iter().any(|x| x.code == code && x.severity == severity)
}

const RW: &str = "kind = \"reaction_wheel\"\n\n[attr]\nmass = { value = 0.15, unit = \"kg\" }\nmax_momentum = { value = 10.0, unit = \"mN*m*s\" }\nmax_torque = { value = 1.0, unit = \"mN*m\" }\nspin_axis = { x = 0.0, y = 0.0, z = 1.0 }\n\n[attr.power]\non = { value = 0.6, unit = \"W\" }\n";

#[test]
fn a_clean_part_has_no_errors() {
    let d = diags(&[("parts/aocs/RW.toml", RW)]);
    assert!(d.iter().all(|x| x.severity != Severity::Error), "{d:?}");
}

#[test]
fn units_and_value_shapes() {
    let bad = RW
        .replace("unit = \"kg\"", "unit = \"W\"")
        .replace("mN*m*s", "mN*m*sec")
        .replace("z = 1.0 }", "z = 2.0 }");
    let d = diags(&[("parts/aocs/RW.toml", &bad)]);
    assert!(has(&d, "unit.dimension", Severity::Error), "{d:?}");
    assert!(has(&d, "unit.parse", Severity::Error));
    assert!(has(&d, "vector.not_unit", Severity::Warning));
    let located = d
        .iter()
        .find(|x| x.code == "unit.dimension")
        .and_then(|x| x.line);
    assert_eq!(located, Some(4));
    let by_state = "kind = \"panel\"\n\n[attr.mass]\non = { value = 1.0, unit = \"kg\" }\n";
    assert!(has(
        &diags(&[("parts/P.toml", by_state)]),
        "value.axis_mismatch",
        Severity::Error
    ));
    let wrong_state = RW.replace("[attr.power]\non", "[attr.power]\nfast");
    assert!(has(
        &diags(&[("parts/RW.toml", &wrong_state)]),
        "state.unknown",
        Severity::Error
    ));
    let other_unit = RW.replace("unit = \"kg\"", "unit = \"lb\"");
    let d = diags(&[("parts/RW.toml", &other_unit)]);
    assert!(has(&d, "unit.parse", Severity::Error), "{d:?}");
    let not_listed = RW.replace(
        "value = 0.15, unit = \"kg\"",
        "value = 150000.0, unit = \"mg\"",
    );
    assert!(has(
        &diags(&[("parts/RW.toml", &not_listed)]),
        "unit.not_allowed",
        Severity::Warning
    ));
}

#[test]
fn missing_values_and_mode_rows_are_warnings() {
    let d = diags(&[
        ("parts/RW.toml", "kind = \"reaction_wheel\"\n"),
        ("modes/SAFE.toml", "[states]\n"),
    ]);
    assert!(has(&d, "attr.missing", Severity::Warning));
    assert!(has(&d, "mode.unassigned", Severity::Warning));
    assert!(d.iter().all(|x| x.severity != Severity::Error), "{d:?}");
}

#[test]
fn dangling_references_are_errors() {
    let part = "kind = \"panel\"\n\n[mount]\non = \"NOPE\"\n\n[ports.PWR]\nnet = \"P5V\"\n";
    let mode = "[states]\nGHOST = { state = \"on\" }\n";
    let net = "kind = \"power\"\nsource = \"NOPE.OUT\"\nrail = { value = 5.0, unit = \"A\" }\n";
    let d = diags(&[
        ("parts/P.toml", part),
        ("modes/M.toml", mode),
        ("nets/N.toml", net),
    ]);
    for code in ["ref.mount", "ref.net", "ref.part", "unit.dimension"] {
        assert!(has(&d, code, Severity::Error), "{code}: {d:?}");
    }
}

/// ポートの参照は読み込みで `PortRef` にする。`部品.ポート` の形でなければ、そのファイルの形の誤り（行付き）。
#[test]
fn port_refs_are_read_as_structure() {
    let harness = "from = \"PCDU.OUT3\"\nto = \"RW_X\"\n";
    let d = diags(&[("harness/H.toml", harness)]);
    let schema = d
        .iter()
        .find(|x| x.code == "toml.schema")
        .expect("schema error");
    assert_eq!(schema.line, Some(2));
    assert!(schema.message.contains("part.port"), "{schema:?}");
    let mut set = FileSet::new();
    set.insert("fune.toml", PROJECT);
    set.insert(
        "harness/H.toml",
        "from = \"PCDU.OUT3\"\nto = \"RW_X.PWR\"\n",
    );
    let design = design_of(&set);
    let h = &design.harnesses["H"];
    assert_eq!(
        (h.from.part.as_str(), h.from.port.as_str()),
        ("PCDU", "OUT3")
    );
    assert_eq!(h.to.to_string(), "RW_X.PWR");
}

#[test]
fn mount_cycles_and_syntax_errors() {
    let a = "kind = \"panel\"\n\n[mount]\non = \"B\"\n";
    let b = "kind = \"panel\"\n\n[mount]\non = \"A\"\n";
    assert!(has(
        &diags(&[("parts/A.toml", a), ("parts/B.toml", b)]),
        "mount.cycle",
        Severity::Error
    ));
    let d = diags(&[
        ("parts/A.toml", "kind = \n"),
        ("parts/B.toml", "kind = \"panel\"\ncolour = 1\n"),
    ]);
    assert!(has(&d, "toml.syntax", Severity::Error));
    let schema = d
        .iter()
        .find(|x| x.code == "toml.schema")
        .expect("schema error");
    assert_eq!(schema.line, Some(2));
}

#[test]
fn budget_over_allocation_is_a_warning() {
    // 上限は要求が持つ（`verify.budget = "mass"` の `at_most`）
    let limit = "statement = \"The mass shall not exceed 10 kg.\"\nquantity = \"Mass\"\n\n[constraint]\nkind = \"at_most\"\nvalue = { value = 10.0, unit = \"kg\" }\n\n[verify]\nbudget = \"mass\"\n";
    let alloc =
        "[mass]\nsystem_margin = 0.1\n\n[mass.alloc]\naocs = { value = 9.5, unit = \"kg\" }\n";
    let d = diags(&[
        ("budget/allocation.toml", alloc),
        ("requirements/R_MASS.toml", limit),
        ("parts/aocs/RW.toml", RW),
    ]);
    assert!(has(&d, "budget.over_allocated", Severity::Warning), "{d:?}");
    let wrong = alloc.replace("9.5, unit = \"kg\"", "9.5, unit = \"W\"");
    assert!(has(
        &diags(&[
            ("budget/allocation.toml", &wrong),
            ("requirements/R_MASS.toml", limit),
            ("parts/aocs/RW.toml", RW)
        ]),
        "unit.dimension",
        Severity::Error
    ));
}

#[test]
fn project_definitions_are_checked() {
    let good = "label = \"線密度\"\ntype = \"scalar\"\nunits = [\"g/m\", \"kg/m\"]\n";
    assert!(
        diags(&[("defs/attribute/lin.toml", good)])
            .iter()
            .all(|x| !x.is_error())
    );
    let mixed = "label = \"x\"\ntype = \"scalar\"\ndimension = \"Mass\"\nunits = [\"kg\", \"W\"]\n";
    assert!(has(
        &diags(&[("defs/attribute/x.toml", mixed)]),
        "def.unit",
        Severity::Error
    ));
    let shadow = "label = \"x\"\ntype = \"scalar\"\ndimension = \"Mass\"\n";
    assert!(has(
        &diags(&[("defs/attribute/mass.toml", shadow)]),
        "def.shadow_builtin",
        Severity::Error
    ));
}

#[test]
fn an_id_field_must_match_the_file_name() {
    let net = "id = \"OTHER\"\nkind = \"power\"\n";
    let d = diags(&[("nets/P5V.toml", net)]);
    let mismatch = d
        .iter()
        .find(|x| x.code == "id.mismatch")
        .expect("id.mismatch");
    assert_eq!(
        (mismatch.severity, mismatch.line),
        (Severity::Error, Some(1))
    );
    let same = diags(&[("nets/P5V.toml", "id = \"P5V\"\nkind = \"power\"\n")]);
    assert!(same.iter().all(|x| !x.is_error()), "{same:?}");
    let mut set = FileSet::new();
    set.insert("fune.toml", PROJECT);
    set.insert("nets/P5V.toml", net);
    assert_eq!(design_of(&set).nets["P5V"].id.as_str(), "P5V");
}

/// ID の一意性（大文字小文字を無視）と規則は、部品だけでなくケース・出典・ハーネスにも同じに当てる。
#[test]
fn ids_are_unique_ignoring_case_in_every_namespace() {
    let case = "mode = \"SAFE\"\n";
    let source = "kind = \"datasheet\"\n";
    let harness = "from = \"A.P\"\nto = \"B.P\"\n";
    for (a, b, text) in [
        ("cases/hot.toml", "cases/Hot.toml", case),
        ("sources/ds.toml", "sources/DS.toml", source),
        ("harness/H1.toml", "harness/h1.toml", harness),
        ("modes/SAFE.toml", "modes/safe.toml", "[states]\n"),
        ("nets/P5V.toml", "nets/p5v.toml", "kind = \"power\"\n"),
    ] {
        let d = diags(&[(a, text), (b, text)]);
        assert!(has(&d, "id.duplicate", Severity::Error), "{a}: {d:?}");
        let one = diags(&[(a, text)]);
        assert!(!has(&one, "id.duplicate", Severity::Error), "{a}: {one:?}");
    }
    let bad = diags(&[("cases/9hot.toml", case)]);
    assert!(has(&bad, "id.invalid", Severity::Error), "{bad:?}");
}

/// 手で書いた図の線の経路も、GUI と同じ規則（直交・端子の位置が 0 以上）で検査する。
#[test]
fn view_routes_are_orthogonal() {
    let good = "[edge]\n\"k\" = { points = [[0, 0], [0, 40], [30, 40]], source_offset = 0 }\n";
    let d = diags(&[("view/electrical.toml", good)]);
    assert!(d.iter().all(|x| !x.is_error()), "{d:?}");
    let diagonal = "[edge]\n\"k\" = { points = [[0, 0], [10, 10]] }\n";
    assert!(has(
        &diags(&[("view/electrical.toml", diagonal)]),
        "view.route",
        Severity::Error
    ));
    let one_point = "[edge]\n\"k\" = { points = [[0, 0]] }\n";
    assert!(has(
        &diags(&[("view/electrical.toml", one_point)]),
        "view.route",
        Severity::Error
    ));
    let negative = "[edge]\n\"k\" = { points = [[0, 0], [0, 10]], target_offset = -1 }\n";
    assert!(has(
        &diags(&[("view/electrical.toml", negative)]),
        "view.port_offset",
        Severity::Error
    ));
}

/// 設定の量の次元は表（`setting_dims`）で引く。配列の中（`[[sequence]]`）や名前付きの表（`[link.<回線>]`）も。
#[test]
fn setting_dimensions_come_from_the_table() {
    let power = "[[sequence]]\nmode = \"SAFE\"\nduration = { value = 10.0, unit = \"kg\" }\n";
    let d = diags(&[
        ("modes/SAFE.toml", "[states]\n"),
        ("analysis/power.toml", power),
    ]);
    let dim = d
        .iter()
        .find(|x| x.code == "unit.dimension")
        .expect("dimension");
    assert_eq!(
        dim.target.as_ref().map(ToString::to_string).as_deref(),
        Some("analysis.power.sequence.0.duration")
    );
    let ground = "latitude = { value = 35.0, unit = \"deg\" }\nlongitude = { value = 139.0, unit = \"m\" }\n\n[link.s]\ntx_power = { value = 1.0, unit = \"K\" }\n";
    let d = diags(&[("conditions/ground/GS.toml", ground)]);
    let paths: Vec<String> = d
        .iter()
        .filter(|x| x.code == "unit.dimension")
        .filter_map(|x| x.target.as_ref().map(ToString::to_string))
        .collect();
    assert_eq!(
        paths,
        [
            "conditions.ground.GS.link.s.tx_power",
            "conditions.ground.GS.longitude"
        ]
    );
    let material = "[material.al]\nthermal_conductivity = { value = 1.0, unit = \"W\" }\n";
    assert!(has(
        &diags(&[("analysis/structure.toml", material)]),
        "unit.dimension",
        Severity::Error
    ));
}

/// 最上位が系の ID でない組立品の置き場は許す（info で知らせるだけ。解析を持つ系と製品ツリーの最上位は別）。
#[test]
fn top_level_directories_that_are_not_systems_are_info_only() {
    for path in [
        "parts/aocs/RW.toml",
        "parts/RW.toml",
        "parts/power/spares/RW.toml",
    ] {
        let d = diags(&[(path, RW)]);
        assert!(
            !d.iter().any(|d| d.code == "part.system_dir"),
            "{path}: {d:?}"
        );
    }
    for path in [
        "parts/ADCS/RW.toml",
        "parts/mechanism/nested/RW.toml",
        "parts/harness/RW.toml",
    ] {
        let d = diags(&[(path, RW)]);
        let info = d.iter().find(|d| d.code == "part.system_dir").unwrap();
        assert_eq!(info.severity, Severity::Info, "{path}");
        for system in fune_model::design::AnalysisSystem::ALL {
            assert!(info.message.contains(system.as_str()), "{info:?}");
        }
        assert!(
            !d.iter().any(|d| d.severity == Severity::Error),
            "{path}: {d:?}"
        );
    }
}

#[test]
fn part_system_uses_location_then_role_then_none() {
    use fune_model::design::AnalysisSystem;
    let mut files = FileSet::new();
    files.insert("fune.toml", PROJECT);
    files.insert("parts/power/spares/WHEEL.toml", RW);
    files.insert(
        "defs/kind/custom_wheel.toml",
        "label = \"wheel\"\nextends = \"reaction_wheel\"\n",
    );
    files.insert(
        "parts/FREE_WHEEL.toml",
        RW.replace("reaction_wheel", "custom_wheel"),
    );
    files.insert("parts/UNASSIGNED.toml", "kind = \"component\"\n");
    files.insert("parts/payload/ASSEMBLY.toml", "kind = \"structure\"\n");
    files.insert("parts/payload/ASSEMBLY/CHILD.toml", RW);
    let design = design_of(&files);
    assert_eq!(design.system_of("WHEEL"), Some(AnalysisSystem::Power));
    assert_eq!(design.system_of("FREE_WHEEL"), Some(AnalysisSystem::Aocs));
    assert_eq!(design.system_of("UNASSIGNED"), None);
    assert_eq!(design.system_of("CHILD"), Some(AnalysisSystem::Payload));
    assert_eq!(design.system_of("MISSING"), None);
    let resolved = fune_model::resolve::resolve(&design);
    assert_eq!(resolved.parts["WHEEL"].system, design.system_of("WHEEL"));
}

#[test]
fn allocations_accept_systems_without_directories_and_parts_but_not_arbitrary_groups() {
    let d = diags(&[
        ("parts/RW.toml", RW),
        ("parts/aocs/spares/P.toml", "kind = \"component\"\n"),
        (
            "budget/allocation.toml",
            "[mass.alloc]\npayload = { value = 1.0, unit = \"kg\" }\nRW = { value = 0.2, unit = \"kg\" }\n",
        ),
    ]);
    assert!(!has(&d, "ref.node", Severity::Error), "{d:?}");
    assert!(has(&d, "budget.limit_unset", Severity::Warning));
    let bad = diags(&[
        ("parts/aocs/spares/RW.toml", RW),
        (
            "budget/allocation.toml",
            "[mass.alloc]\nspares = { value = 1.0, unit = \"kg\" }\n",
        ),
    ]);
    assert!(has(&bad, "ref.node", Severity::Error));
    assert!(!has(&d, "budget.over_allocated", Severity::Warning));
}

#[test]
fn removed_project_groups_are_rejected() {
    let mut files = FileSet::new();
    files.insert(
        "fune.toml",
        "[project]\nname = \"t\"\n[groups]\naocs = \"custom\"\n",
    );
    assert!(check(&design_of(&files)).iter().any(|d| d.is_error()));
}

/// 上限の未設定は、配分にその種類の設定があって上限が無いときだけ知らせる（バジェットを持たないプロジェクトでは出さない）。
#[test]
fn limit_unset_only_for_configured_budget_kinds() {
    let none = diags(&[("parts/RW.toml", RW)]);
    assert!(
        !has(&none, "budget.limit_unset", Severity::Warning),
        "{none:?}"
    );
    let mass_only = diags(&[
        ("parts/RW.toml", RW),
        ("budget/allocation.toml", "[mass]\nsystem_margin = 0.1\n"),
    ]);
    let unset: Vec<_> = mass_only
        .iter()
        .filter(|d| d.code == "budget.limit_unset")
        .collect();
    assert_eq!(unset.len(), 1, "{mass_only:?}");
    assert!(unset[0].message.contains("mass"));
}

/// 基準（上限）は要求が持つので、設定に旧い `limit` が書かれていれば移し先を知らせる（読まない。黙って残さない）。
#[test]
fn a_legacy_limit_in_the_allocation_is_reported_as_moved() {
    let d = diags(&[
        ("parts/RW.toml", RW),
        (
            "budget/allocation.toml",
            "[mass]\nlimit = { value = 10.0, unit = \"kg\" }\nsystem_margin = 0.1\n",
        ),
    ]);
    let moved: Vec<_> = d
        .iter()
        .filter(|x| x.code == "setting.moved_to_requirement")
        .collect();
    assert_eq!(moved.len(), 1, "{d:?}");
    assert!(moved[0].is_error());
    assert!(moved[0].message.contains("budget.mass.limit"));
    assert!(moved[0].message.contains("verify.budget"));
    // 旧い値は読まない: 上限を持つ要求が無いので上限は未設定のまま
    assert!(has(&d, "budget.limit_unset", Severity::Warning), "{d:?}");
}
