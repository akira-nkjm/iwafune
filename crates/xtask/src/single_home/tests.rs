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
    let rule = |needle: &str| RULES.iter().find(|r| r.0 == needle).unwrap();
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

/// 読み込み中のスケルトンは kit/Loading の部品だけが ui/skeleton を使う。
#[test]
fn skeletons_come_from_kit_loading() {
    let rule = WEB_RULES.iter().find(|r| r.0 == "@/ui/skeleton").unwrap();
    let line = "import { Skeleton } from \"@/ui/skeleton\";";
    let mut errors = Vec::new();
    check_rule("web/src/kit/Loading.tsx", line, rule, &mut errors);
    assert!(errors.is_empty());
    check_rule("web/src/kit/KpiCard.tsx", line, rule, &mut errors);
    check_rule(
        "web/src/features/layout3d/LazyLayout3D.tsx",
        line,
        rule,
        &mut errors,
    );
    assert_eq!(errors.len(), 2);
}
