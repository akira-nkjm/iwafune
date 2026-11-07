use super::*;

fn graph(source: &str) -> Graph {
    let mut sources = Sources::default();
    let file = syn::parse_file(source).unwrap();
    sources
        .collect(file.items, vec![], Path::new("/tmp"), Path::new("/tmp"))
        .unwrap();
    sources.graph()
}

#[test]
fn catches_direct_nested_super_and_macro_cycles() {
    for source in [
        "mod a { use crate::b::B; pub struct A; } mod b { use crate::a::A; pub struct B; }",
        "mod a { mod inner { use super::super::b::B; } pub struct A; } mod b { use crate::a::A; pub struct B; }",
        "mod a { registry!(crate::b::B); pub struct A; } mod b { use crate::a::A; pub struct B; }",
    ] {
        assert!(!cycles(&graph(source)).is_empty(), "{source}");
    }
}

#[test]
fn facades_and_tests_do_not_introduce_cycles() {
    let source = r#"
        pub use leaf::Thing;
        pub use api::run;
        mod leaf { pub struct Thing; }
        mod api {
            use crate::Thing;
            /// crate::consumer::run() is only documentation.
            pub fn run() {}
            #[cfg(test)] mod tests { use crate::consumer; }
            #[cfg(test)] fn helper() { crate::consumer::run(); }
        }
        mod consumer { pub fn run() { crate::run(); } }
    "#;
    assert!(cycles(&graph(source)).is_empty());
}

#[test]
fn reexports_are_followed_to_definitions() {
    let source = r#"
        mod facade { pub use crate::leaf::Thing as Alias; }
        mod leaf { pub struct Thing; pub fn go() { crate::consumer::run(); } }
        mod consumer { use crate::facade::Alias; pub fn run() {} }
    "#;
    let graph = graph(source);
    assert!(graph[&vec!["consumer".into()]].contains(&vec!["leaf".into()]));
    assert!(!cycles(&graph).is_empty());
}

#[test]
fn module_and_reexported_function_may_have_the_same_name() {
    let source = "pub use check::check; mod check { pub fn check() {} } mod client { fn run() { crate::check::check(); } }";
    assert!(cycles(&graph(source)).is_empty());
}
