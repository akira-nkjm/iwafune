//! 熱容量が無い節点の警告（`thermal.zero_capacity`）。質量 0 の部品が自分の節点になると、解析を回す前の `fune check` で分かる。
//! 判定は解析と同じ（`ThermalNode::lacks_heat_capacity`）。熱の設定が無い設計には出さない。

use fune_model::{Design, Diagnostic};
use fune_projection::thermal;
use fune_store::FileSet;
use fune_store::read::design_of;

fn part(mass: &str, extra: &str) -> String {
    format!(
        "{extra}[attr]\nmass = {{ value = {mass}, unit = \"kg\" }}\nspecific_heat = {{ value = 900.0, unit = \"J/(kg*K)\" }}\n"
    )
}

fn design(files: &[(&str, String)]) -> Design {
    let mut set = FileSet::new();
    set.insert("fune.toml", "[project]\nname = \"t\"\n");
    for (path, text) in files {
        set.insert(*path, text.as_str());
    }
    design_of(&set)
}

fn warnings(d: &Design) -> Vec<Diagnostic> {
    thermal::check(d, &fune_model::resolve(d))
}

const THERMAL: &str = "orbits = 3\n";

/// 質量 0 の部品 A は自分の節点（既定の 1 部品 1 節点）で熱容量が無い。質量のある B と、熱の設定が無い設計は出さない。
#[test]
fn a_zero_mass_part_that_is_its_own_node_is_warned() {
    let files = [
        ("parts/A.toml", part("0.0", "")),
        ("parts/B.toml", part("1.0", "")),
    ];
    let d = design(&[
        files[0].clone(),
        files[1].clone(),
        ("analysis/thermal.toml", THERMAL.to_owned()),
    ]);
    let w = warnings(&d);
    assert_eq!(w.len(), 1, "{w:?}");
    assert_eq!(w[0].code, "thermal.zero_capacity");
    assert_eq!(w[0].path().as_deref(), Some("A.mass"));
    assert!(
        w[0].message.contains("analysis.thermal.nodes"),
        "{}",
        w[0].message
    );
    // 熱の設定が無ければ警告しない
    assert!(warnings(&design(&files)).is_empty());
}

/// 質量が無い部品（解析では不足入力 `A.mass`）も、自分の節点に熱容量が無いので警告する。
/// 比熱が無い部品は原因が比熱なので、この警告は出さない（解析の不足入力 `A.specific_heat` が知らせる）。
#[test]
fn a_part_without_mass_is_warned_but_one_without_specific_heat_is_not() {
    let no_mass = design(&[
        (
            "parts/A.toml",
            "[attr]\nspecific_heat = { value = 900.0, unit = \"J/(kg*K)\" }\n".to_owned(),
        ),
        ("analysis/thermal.toml", THERMAL.to_owned()),
    ]);
    assert_eq!(warnings(&no_mass).len(), 1);
    let no_c = design(&[
        (
            "parts/A.toml",
            "[attr]\nmass = { value = 1.0, unit = \"kg\" }\n".to_owned(),
        ),
        ("analysis/thermal.toml", THERMAL.to_owned()),
    ]);
    assert!(warnings(&no_c).is_empty());
}

/// `[nodes]` で質量のある部品と同じ節点にまとめれば熱容量があり、警告しない（節点の熱容量は部品の和）。
/// まとめた節点の部品がどれも質量 0 なら、節点の側（`analysis.thermal.nodes.<節点>`）で警告する。
#[test]
fn lumping_with_a_part_that_has_mass_fixes_it() {
    let lumped = |nodes: &str| {
        design(&[
            ("parts/A.toml", part("0.0", "")),
            ("parts/B.toml", part("1.0", "")),
            ("parts/C.toml", part("0.0", "")),
            (
                "analysis/thermal.toml",
                format!("{THERMAL}\n[nodes]\n{nodes}"),
            ),
        ])
    };
    // A は B と同じ節点、C は自分の節点のまま
    let w = warnings(&lumped("N = [\"A\", \"B\"]\n"));
    assert_eq!(w.len(), 1, "{w:?}");
    assert_eq!(w[0].path().as_deref(), Some("C.mass"));
    let w = warnings(&lumped("N = [\"A\", \"C\"]\n"));
    assert_eq!(w.len(), 1, "{w:?}");
    assert_eq!(w[0].path().as_deref(), Some("analysis.thermal.nodes.N"));
}
