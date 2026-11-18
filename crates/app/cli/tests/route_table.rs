//! HTTP の経路と CLI のコマンドの対応表（docs/architecture.md §4.3）。表が経路と食い違ったら落ちる:
//! `crates/app/server/src/routes.rs` の経路はすべて表にあり、表の経路はすべて `routes.rs` にある。
//! CLI のコマンドは実在する（`--help` が通る）。すべての経路に CLI がある（#158）。
//! 例外は `fune init`（サーバーはプロジェクトの中でしか動かない）・`fune gui`（サーバーを起動するコマンド）・
//! `fune self-update`（CLI のバイナリ自身を置き換える）だけ。

mod common;

use std::collections::BTreeSet;

/// 経路 → (CLI のコマンド, service の関数)。CLI は `fune` のあとの語（`--help` で存在を確かめる）。
const TABLE: &[(&str, &str, &[&str], &str)] = &[
    ("GET", "/project", &["status"], "query::project_info"),
    (
        "GET",
        "/design",
        &["net", "ls"],
        "query::design_view (`… ls` lists each; `view ls` is `query::view_layout`)",
    ),
    ("GET", "/maneuvers", &["maneuver", "ls"], "query::maneuvers"),
    ("GET", "/parts", &["part", "ls"], "query::list_parts"),
    ("GET", "/parts/{id}", &["part", "show"], "query::part_view"),
    (
        "POST",
        "/parts/{id}",
        &["part", "show"],
        "query::part_view（`--ops`）",
    ),
    (
        "POST",
        "/parts/{id}/place_body",
        &["part", "place"],
        "edit::place_body（`--body-pos`）",
    ),
    ("GET", "/values/{path}", &["get"], "query::get_value"),
    ("GET", "/defs", &["attr", "ls"], "query::defs_view"),
    (
        "GET",
        "/power_page",
        &["power-page"],
        "power_page::power_page",
    ),
    (
        "POST",
        "/system_modes/{system}",
        &["mode", "view"],
        "system_modes::system_modes（`--ops`）",
    ),
    ("GET", "/diagnostics", &["check"], "query::check"),
    ("POST", "/fmt", &["fmt"], "format::format_design"),
    ("POST", "/ops", &["apply"], "edit::apply"),
    ("POST", "/ops/from_text", &["set"], "edit::set_op"),
    ("POST", "/ops/unset_from_text", &["unset"], "edit::unset_op"),
    (
        "POST",
        "/ops/from_command",
        &["part", "place"],
        "edit::command_ops",
    ),
    (
        "POST",
        "/ops/case_from_eclipse",
        &["case", "from-eclipse"],
        "edit::case_from_eclipse_op",
    ),
    ("GET", "/history", &["history"], "history::view"),
    ("POST", "/undo", &["undo"], "history::undo"),
    ("POST", "/redo", &["redo"], "history::redo"),
    ("GET", "/units", &["units", "ls"], "units::units_catalog"),
    (
        "POST",
        "/units/parse",
        &["units", "parse"],
        "units::parse_quantity",
    ),
    (
        "POST",
        "/units/format",
        &["units", "format"],
        "units::format_quantities",
    ),
    (
        "GET",
        "/analyses",
        &["analyze"],
        "analysis::analysis_catalog（`analyze ls`）",
    ),
    (
        "POST",
        "/analyses/{id}",
        &["analyze"],
        "analysis::run_analysis",
    ),
    ("GET", "/budgets/{kind}", &["budget"], "budget::budget"),
    (
        "POST",
        "/budgets/{kind}",
        &["budget"],
        "budget::budget（`--ops`・`--case`）",
    ),
    (
        "GET",
        "/data/status",
        &["data", "status"],
        "data::data_status",
    ),
    (
        "GET",
        "/data/space_weather",
        &["data", "space-weather"],
        "data::space_weather::lookup",
    ),
    (
        "POST",
        "/data/{id}/fetch",
        &["data", "fetch"],
        "data::fetch",
    ),
    (
        "GET",
        "/catalog/{*id}",
        &["catalog", "show"],
        "query::catalog_item",
    ),
    (
        "POST",
        "/catalog/fetch",
        &["catalog", "fetch"],
        "shared_catalog::fetch",
    ),
    (
        "POST",
        "/catalog/update",
        &["catalog", "update"],
        "shared_catalog::update",
    ),
];

/// HTTP に無い CLI のコマンド。
const CLI_ONLY: &[&str] = &["init", "gui", "self-update"];

/// `routes.rs` の `.route("<path>", get(..).post(..))` を `METHOD path` の集合にする。
fn routes_in_source() -> BTreeSet<(String, String)> {
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../server/src/routes.rs");
    let src = std::fs::read_to_string(path).unwrap();
    let mut found = BTreeSet::new();
    for chunk in src.split(".route(").skip(1) {
        let chunk = chunk.split(".fallback(").next().unwrap_or(chunk);
        let chunk = chunk.split("\n        .route(").next().unwrap_or(chunk);
        let path = chunk.split('"').nth(1).unwrap().to_owned();
        let rest = chunk.split_once(',').map_or("", |(_, r)| r);
        let rest = rest.split(".nest(").next().unwrap_or(rest);
        for (token, method) in [("get(", "GET"), ("post(", "POST")] {
            if rest.contains(token) || rest.contains(&format!(".{token}")) {
                found.insert((method.to_owned(), path.clone()));
            }
        }
    }
    found
}

#[test]
fn every_route_is_in_the_table() {
    let documented: BTreeSet<(String, String)> = TABLE
        .iter()
        .map(|(m, p, _, _)| ((*m).to_owned(), (*p).to_owned()))
        .collect();
    let actual = routes_in_source();
    assert!(actual.len() > 20, "could not read the routes: {actual:?}");
    let missing: Vec<_> = actual.difference(&documented).collect();
    let stale: Vec<_> = documented.difference(&actual).collect();
    assert!(
        missing.is_empty() && stale.is_empty(),
        "routes missing from the table: {missing:?} / table routes missing from routes.rs: {stale:?} (also fix docs/architecture.md §4.3)"
    );
}

#[test]
fn every_cli_command_in_the_table_exists() {
    for (method, path, words, _) in TABLE {
        let out = common::fune().args(*words).arg("--help").output().unwrap();
        assert!(
            out.status.success(),
            "{method} {path} has no CLI `fune {}`",
            words.join(" ")
        );
    }
    for name in CLI_ONLY {
        let out = common::fune().arg(name).arg("--help").output().unwrap();
        assert!(out.status.success(), "fune {name} does not exist");
    }
}
