use super::*;

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
            production_text("check.rs", &code).unwrap(),
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
        production_text("parts.rs", text).unwrap(),
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
