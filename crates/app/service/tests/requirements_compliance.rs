//! 要求適合表: デモの要求（fixtures/demo-6u/requirements）の判定と、確かめられない状態、ケースごとの集約。
use fune_service::Workspace;
use fune_service::dto::{ComplianceReport, ComplianceRow, NotVerifiedReason};
use fune_service::requirements::{compliance, fails};
use sat_analysis::spec::Verdict;
use std::path::Path;

fn demo_root() -> std::path::PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../../fixtures/demo-6u")
}

fn copy_dir(from: &Path, to: &Path) {
    std::fs::create_dir_all(to).unwrap();
    for entry in std::fs::read_dir(from).unwrap() {
        let entry = entry.unwrap();
        let target = to.join(entry.file_name());
        if entry.file_type().unwrap().is_dir() {
            copy_dir(&entry.path(), &target);
        } else {
            std::fs::copy(entry.path(), target).unwrap();
        }
    }
}

fn row<'a>(report: &'a ComplianceReport, id: &str) -> &'a ComplianceRow {
    report.rows.iter().find(|r| r.id == id).unwrap()
}

#[test]
fn the_demo_shows_every_verdict_and_every_unverified_state() {
    let report = compliance(&Workspace::open(demo_root()).unwrap()).unwrap();
    let verdict = |id: &str| row(&report, id).verdict;
    assert_eq!(verdict("R_MASS"), Some(Verdict::Ok));
    assert_eq!(verdict("R_DOD"), Some(Verdict::Ok));
    assert_eq!(verdict("R_LINK_S"), Some(Verdict::Ok));
    assert_eq!(verdict("R_LIFETIME"), Some(Verdict::Ok));
    assert_eq!(verdict("R_STT_SUN"), Some(Verdict::Warning));
    assert_eq!(verdict("R_BAT_TEMP"), Some(Verdict::Violation));
    assert_eq!(verdict("R_STORAGE"), Some(Verdict::Violation));
    // 基準を要求が持つ判定と、解析の入力を与える要求
    for id in ["R_POWER", "R_LINK_MARGIN", "R_PFD", "R_SLEW"] {
        assert_eq!(verdict(id), Some(Verdict::Ok), "{id}");
    }
    assert_eq!(
        row(&report, "R_MISSION").reason,
        Some(NotVerifiedReason::DesignInput)
    );
    // 最低固有振動数の要求は打ち上げ適合の判定（`frequency:<向き>`）を確かめ先にし、粗い有限要素のモードで判定される
    // （梁を持たないデモで確かめ先が `structure.modal_estimate` だった間は、判定が出ず「確かめていない」だった）
    for id in ["R_FREQ_AXIAL", "R_FREQ_LATERAL"] {
        assert_eq!(verdict(id), Some(Verdict::Ok), "{id}");
    }
    assert_eq!(
        row(&report, "R_DEPLOY").reason,
        Some(NotVerifiedReason::NoLink)
    );
    let thrust = row(&report, "R_THRUST_POWER");
    assert_eq!(thrust.reason, Some(NotVerifiedReason::MissingInputs));
    assert_eq!(thrust.detail.as_deref(), Some("maneuvers"));
    let s = &report.summary;
    assert_eq!(
        (s.total, s.ok, s.warning, s.violation, s.not_verified),
        (16, 10, 1, 2, 3)
    );
    assert!(fails(&report, false));
}

#[test]
fn quantified_cases_are_judged_one_by_one_and_the_worst_decides() {
    let report = compliance(&Workspace::open(demo_root()).unwrap()).unwrap();
    // 質量・データ量のバジェットは評価した合計を値として持つ
    assert_eq!(
        row(&report, "R_MASS").value.as_ref().unwrap().dimension,
        "Mass"
    );
    let storage = row(&report, "R_STORAGE");
    let by_case: Vec<(Option<&str>, Option<Verdict>)> = storage
        .cases
        .iter()
        .map(|c| (c.case.as_deref(), c.verdict))
        .collect();
    assert_eq!(
        by_case,
        [
            (Some("eclipse_worst"), Some(Verdict::Ok)),
            (Some("hot_imaging"), Some(Verdict::Violation))
        ]
    );
    assert!(storage.value.as_ref().unwrap().display.contains("GB"));
    // all_cases は cases/ のすべて（ID 順）
    let dod: Vec<&str> = row(&report, "R_DOD")
        .cases
        .iter()
        .filter_map(|c| c.case.as_deref())
        .collect();
    assert_eq!(dod, ["cold_safe", "eclipse_worst", "hot_imaging"]);
}

#[test]
fn keys_that_are_not_produced_and_unknown_targets_are_not_verified() {
    let tmp = tempfile::tempdir().unwrap();
    let root = tmp.path().join("demo-6u");
    copy_dir(&demo_root(), &root);
    let write = |name: &str, body: &str| {
        std::fs::write(root.join("requirements").join(format!("{name}.toml")), body).unwrap();
    };
    let head = "statement = \"s\"\nquantity = \"Mass\"\nsource = \"sys_judgement\"\n[constraint]\nkind = \"at_most\"\nvalue = { value = 1.0, unit = \"kg\" }\n";
    write(
        "X_KEY",
        &format!("{head}[verify]\nanalysis = \"power.energy_balance\"\ncheck = \"no_such_key\"\n"),
    );
    write(
        "X_CASE",
        &format!("{head}[quantify]\ncases = [\"nope\"]\n[verify]\nbudget = \"mass\"\n"),
    );
    write(
        "X_WHOLE",
        &format!("{head}[verify]\nanalysis = \"cdh.data_budget\"\n"),
    );
    write(
        "X_NONE",
        &format!("{head}[verify]\nanalysis = \"orbit.eclipse_season\"\n"),
    );
    write(
        "X_GONE",
        &format!("{head}[verify]\nanalysis = \"power.no_such\"\n"),
    );
    let report = compliance(&Workspace::open(&root).unwrap()).unwrap();
    let key = row(&report, "X_KEY");
    assert_eq!(key.reason, Some(NotVerifiedReason::KeyNotProduced));
    assert!(
        key.detail.as_deref().unwrap().contains("dod"),
        "{:?}",
        key.detail
    );
    assert_eq!(
        row(&report, "X_CASE").reason,
        Some(NotVerifiedReason::UnknownTarget)
    );
    // キーを書かなければ解析の判定全体（データ量は既定のケースで違反）
    assert_eq!(row(&report, "X_WHOLE").verdict, Some(Verdict::Violation));
    // 判定を出さない解析
    let none = row(&report, "X_NONE");
    assert_eq!(none.reason, Some(NotVerifiedReason::KeyNotProduced));
    assert_eq!(
        none.detail.as_deref(),
        Some("The analysis produced no checks")
    );
    assert_eq!(
        row(&report, "X_GONE").reason,
        Some(NotVerifiedReason::UnknownTarget)
    );
}

/// 同じ基準（質量の上限）に要求が複数あれば、バジェットは最も厳しい上限を使うが、各要求の行は**その要求自身の制約**で判定する。
/// 緩い要求は、厳しい要求が違反でも満たされる。
#[test]
fn each_row_is_judged_against_its_own_constraint_when_limits_overlap() {
    let tmp = tempfile::tempdir().unwrap();
    let root = tmp.path().join("demo-6u");
    copy_dir(&demo_root(), &root);
    let write = |name: &str, kg: f64| {
        let body = format!(
            "statement = \"s\"\nquantity = \"Mass\"\nsource = \"sys_judgement\"\n[constraint]\nkind = \"at_most\"\nvalue = {{ value = {kg}, unit = \"kg\" }}\n[verify]\nbudget = \"mass\"\n"
        );
        std::fs::write(root.join("requirements").join(format!("{name}.toml")), body).unwrap();
    };
    write("X_TIGHT", 1.0);
    write("X_LOOSE", 20.0);
    let ws = Workspace::open(&root).unwrap();
    // 質量のバジェットの上限は最も厳しい 1 kg
    let view = fune_service::budget::budget(
        &ws,
        fune_service::dto::BudgetKind::Mass,
        &fune_service::dto::BudgetRequest::default(),
    )
    .unwrap();
    assert_eq!(view.limit.as_ref().map(|q| q.si), Some(1.0));
    assert_eq!(view.limit_requirement.as_deref(), Some("X_TIGHT"));
    assert_eq!(view.verdict, Some(Verdict::Violation));
    // 適合表の行はそれぞれ自分の制約で判定する
    let report = compliance(&ws).unwrap();
    let verdict = |id: &str| row(&report, id).verdict;
    assert_eq!(verdict("X_TIGHT"), Some(Verdict::Violation));
    assert_eq!(verdict("R_MASS"), Some(Verdict::Ok));
    assert_eq!(verdict("X_LOOSE"), Some(Verdict::Ok));
}
