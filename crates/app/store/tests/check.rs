//! 小さなプロジェクトを組んで、検証の診断のコードと重さを確かめる。

use fune_model::{Diagnostic, Severity, check};
use fune_store::FileSet;
use fune_store::read::design_of;

const PROJECT: &str = "[project]\nname = \"t\"\n\n[groups]\nAOCS = \"姿勢\"\n";

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
    let d = diags(&[("parts/AOCS/RW.toml", RW)]);
    assert!(d.iter().all(|x| x.severity != Severity::Error), "{d:?}");
}

#[test]
fn units_and_value_shapes() {
    let bad = RW
        .replace("unit = \"kg\"", "unit = \"W\"")
        .replace("mN*m*s", "mN*m*sec")
        .replace("z = 1.0 }", "z = 2.0 }");
    let d = diags(&[("parts/AOCS/RW.toml", &bad)]);
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
    assert!(schema.message.contains("部品.ポート"), "{schema:?}");
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
    let alloc = "[mass]\nlimit = { value = 10.0, unit = \"kg\" }\nsystem_margin = 0.1\n\n[mass.alloc]\nAOCS = { value = 9.5, unit = \"kg\" }\n";
    let d = diags(&[
        ("budget/allocation.toml", alloc),
        ("parts/AOCS/RW.toml", RW),
    ]);
    assert!(has(&d, "budget.over_allocated", Severity::Warning), "{d:?}");
    let wrong = alloc.replace("9.5, unit = \"kg\"", "9.5, unit = \"W\"");
    assert!(has(
        &diags(&[
            ("budget/allocation.toml", &wrong),
            ("parts/AOCS/RW.toml", RW)
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
