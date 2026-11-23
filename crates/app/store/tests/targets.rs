//! 診断の対象（`fune_model::Target`）が、文字列を読み直さずに構造で付くこと。

use fune_model::path::{PortRef, SettingPath};
use fune_model::{Diagnostic, Target, check};
use fune_store::FileSet;
use fune_store::read::design_of;

fn diags(files: &[(&str, &str)]) -> Vec<Diagnostic> {
    let mut set = FileSet::new();
    set.insert("fune.toml", "[project]\nname = \"t\"\n");
    for (p, t) in files {
        set.insert(*p, *t);
    }
    check(&design_of(&set))
}

/// ファイル `file` の `code` の診断の対象。
fn target_of(d: &[Diagnostic], code: &str, file: &str) -> Option<Target> {
    d.iter()
        .find(|x| x.code == code && x.file.as_deref() == Some(file))
        .and_then(|x| x.target.clone())
}

#[test]
fn diagnostics_carry_typed_targets() {
    let d = diags(&[
        (
            "parts/aocs/RW.toml",
            "kind = \"reaction_wheel\"\n\n[ports.PWR]\nnet = \"NOPE\"\n",
        ),
        (
            "nets/P12V.toml",
            "kind = \"power\"\nsource = \"GHOST.OUT\"\nrail = { value = 12.0, unit = \"kg\" }\n",
        ),
        ("modes/SAFE.toml", "[states]\nGHOST = { state = \"on\" }\n"),
        ("cases/HOT.toml", "beta = { value = 1.0, unit = \"kg\" }\n"),
        ("analysis/power.toml", "initial_soc = 2.0\n"),
    ]);
    let port = PortRef {
        part: "RW".into(),
        port: "PWR".into(),
    };
    let part_file = "parts/aocs/RW.toml";
    assert_eq!(target_of(&d, "ref.net", part_file), Some(port.into()));
    // ネットの供給元の誤りは、参照を書いたネットのキーが対象（参照先の部品 ID を対象にしない）
    let source = Target::net("P12V", Some("source"));
    assert_eq!(
        target_of(&d, "ref.part", "nets/P12V.toml"),
        Some(source.clone())
    );
    assert_eq!(source.part(), None);
    let rail = target_of(&d, "unit.dimension", "nets/P12V.toml");
    assert_eq!(rail, Some(Target::net("P12V", Some("rail"))));
    let mode = target_of(&d, "ref.part", "modes/SAFE.toml");
    assert_eq!(mode, Some(Target::mode("SAFE", Some("GHOST"))));
    let beta = target_of(&d, "unit.dimension", "cases/HOT.toml");
    assert_eq!(
        beta.map(|t| t.to_string()).as_deref(),
        Some("cases.HOT.beta")
    );
    assert_eq!(
        target_of(&d, "setting.range", "analysis/power.toml"),
        Some(SettingPath::analysis(&["power", "initial_soc"]).into())
    );
}

/// 読み込みの診断（構文の誤り）には、ファイルが書いているものを対象として付ける。
#[test]
fn load_diagnostics_target_the_file_entity() {
    let d = diags(&[
        ("parts/aocs/BAD.toml", "kind = \n"),
        ("nets/BROKEN.toml", "kind = [\n"),
    ]);
    let of_file = |file: &str| {
        d.iter()
            .find(|x| x.file.as_deref() == Some(file))
            .and_then(|x| x.target.clone())
    };
    let bad = of_file("parts/aocs/BAD.toml");
    assert_eq!(bad, Some(Target::Part { id: "BAD".into() }));
    assert_eq!(bad.as_ref().and_then(Target::part), Some("BAD"));
    assert_eq!(
        of_file("nets/BROKEN.toml"),
        Some(Target::net("BROKEN", None))
    );
}
