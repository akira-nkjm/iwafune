use super::*;

/// 消したあとに残る語（空白区切り）。
fn words(s: &str) -> Vec<String> {
    s.split_whitespace().map(str::to_string).collect()
}

#[test]
fn trailing_test_module_is_removed() {
    let text = "fn a() {}\n#[cfg(test)]\nmod tests {\n    fn t() { let s = \"}\"; }\n}\n";
    let out = strip_test_items(text);
    assert_eq!(words(&out), ["fn", "a()", "{}"]);
    assert_eq!(out.lines().count(), text.lines().count());
}

#[test]
fn production_after_a_mid_file_test_fn_is_kept() {
    let text = "#[cfg(test)]\nfn helper() -> u8 { 1 }\nfn real() { work(); }\n";
    assert_eq!(
        words(&strip_test_items(text)),
        ["fn", "real()", "{", "work();", "}"]
    );
}

#[test]
fn test_use_and_test_mod_declaration_end_at_semicolon() {
    let text = "#[cfg(test)]\nuse x::{a, b};\n#[cfg(test)]\nmod tests;\nfn keep() {}\n";
    assert_eq!(words(&strip_test_items(text)), ["fn", "keep()", "{}"]);
    let simple = "#[cfg(test)] use y;\nfn keep() {}";
    assert_eq!(words(&strip_test_items(simple)), ["fn", "keep()", "{}"]);
}

#[test]
fn test_method_inside_an_impl_is_removed() {
    let text = "impl A {\n    #[cfg(test)]\n    #[allow(dead_code)]\n    /// doc\n    fn t(&self) -> [u8; 2] { [0; 2] }\n    fn p(&self) {}\n}\n";
    assert_eq!(
        words(&strip_test_items(text)),
        ["impl", "A", "{", "fn", "p(&self)", "{}", "}"]
    );
}

#[test]
fn braces_in_strings_comments_and_chars_do_not_count() {
    let text = "#[cfg(test)]\nmod t {\n    // }\n    /* } */\n    fn f() { let a = '}'; let b = \"}{\"; let c = r#\"}\"#; let d = '\\''; }\n}\nfn after<'a>(x: &'a str) {}\n";
    assert_eq!(
        words(&strip_test_items(text)),
        ["fn", "after<'a>(x:", "&'a", "str)", "{}"]
    );
}

#[test]
fn two_test_items_and_attribute_in_a_string() {
    let text = "#[cfg(test)]\nfn a() {}\nfn keep() { let s = \"#[cfg(test)] fn x() {}\"; }\n#[cfg(test)]\nmod m { fn b() {} }\n";
    let out = strip_test_items(text);
    assert!(out.contains("keep"));
    assert!(out.contains("\"#[cfg(test)] fn x() {}\""));
    assert!(!out.contains("fn a"));
    assert!(!out.contains("mod m"));
}

#[test]
fn static_with_struct_body_keeps_no_stray_semicolon() {
    let out = strip_test_items("#[cfg(test)]\nstatic X: T = T { a: 1 };\nfn k() {}");
    assert_eq!(words(&out), ["fn", "k()", "{}"]);
}

#[test]
fn test_files_are_none() {
    assert!(production_text("crates/app/ops/tests/ops.rs", "x").is_none());
    assert!(production_text("crates/app/ops/src/tests.rs", "x").is_none());
    assert!(production_text("web/src/a.test.ts", "x").is_none());
    assert!(production_text("crates/app/ops/src/a.rs", "x").is_some());
}

#[test]
fn function_lengths_count_from_fn_to_closing_brace() {
    let code = "fn a() {\n    let s = \"}\";\n}\npub fn b<T>(x: [u8; 2]) -> u8 {\n    fn inner() {}\n    0\n}\ntrait T { fn decl(&self); }\nlet f: fn(u8) -> u8;\n";
    let spans = function_lengths(code);
    let got: Vec<(&str, usize, usize)> = spans
        .iter()
        .map(|s| (s.name.as_str(), s.line, s.len))
        .collect();
    assert_eq!(got, [("a", 1, 3), ("b", 4, 4), ("inner", 5, 1)]);
}
