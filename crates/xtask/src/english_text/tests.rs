use super::*;

fn lines(text: &str) -> Vec<usize> {
    scan(text).iter().map(|f| f.line).collect()
}

#[test]
fn literals_are_found_but_comments_are_not() {
    let src = "// 日本語のコメント\n/// 日本語のドキュメント\n/* ブロック /* 入れ子 */ まだコメント */\nlet a = \"こんにちは\";\nlet b = \"english\";\n";
    assert_eq!(lines(src), vec![4]);
    assert_eq!(scan(src)[0].text, "こんにちは");
}

#[test]
fn comment_markers_inside_literals_do_not_start_comments() {
    let src = "let a = \"http://x // 漢字\";\nlet b = \"/* 漢字\";\nlet c = \"漢字\";\n";
    assert_eq!(lines(src), vec![1, 2, 3]);
}

#[test]
fn raw_strings_and_escapes() {
    let src = "let a = r#\"raw \" 引用符 \"#;\nlet b = \"esc \\\" 漢字\";\nlet c = br\"ascii\";\nlet d = r\"かな\";\nlet var = \"ok\";\n";
    assert_eq!(lines(src), vec![1, 2, 4]);
}

#[test]
fn char_literals_and_lifetimes() {
    let src =
        "fn f<'a>(x: &'a str) -> char { 'あ' }\nlet q = '\"';\nlet s = \"漢字\";\nlet t = '\\'';\n";
    // 'あ' は char リテラル、'"' の引用符は文字列を始めない
    assert_eq!(lines(src), vec![1, 3]);
}

#[test]
fn identifiers_ending_in_r_are_not_raw_strings() {
    let src = "let bar = \"漢字\";\nlet r = 1;\nlet s = \"字\";\n";
    assert_eq!(lines(src), vec![1, 3]);
}

#[test]
fn cfg_test_items_are_skipped() {
    let src = "fn a() { let x = \"本番\"; }\n#[cfg(test)]\nmod tests {\n    fn t() { let y = \"テスト { \"; }\n    fn u() { if true { let z = \"漢字\"; } }\n}\nfn b() { let w = \"本番2\"; }\n#[cfg(test)]\nmod more;\nfn c() { let v = \"本番3\"; }\n";
    let texts: Vec<String> = scan(src).into_iter().map(|f| f.text).collect();
    assert_eq!(texts, vec!["本番", "本番2", "本番3"]);
}

#[test]
fn cfg_test_attribute_with_spaces_and_on_a_function() {
    let src = "# [ cfg ( test ) ]\nfn t() { let y = \"テスト\"; }\nfn p() { let y = \"本番\"; }\n";
    assert_eq!(lines(src), vec![3]);
}

#[test]
fn other_cfg_attributes_are_not_tests() {
    let src = "#[cfg(not(test))]\nfn p() { let y = \"本番\"; }\n";
    assert_eq!(lines(src), vec![2]);
}

#[test]
fn doc_lines_in_cli_args_are_found() {
    let src =
        "/// English help\n//! 日本語（モジュール）\n/// 日本語のヘルプ\n// 日本語のコメント\n";
    assert_eq!(japanese_doc_lines(src), vec![3]);
    assert!(is_cli_args("crates/app/cli/src/args/defs.rs"));
    assert!(is_cli_args("crates/app/cli/src/args.rs"));
    assert!(!is_cli_args("crates/app/cli/src/cmd/set.rs"));
}

#[test]
fn scope_covers_sources_but_not_tests() {
    assert!(in_scope("crates/lib/analysis/src/power.rs"));
    assert!(in_scope("crates/app/cli/src/args/defs.rs"));
    assert!(in_scope("crates/xtask/src/main.rs"));
    assert!(!in_scope("crates/lib/analysis/src/power/tests.rs"));
    assert!(!in_scope("crates/lib/analysis/tests/demo.rs"));
    assert!(!in_scope("crates/lib/analysis/build.rs"));
    assert!(!in_scope("web/src/a.ts"));
}

fn finding(line: usize) -> Finding {
    Finding {
        line,
        text: "日本語".to_owned(),
    }
}

#[test]
fn allowlist_hides_listed_files_and_reports_stale_entries() {
    let mut found = BTreeMap::new();
    found.insert("a.rs".to_owned(), vec![finding(3)]);
    found.insert("b.rs".to_owned(), vec![finding(7)]);
    let errors = judge(&found, &[("a.rs", "bilingual titles")]);
    assert_eq!(errors.len(), 1);
    assert!(errors[0].starts_with("b.rs:7:"));

    let errors = judge(
        &found,
        &[("a.rs", "x"), ("b.rs", "y"), ("gone.rs", "stale")],
    );
    assert_eq!(errors.len(), 1);
    assert!(errors[0].contains("gone.rs") && errors[0].contains("remove the entry"));
}
