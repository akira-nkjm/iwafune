//! KPI と check の対応（`rule::rule`）の検査。sat-analysis が check のキーを変えたら、KPI が黙って判定を失わずにここで落ちる。
//!
//! 各 KPI が名指しするキー（完全一致・接頭辞）が、その解析（の系）で出せることを確かめる。
//! デモで出る check はデモの実行で（動的なキーの接頭辞も含む）、デモが出さないキー（要求や上限を置かないと出ない判定、
//! 設計が持たない手段の判定、噴射中の収支）は、解析の系のソース（テストを除く）の文字列に現れることで確かめる。

use super::rule::{CheckMatch, rule};
use super::*;
use crate::Workspace;
use crate::dto::{CheckView, RunAnalysisRequest};
use serde_json::json;
use std::path::{Path, PathBuf};

fn analysis_src() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../lib/analysis/src")
}

/// 解析の系（`power.sizing` なら `power`）のソース（`<系>.rs` と `<系>/` の下。テストと `tests` の下は除く）の全文。
fn system_source(id: &str) -> String {
    fn walk(dir: &Path, out: &mut String) {
        for entry in std::fs::read_dir(dir).unwrap().flatten() {
            let path = entry.path();
            let name = entry.file_name().to_string_lossy().into_owned();
            if path.is_dir() {
                if name != "tests" {
                    walk(&path, out);
                }
            } else if name.ends_with(".rs") && name != "tests.rs" {
                out.push_str(&std::fs::read_to_string(&path).unwrap());
            }
        }
    }
    let system = id.split('.').next().unwrap();
    let mut out = std::fs::read_to_string(analysis_src().join(format!("{system}.rs"))).unwrap();
    walk(&analysis_src().join(system), &mut out);
    out
}

/// 文字列がソースに現れるか（キーは `"key"`、接頭辞は `"prefix…` の形の文字列の頭。
/// 種類の名前とコロンを別々に組み立てるキー（`"panel"` と `:{id}`）のために、コロンを除いた `"prefix"` も認める）。
fn in_source(source: &str, m: CheckMatch) -> bool {
    match m {
        CheckMatch::Exact(k) => source.contains(&format!("\"{k}\"")),
        CheckMatch::Prefix(p) => {
            source.contains(&format!("\"{p}"))
                || source.contains(&format!("\"{}\"", p.trim_end_matches(':')))
        }
        _ => unreachable!("named() は Exact と Prefix だけ"),
    }
}

fn emitted_by_demo(m: CheckMatch, keys: &[String]) -> bool {
    keys.iter().any(|k| m.matches(k))
}

#[test]
fn every_named_check_can_be_emitted_by_its_analysis() {
    let ws =
        Workspace::open(Path::new(env!("CARGO_MANIFEST_DIR")).join("../../../fixtures/demo-6u"))
            .unwrap();
    let mut checked = 0;
    for info in sat_analysis::catalog::catalog() {
        let source = system_source(&info.id);
        let result =
            crate::analysis::run_analysis(&ws, &info.id, &RunAnalysisRequest::default()).unwrap();
        let emitted: Vec<String> = result.checks.into_iter().map(|c| c.key).collect();
        for key in keys(&info.id) {
            for m in rule(*key).checks.named() {
                assert!(
                    emitted_by_demo(m, &emitted) || in_source(&source, m),
                    "{key:?}: 解析 {} に {m:?} の check が見つからない（キーが変わった？）",
                    info.id
                );
                checked += 1;
            }
        }
    }
    assert!(checked >= 28, "名指しの数が減っている: {checked}");
}

/// 検査の道具が効いていること: 存在しないキーは見つからない。
#[test]
fn the_guard_rejects_unknown_keys() {
    let source = system_source("power.sizing");
    assert!(in_source(&source, CheckMatch::Exact("sap_margin")));
    assert!(!in_source(&source, CheckMatch::Exact("sap_margins")));
    assert!(!emitted_by_demo(
        CheckMatch::Prefix("range:"),
        &["dod".to_owned()]
    ));
}

/// 判定の対象の KPI はキーを名指しするか、解析全体を代表する（`Any`）。`Any` は節点の動作範囲だけ。
#[test]
fn only_the_whole_analysis_kpi_uses_any() {
    for info in sat_analysis::catalog::catalog() {
        for key in keys(&info.id) {
            let any = rule(*key).checks == CheckMatch::Any;
            assert_eq!(any, *key == KpiKey::NodesInRange, "{key:?}");
        }
    }
}

fn checked(key: &str, verdict: Verdict) -> CheckView {
    CheckView {
        key: key.into(),
        label: String::new(),
        label_en: None,
        verdict,
        message: String::new(),
        message_en: None,
        at: "/checks/0".into(),
        scope: None,
    }
}

/// 出力が型に戻せなくても、KPI の並びと判定は check から決まる（値は無い）。
fn verdicts_of(id: &str, checks: &[CheckView]) -> Vec<(KpiKey, Option<Verdict>)> {
    kpis(id, Some(&json!({})), checks, KpiContext::default())
        .iter()
        .map(|k| (k.key, k.verdict))
        .collect()
}

/// 電力収支: DoD の上限だけ違反なら DoD の KPI だけが違反、周回の収支だけ違反ならその逆。
#[test]
fn energy_balance_kpis_follow_their_own_check() {
    use KpiKey::{MaxDod, OrbitBalance};
    use Verdict::{Ok, Violation};
    let dod = [checked("dod", Violation), checked("energy_balance", Ok)];
    assert_eq!(
        verdicts_of("power.energy_balance", &dod),
        [(OrbitBalance, Some(Ok)), (MaxDod, Some(Violation))]
    );
    let balance = [checked("dod", Ok), checked("energy_balance", Violation)];
    assert_eq!(
        verdicts_of("power.energy_balance", &balance),
        [(OrbitBalance, Some(Violation)), (MaxDod, Some(Ok))]
    );
    // 運用（ケース・モード）ごとに同じキーの check が並ぶので、KPI は最悪を取る
    let many = [
        checked("dod", Ok),
        checked("dod", Verdict::Warning),
        checked("energy_balance", Ok),
    ];
    assert_eq!(
        verdicts_of("power.energy_balance", &many),
        [(OrbitBalance, Some(Ok)), (MaxDod, Some(Verdict::Warning))]
    );
}

/// データ送信量: 1 日の送信可能量の判定と、滞留とストレージの判定は別の KPI に付く。
#[test]
fn data_throughput_kpis_follow_their_own_check() {
    use KpiKey::{DownlinkPerDay, MaxBacklog};
    use Verdict::{Ok, Violation};
    let capacity = [
        checked("downlinks_not_counted", Ok),
        checked("capacity", Violation),
    ];
    // ストレージの判定が無い（容量を置いていない）ときは、滞留の KPI は判定を持たない
    assert_eq!(
        verdicts_of("comm.data_throughput", &capacity),
        [(DownlinkPerDay, Some(Violation)), (MaxBacklog, None)]
    );
    let storage = [checked("capacity", Ok), checked("storage", Violation)];
    assert_eq!(
        verdicts_of("comm.data_throughput", &storage),
        [(DownlinkPerDay, Some(Ok)), (MaxBacklog, Some(Violation))]
    );
}

/// 熱の判定は節点と範囲の種類ごとの 1 つで、最低と最高の両側を見る（片側だけの判定は解析が出さない）。
/// 最高・最低の KPI は同じ判定の並びを使い、温度範囲でない判定には巻き込まれない。
#[test]
fn thermal_kpis_use_the_range_checks_only() {
    use KpiKey::{MaxTemperature, MinTemperature, NodesInRange};
    let checks = [
        checked("range:BAT:operating", Verdict::Warning),
        checked("other", Verdict::Violation),
    ];
    assert_eq!(
        verdicts_of("thermal.steady", &checks),
        [
            (MaxTemperature, Some(Verdict::Warning)),
            (MinTemperature, Some(Verdict::Warning)),
            // 解析全体を代表する KPI は全 check の最悪
            (NodesInRange, Some(Verdict::Violation)),
        ]
    );
}

/// 要素ごとの check（回線・センサ・取付）は接頭辞で束ねて最悪を取り、他の check には引きずられない。
#[test]
fn per_item_checks_are_gathered_by_prefix() {
    let links = [
        checked("margin:a@X", Verdict::Ok),
        checked("margin:b@X", Verdict::Violation),
    ];
    assert_eq!(
        verdicts_of("comm.link_budget", &links),
        [(KpiKey::LinkMargin, Some(Verdict::Violation))]
    );
    let unrelated = [checked("note", Verdict::Violation)];
    assert_eq!(
        verdicts_of("comm.link_budget", &unrelated),
        [(KpiKey::LinkMargin, None)]
    );
}
