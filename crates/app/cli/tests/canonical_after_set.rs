//! 「`fune set` などが書いたファイルを、`fune` 自身が拒まない」を確かめる（#504）。
//! 新しい空のプロジェクトとデモの一時コピーに、打った文字列の編集（整数の文字列・出典付きの最初の属性・カタログ・
//! 状態の行・消去）を順に当て、1 つごとに `fune fmt --check` が通り、`fune check` が TOML の書き方の指摘（`toml.*`）を
//! 出さないことを見る。

mod common;

use common::{demo_copy, fune, run_cli};
use std::path::Path;

/// 1 つの編集（`fune` の引数）。
type Step = &'static [&'static str];

/// 空のプロジェクトでの編集。設定の欄はまだ何も書かれていない（打った文字列から型を決める）。
const FRESH: &[Step] = &[
    &["set", "conditions.orbit.eccentricity", "0"],
    &["set", "analysis.aocs.srp_coefficient", "1"],
    &["set", "conditions.launch.safety_factor_yield", "2"],
    &["set", "analysis.aocs.drag_coefficient", "2"],
    &["set", "analysis.power.initial_soc", "1"],
    &["set", "analysis.power.orbits", "3"],
    &["set", "analysis.common.radiation.design_margin", "2"],
    &["set", "analysis.thermal.enclosure.emissivity", "1"],
    &[
        "setting",
        "set",
        "analysis.aocs.unload_dipole_fraction",
        "1",
    ],
    &["unset", "conditions.orbit.eccentricity"],
    &[
        "source",
        "set",
        "src1",
        "--kind",
        "datasheet",
        "--label",
        "DS",
    ],
    &["part", "add", "A", "--kind", "battery"],
    &["part", "add", "B", "--kind", "battery", "--label", "b"],
    &["part", "add", "C", "--kind", "battery"],
    // 属性のまだ無い部品への、出典付きの最初の属性（値の行の無い `[attr]` が残らない）
    &["set", "A.mass", "1 kg", "--source", "src1"],
    &[
        "set", "B.mass", "1 kg", "--source", "src1", "--note", "n", "--at", "p.1",
    ],
    &[
        "set", "C.mass", "1 kg", "--min", "0.9 kg", "--max", "1.1 kg",
    ],
    &["set", "C.capacity", "10 Wh", "--source", "src1"],
    &["set", "C.charge_efficiency", "1"],
    &[
        "set",
        "C.charge_efficiency",
        "1",
        "--min",
        "0",
        "--max",
        "1",
    ],
    &["set", "A.voltage", "8 V", "--source", "src1", "--note", "a"],
    &["set", "A.mass", "2 kg"],
    &["unset", "A.mass"],
    &["unset", "A.voltage"],
    &["unset", "B.mass"],
    &["unset", "C.mass"],
    &["unset", "C.capacity"],
];

/// デモでの編集。書かれていない欄と、書かれている欄、レコードの欄、カタログ。
const DEMO: &[Step] = &[
    &["set", "analysis.aocs.unload_dipole_fraction", "1"],
    &["set", "analysis.thermal.enclosure.emissivity", "1"],
    &["set", "cases.hot_imaging.orbits", "3"],
    &["set", "cases.hot_imaging.beta", "0 deg"],
    &["set", "maneuvers.RAISE.constraints.duty", "1"],
    &["set", "harness.H_OBC_CAM.cores", "4"],
    &["set", "harness.H_OBC_CAM.length", "1 m"],
    &["set", "phases.LEOP.order", "5"],
    &[
        "phase",
        "set-field",
        "NOMINAL_OPS",
        "orbit.eccentricity",
        "0",
    ],
    &["set", "RW_X.absorptivity", "1"],
    &["set", "RW_X.count", "2"],
    &[
        "set",
        "BAT.charge_efficiency",
        "1",
        "--min",
        "0",
        "--max",
        "1",
    ],
    &["set", "BAT.mass", "5 kg", "--source", "ds_vendor"],
    &[
        "catalog",
        "set",
        "local/bat_2s2p",
        "charge_efficiency",
        "1",
        "--min",
        "0",
        "--max",
        "1",
    ],
    &[
        "catalog",
        "set",
        "local/bat_2s2p",
        "mass",
        "5 kg",
        "--source",
        "ds_vendor",
    ],
    &["unset", "BAT.mass"],
    &["unset", "RW_X.absorptivity"],
];

/// 編集を当て、1 つごとに整形済みで、TOML の書き方の指摘が無いことを見る。守れなかった編集を返す。
fn failures(root: &Path, steps: &[Step]) -> Vec<String> {
    let mut out = Vec::new();
    for step in steps {
        run_cli(root, step);
        let fmt = fune()
            .arg("-C")
            .arg(root)
            .args(["fmt", "--check"])
            .output()
            .unwrap();
        if !fmt.status.success() {
            out.push(format!(
                "`fune {}` wrote a file that `fune fmt --check` rejects: {}",
                step.join(" "),
                String::from_utf8_lossy(&fmt.stdout).trim()
            ));
        }
        let check = fune()
            .arg("-C")
            .arg(root)
            .args(["--json", "check"])
            .output()
            .unwrap();
        let json: serde_json::Value = serde_json::from_slice(&check.stdout).unwrap();
        for d in json["diagnostics"].as_array().unwrap() {
            let code = d["code"].as_str().unwrap_or_default();
            if code.starts_with("toml.") {
                out.push(format!(
                    "`fune {}` wrote a file that `fune check` rejects ({code}): {}",
                    step.join(" "),
                    d["message"]
                ));
            }
        }
    }
    out
}

#[test]
fn edits_on_a_fresh_project_write_files_the_tool_accepts() {
    let tmp = tempfile::tempdir().unwrap();
    let root = tmp.path().join("sat");
    fune().arg("init").arg(&root).assert().success();
    let bad = failures(&root, FRESH);
    assert!(bad.is_empty(), "{}", bad.join("\n"));
}

#[test]
fn edits_on_the_demo_write_files_the_tool_accepts() {
    let (_tmp, root) = demo_copy();
    let bad = failures(&root, DEMO);
    assert!(bad.is_empty(), "{}", bad.join("\n"));
}
