//! 出典の台帳の欄（版・取得日・sha256・配布の区分）と値の出典の位置（`from.at`）: 読み込みと検査。

use fune_model::design::{Distribution, Source, SourceKind};
use fune_model::{Design, Diagnostic, Severity, check};
use fune_store::FileSet;
use fune_store::read::design_of;

const PROJECT: &str =
    "[project]\nname = \"t\"\nschema = 2\n\n[[catalog]]\nname = \"local\"\npath = \"catalog\"\n";

const HASH: &str = "9f86d081884c7d659a2feaa0c55ad015a3bf4f1b2b0b822cd15d6c15b0f00a08";

fn design(files: &[(&str, &str)]) -> Design {
    let mut set = FileSet::new();
    set.insert("fune.toml", PROJECT);
    for (p, t) in files {
        set.insert(*p, *t);
    }
    design_of(&set)
}

fn codes(d: &[Diagnostic], code: &str) -> Vec<String> {
    d.iter()
        .filter(|x| x.code == code && x.severity == Severity::Error)
        .map(|x| x.message.clone())
        .collect()
}

fn source_text(extra: &str) -> String {
    format!("kind = \"datasheet\"\nref = \"https://example.com/rw.pdf\"\n{extra}")
}

#[test]
fn every_source_field_reads_from_toml() {
    let text = source_text(&format!(
        "revision = \"Rev. C\"\nretrieved = \"2026-10-15\"\nsha256 = \"{HASH}\"\ndistribution = \"on_request\"\n"
    ));
    let d = design(&[("sources/ds_rw.toml", &text)]);
    assert!(d.load_diagnostics.is_empty(), "{:?}", d.load_diagnostics);
    let want = Source {
        reference: Some("https://example.com/rw.pdf".into()),
        revision: Some("Rev. C".into()),
        retrieved: Some("2026-10-15".into()),
        sha256: Some(HASH.into()),
        distribution: Some(Distribution::OnRequest),
        ..Source::new("ds_rw", SourceKind::Datasheet)
    };
    assert_eq!(d.sources["ds_rw"], want);
    assert_eq!(want.web_link(), Some("https://example.com/rw.pdf"));
    assert!(check(&d).iter().all(|x| !x.is_error()), "{:?}", check(&d));
    let bad = design(&[(
        "sources/x.toml",
        "kind = \"datasheet\"\ndistribution = \"nda\"\n",
    )]);
    assert!(!bad.load_diagnostics.is_empty());
}

#[test]
fn sha256_must_be_64_hex_digits() {
    for (hash, ok) in [
        (HASH.to_string(), true),
        (HASH.to_uppercase(), true),
        (HASH[..63].to_string(), false),
        (format!("{}g", &HASH[..63]), false),
        (format!("sha256:{HASH}"), false),
    ] {
        let d = check(&design(&[(
            "sources/ds.toml",
            &source_text(&format!("sha256 = \"{hash}\"\n")),
        )]));
        assert_eq!(codes(&d, "source.sha256").is_empty(), ok, "{hash}: {d:?}");
    }
    let d = check(&design(&[(
        "sources/ds.toml",
        &source_text("sha256 = \"abc\"\n"),
    )]));
    let found = d.iter().find(|x| x.code == "source.sha256");
    assert_eq!(found.and_then(|x| x.line), Some(3));
}

#[test]
fn dates_must_be_calendar_days() {
    let d = check(&design(&[(
        "sources/ds.toml",
        &source_text("date = \"2026-02-30\"\nretrieved = \"Oct 2026\"\n"),
    )]));
    let found = codes(&d, "source.date");
    assert_eq!(found.len(), 2, "{d:?}");
    assert!(found[0].contains("date") && found[1].contains("retrieved"));
    let d = check(&design(&[(
        "sources/ds.toml",
        &source_text("date = \"2024-02-29\"\nretrieved = \"2026-10-15\"\n"),
    )]));
    assert!(codes(&d, "source.date").is_empty(), "{d:?}");
}

/// カタログ品の `datasheet` は出典の ID。無い出典は `ref.source`、古い書き方の URL・パスは移行を促す warning。
#[test]
fn datasheet_and_default_source_must_name_a_source() {
    let item = "kind = \"reaction_wheel\"\ndatasheet = \"docs/rw.pdf\"\nsource = \"ds_none\"\n";
    let d = check(&design(&[("catalog/rw.toml", item)]));
    assert_eq!(codes(&d, "ref.source").len(), 1, "{d:?}");
    let legacy = d
        .iter()
        .filter(|x| x.code == "source.datasheet_location" && x.severity == Severity::Warning)
        .count();
    assert_eq!(legacy, 1, "{d:?}");
    let item = "kind = \"reaction_wheel\"\ndatasheet = \"ds_missing\"\n";
    let d = check(&design(&[("catalog/rw.toml", item)]));
    assert!(
        codes(&d, "ref.source")
            .iter()
            .any(|m| m.contains("データシート"))
    );
    let item = "kind = \"reaction_wheel\"\ndatasheet = \"ds_rw\"\nsource = \"ds_rw\"\n";
    let d = check(&design(&[
        ("catalog/rw.toml", item),
        ("sources/ds_rw.toml", &source_text("")),
    ]));
    assert!(codes(&d, "ref.source").is_empty(), "{d:?}");
}

/// 公開できない出典を使う品は、書き込めないカタログ（共有カタログ）のときだけ error。
#[test]
fn shared_catalog_items_must_not_use_restricted_sources() {
    let restricted = source_text("distribution = \"restricted\"\n");
    let item = "kind = \"reaction_wheel\"\ndatasheet = \"ds_nda\"\n\n[attr.mass]\nvalue = 0.15\nunit = \"kg\"\nfrom.source = \"ds_nda\"\nfrom.at = \"p.2\"\n";
    let mut d = design(&[
        ("catalog/rw.toml", item),
        ("sources/ds_nda.toml", &restricted),
    ]);
    assert!(d.load_diagnostics.is_empty(), "{:?}", d.load_diagnostics);
    let from = d.catalog["local/rw"].attr["mass"].from.clone().unwrap();
    assert_eq!(from.at.as_deref(), Some("p.2"));
    assert!(codes(&check(&d), "source.restricted").is_empty());
    // 同じ品が共有カタログ（`git = …`）の品なら、datasheet と値の出典の 2 か所が error。
    let shared = &mut d.project.catalogs[0];
    shared.path = None;
    shared.git = Some("https://example.com/catalog.git".into());
    let found = codes(&check(&d), "source.restricted");
    assert_eq!(found.len(), 2, "{found:?}");
    assert!(found[0].contains("local/rw") && found[0].contains("ds_nda"));
    // 区分が restricted でなければ共有カタログでも使える。
    let mut open = design(&[
        ("catalog/rw.toml", item),
        (
            "sources/ds_nda.toml",
            &source_text("distribution = \"on_request\"\n"),
        ),
    ]);
    open.project.catalogs[0].path = None;
    assert!(codes(&check(&open), "source.restricted").is_empty());
}
