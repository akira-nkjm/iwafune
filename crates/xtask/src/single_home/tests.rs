use super::*;
use crate::scan::production_text;

#[test]
fn analysis_display_conversions_are_rejected_only_in_production() {
    for needle in ["/ 3600.0", "* 100.0", ".log10()"] {
        let rule = PROD_RULES.iter().find(|r| r.0 == needle).unwrap();
        let mut errors = Vec::new();
        check_rule(
            "crates/lib/analysis/src/check.rs",
            needle,
            rule,
            &mut errors,
        );
        assert_eq!(errors.len(), 1);
        errors.clear();
        check_rule("crates/lib/units/src/format.rs", needle, rule, &mut errors);
        assert!(errors.is_empty());
        let code = format!("fn run() {{}}\n#[cfg(test)]\nmod tests {{ {needle} }}");
        check_rule(
            "crates/lib/analysis/src/check.rs",
            &production_text("check.rs", &code).unwrap(),
            rule,
            &mut errors,
        );
        assert!(errors.is_empty());
    }
}

/// 列挙の綴り・値の文字列の書式は model の 1 か所だけ（サブモジュールの `value/text/write.rs` も許す）。
#[test]
fn spelling_and_text_format_have_one_home() {
    let rule = |needle: &str| {
        SHARED
            .iter()
            .flat_map(|rules| rules.iter())
            .find(|r| r.0 == needle)
            .unwrap()
    };
    let mut errors = Vec::new();
    let enum_str = rule("fn enum_str");
    check_rule(
        "crates/app/ops/src/cli/records.rs",
        "fn enum_str<T>",
        enum_str,
        &mut errors,
    );
    assert_eq!(errors.len(), 1);
    errors.clear();
    let to_text = rule("fn value_to_text");
    check_rule(
        "crates/app/model/src/value/text/write.rs",
        "fn value_to_text(",
        to_text,
        &mut errors,
    );
    assert!(errors.is_empty());
    check_rule(
        "crates/app/ops/src/cli/text.rs",
        "fn value_to_text(",
        to_text,
        &mut errors,
    );
    assert_eq!(errors.len(), 1);
}

/// 表示名と ID の組み方は model の `display_name` だけ。`label` の空を見ない ID への置き換えも書かない。
#[test]
fn display_name_rules_have_one_home() {
    let rule = |needle: &str| RUST_RULES.iter().find(|r| r.0 == needle).unwrap();
    let mut errors = Vec::new();
    let home = "crates/app/model/src/display_name.rs";
    check_rule(home, "fn name_or_id(", rule("fn name_or_id("), &mut errors);
    check_rule(
        home,
        "fn name_with_id(",
        rule("fn name_with_id("),
        &mut errors,
    );
    assert!(errors.is_empty());
    check_rule(
        "crates/app/service/src/report/common.rs",
        "fn name_with_id(",
        rule("fn name_with_id("),
        &mut errors,
    );
    assert_eq!(errors.len(), 1);
    errors.clear();
    let fallback = "label.clone().unwrap_or_else(|| ";
    check_rule(
        "crates/app/service/src/query/views.rs",
        fallback,
        rule(fallback),
        &mut errors,
    );
    assert_eq!(errors.len(), 1);
}

/// 読み込み中のスケルトンは kit/feedback/Loading の部品だけが ui/skeleton を使う。
#[test]
fn skeletons_come_from_kit_loading() {
    let rule = WEB_RULES.iter().find(|r| r.0 == "@/ui/skeleton").unwrap();
    let line = "import { Skeleton } from \"@/ui/skeleton\";";
    let mut errors = Vec::new();
    check_rule("web/src/kit/feedback/Loading.tsx", line, rule, &mut errors);
    assert!(errors.is_empty());
    check_rule("web/src/kit/metrics/KpiCard.tsx", line, rule, &mut errors);
    check_rule(
        "web/src/features/layout3d/LazyLayout3D.tsx",
        line,
        rule,
        &mut errors,
    );
    // 解析の結果の表示にも一時的な許可は無い
    check_rule("web/src/analysis/AnalysisCard.tsx", line, rule, &mut errors);
    check_rule(
        "web/src/features/systems/SystemTabs.tsx",
        line,
        rule,
        &mut errors,
    );
    assert_eq!(errors.len(), 4);
}

/// Web は系とバジェットの種類の一覧を API から取り、写しを書かない（テストの補助のデータは除く）。
#[test]
fn web_takes_systems_and_budget_kinds_from_the_api() {
    let systems = "export const systemOrder = [\"aocs\", \"cdh\", \"comm\"];";
    let old_systems = "export const systemOrder = [\"power\", \"aocs\", \"cdh\"];";
    let kinds = "const DELTA_KINDS = [\"mass\", \"power\", \"data\"];";
    for (needle, line) in [
        ("\"aocs\", \"cdh\"", systems),
        ("\"power\", \"aocs\"", old_systems),
        ("\"mass\", \"power\", \"data\"", kinds),
    ] {
        let rule = WEB_RULES.iter().find(|r| r.0 == needle).unwrap();
        let mut errors = Vec::new();
        check_rule(
            "web/src/features/systems/systems.ts",
            line,
            rule,
            &mut errors,
        );
        assert_eq!(errors.len(), 1, "{needle}");
        check_rule("web/src/test/fixture.ts", line, rule, &mut errors);
        assert_eq!(errors.len(), 1, "{needle}");
    }
}

/// バジェットの種類を JSX の属性に直書きしない（`<BudgetPanel kind="mass" />`）。
#[test]
fn web_does_not_write_budget_kinds_in_jsx() {
    for kind in ["mass", "power", "data"] {
        let needle = format!("kind=\"{kind}\"");
        let rule = WEB_RULES.iter().find(|r| r.0 == needle).unwrap();
        let mut errors = Vec::new();
        let line = format!("<BudgetPanel kind=\"{kind}\" />");
        check_rule(
            "web/src/features/overview/OverviewPage.tsx",
            &line,
            rule,
            &mut errors,
        );
        assert_eq!(errors.len(), 1, "{kind}");
        check_rule("web/src/test/fixture.ts", &line, rule, &mut errors);
        assert_eq!(errors.len(), 1, "{kind}");
    }
}

/// 系とバジェットの種類の一覧は model の列挙だけ。参照の所在は model の索引だけ（Rust のコードの規則）。
#[test]
fn lists_and_reference_sites_have_one_home() {
    let rust = |needle: &str| RUST_RULES.iter().find(|r| r.0 == needle).unwrap();
    let mut errors = Vec::new();
    let systems = rust("\"aocs\", \"cdh\"");
    check_rule(
        "crates/app/store/src/layout.rs",
        "&[\"aocs\", \"cdh\", \"comm\"]",
        systems,
        &mut errors,
    );
    assert_eq!(errors.len(), 1);
    errors.clear();
    check_rule(
        "crates/app/model/src/design/analysis.rs",
        "&[\"aocs\", \"cdh\"]",
        systems,
        &mut errors,
    );
    assert!(errors.is_empty());
    let site = PROD_RULES
        .iter()
        .find(|r| r.0 == "wire.as_deref() == Some(")
        .unwrap();
    check_rule(
        "crates/app/store/src/write/refs.rs",
        "h.wire.as_deref() == Some(from)",
        site,
        &mut errors,
    );
    assert_eq!(errors.len(), 1);
}

/// 役割は Role、慣例の停止状態は STATE_OFF を使う。停止状態のテストデータは許す。
#[test]
fn roles_and_off_state_have_one_home() {
    for needle in [
        "by_role(ctx, \"",
        "any_of_roles(&[\"",
        "SENSOR_ROLES: &[&str]",
    ] {
        let rule = RUST_RULES.iter().find(|r| r.0 == needle).unwrap();
        let mut errors = Vec::new();
        check_rule(
            "crates/app/projection/src/aocs.rs",
            needle,
            rule,
            &mut errors,
        );
        assert_eq!(errors.len(), 1, "{needle}");
    }
    let rule = PROD_RULES.iter().find(|r| r.0 == "\"off\"").unwrap();
    let mut errors = Vec::new();
    check_rule(
        "crates/app/ops/src/plan/parts.rs",
        "Some(\"off\")",
        rule,
        &mut errors,
    );
    assert_eq!(errors.len(), 1);
    errors.clear();
    check_rule("crates/app/model/src/role.rs", "\"off\"", rule, &mut errors);
    assert!(errors.is_empty());
    let text = "fn run() {}\n#[cfg(test)]\nmod tests { let state = \"off\"; }";
    check_rule(
        "crates/app/ops/src/plan/parts.rs",
        &production_text("parts.rs", text).unwrap(),
        rule,
        &mut errors,
    );
    assert!(errors.is_empty());
    assert!(production_text("crates/app/ops/tests/ops.rs", "\"off\"").is_none());
    assert!(
        production_text(
            "crates/app/cli/src/cmd/roundtrip/samples/parts.rs",
            "\"off\""
        )
        .is_none()
    );
}

/// 読み直し中に中身を薄くする class（`opacity-55`）は kit/feedback だけが持つ。
#[test]
fn refresh_dimming_comes_from_kit_feedback() {
    let rule = WEB_RULES.iter().find(|r| r.0 == "opacity-55").unwrap();
    let line = "item.updating && \"opacity-55\",";
    let mut errors = Vec::new();
    check_rule(
        "web/src/kit/feedback/Refreshable.tsx",
        line,
        rule,
        &mut errors,
    );
    assert!(errors.is_empty());
    check_rule(
        "web/src/analysis/AnalysisTable.tsx",
        line,
        rule,
        &mut errors,
    );
    assert_eq!(errors.len(), 1);
}

/// 待ちを示す動くロゴ（`MarkLoader`・`mark-glow`）は kit/feedback の部品だけが使い、画面は `Working` などを通す。
#[test]
fn animated_logo_comes_from_kit_feedback() {
    let mut errors = Vec::new();
    for needle in ["MarkLoader", "mark-glow"] {
        let rule = WEB_RULES.iter().find(|r| r.0 == needle).unwrap();
        let line = format!("<{needle} size={{44}} />");
        check_rule("web/src/kit/feedback/Working.tsx", &line, rule, &mut errors);
        assert!(errors.is_empty());
        check_rule(
            "web/src/features/diagram/DiagramPage.tsx",
            &line,
            rule,
            &mut errors,
        );
        check_rule(
            "web/src/shell/brand/BrandMark.tsx",
            &line,
            rule,
            &mut errors,
        );
        assert_eq!(errors.len(), 2);
        errors.clear();
    }
}

/// 本番の規則を `rel` のファイルの本文 `text` に当てた違反の数（テストのファイルは見ない）。
fn prod_violations(rel: &str, text: &str) -> usize {
    let Some(prod) = production_text(rel, text) else {
        return 0;
    };
    let mut errors = Vec::new();
    for rule in PROD_RULES {
        check_rule(rel, &prod, rule, &mut errors);
    }
    errors.len()
}

/// 推薬の流量の手書き（`F / (Isp · g₀)`）は rocket.rs だけが持ち、ほかは `mass_flow_kg_s` を通す。
#[test]
fn propellant_mass_flow_has_one_home() {
    let by_hand = "flow += tt.thrust / (tt.isp * G0);";
    assert_eq!(
        prod_violations("crates/lib/analysis/src/aocs/unload_time/rcs.rs", by_hand),
        1
    );
    assert_eq!(
        prod_violations("crates/lib/analysis/src/x.rs", "f / exhaust_velocity(isp)"),
        1
    );
    assert_eq!(
        prod_violations("crates/lib/astro/src/rocket.rs", by_hand),
        0
    );
    // 重量（質量 × g₀）は流量ではない
    assert_eq!(
        prod_violations("crates/lib/analysis/src/x.rs", "mass * G0 * n_ax"),
        0
    );
}

/// `/ 3600.0`・`* 100.0` は sat-units（と明示した例外）だけ。app や astro の全体を許さない。
#[test]
fn unit_display_conversions_are_not_allowed_across_app() {
    for needle in ["x / 3600.0", "r * 100.0"] {
        assert_eq!(prod_violations("crates/app/service/src/x.rs", needle), 1);
        assert_eq!(prod_violations("crates/lib/astro/src/x.rs", needle), 1);
        assert_eq!(prod_violations("crates/lib/units/src/x.rs", needle), 0);
    }
}

/// Web の規則: パルス・バジェットの種類の引数・`queryKey: [` は決めたファイルだけ（テストは除く）。
#[test]
fn web_pulse_budget_literals_and_query_keys_have_homes() {
    assert_eq!(
        prod_violations("web/src/features/x/X.tsx", "animate-pulse"),
        1
    );
    assert_eq!(
        prod_violations("web/src/ui/skeleton.tsx", "animate-pulse"),
        0
    );
    for call in [
        "s.budget(\"mass\")",
        "s.budget('power')",
        "budget(\"data\")",
    ] {
        assert_eq!(
            prod_violations("web/src/features/x/X.tsx", call),
            1,
            "{call}"
        );
        assert_eq!(
            prod_violations("web/src/features/overview/cards.ts", call),
            0
        );
        assert_eq!(prod_violations("web/src/api/keys.test.ts", call), 0);
    }
    assert_eq!(
        prod_violations("web/src/features/x/X.tsx", "budget(kind)"),
        0
    );
    let key = "useQuery({ queryKey: [\"a\"] })";
    assert_eq!(prod_violations("web/src/shell/Frame.tsx", key), 1);
    for ok in [
        "web/src/api/keys.ts",
        "web/src/quantity/format-batch.ts",
        "web/src/kit/feedback/Settled.tsx",
        "web/src/shell/Frame.test.tsx",
    ] {
        assert_eq!(prod_violations(ok, key), 0, "{ok}");
    }
}

/// 定義の書き出しの規則は、許したパスに 1 つも現れなければ「死んだ規則」になる。
#[test]
fn rules_matching_nothing_are_reported() {
    let rule_sources: Vec<(String, String)> = SHARED
        .iter()
        .flat_map(|rules| rules.iter())
        .chain(RUST_RULES)
        .chain(WEB_RULES)
        .chain(PROD_RULES)
        .filter(|(_, allowed, _)| !allowed.is_empty())
        .map(|(needle, allowed, _)| (allowed[0].to_string(), needle.to_string()))
        .collect();
    assert!(dead_rules(&rule_sources).is_empty());
    let dead = dead_rules(&[]);
    assert!(dead.iter().any(|m| m.contains("fn side_verdict(")));
    assert!(dead.iter().all(|m| !m.contains("fn enum_str")));
    let moved = vec![(
        "crates/lib/analysis/src/thermal/range.rs".to_string(),
        "fn renamed_verdict(".to_string(),
    )];
    assert!(
        dead_rules(&moved)
            .iter()
            .any(|m| m.contains("fn side_verdict("))
    );
}

/// 形態の姿勢は解決（と、サービスの読み取りが通る `query/pose.rs`）だけが読み、形態ごとの解決済みの設計は射影の文脈（と同じ `query/pose.rs`）だけが
/// 選ぶ（テストは確かめに呼んでよい）。
#[test]
fn configuration_pose_is_read_only_by_the_resolution() {
    let rule = |needle: &str| PROD_RULES.iter().find(|r| r.0 == needle).unwrap();
    for (needle, home, other) in [
        (
            ".pose_in(",
            "crates/app/model/src/resolve/compute.rs",
            "crates/app/projection/src/pose.rs",
        ),
        (
            ".in_configuration(",
            "crates/app/projection/src/context.rs",
            "crates/app/service/src/query.rs",
        ),
    ] {
        let mut errors = Vec::new();
        check_rule(home, needle, rule(needle), &mut errors);
        check_rule(
            "crates/app/service/src/query/pose.rs",
            needle,
            rule(needle),
            &mut errors,
        );
        assert!(errors.is_empty());
        check_rule(other, needle, rule(needle), &mut errors);
        assert_eq!(errors.len(), 1);
    }
}
