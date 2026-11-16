//! 出典の台帳の欄（版・取得日・sha256・配布の区分）と値の出典の位置（`from.at`）の書き込み:
//! 新しいファイルの正規形、書いたものを読むと元に戻ること、形の誤りを拒むこと（fixtures/demo-6u に対して計画だけを立てる）。

use fune_model::design::{Distribution, Source, SourceKind};
use fune_model::value::{Provenance, Scalar, Value};
use fune_ops::{Op, OpError, plan, to_cli};
use fune_store::{Snapshot, load};
use std::path::PathBuf;

fn snapshot() -> Snapshot {
    load(&PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../../fixtures/demo-6u")).unwrap()
}

const HASH: &str = "9f86d081884c7d659a2feaa0c55ad015a3bf4f1b2b0b822cd15d6c15b0f00a08";

fn full_source() -> Source {
    Source {
        label: Some("RW データシート".into()),
        reference: Some("https://example.com/rw.pdf".into()),
        revision: Some("Rev. C".into()),
        retrieved: Some("2026-10-15".into()),
        sha256: Some(HASH.into()),
        distribution: Some(Distribution::Public),
        date: Some("2026-10-01".into()),
        by: Some("system-lead".into()),
        note: Some("架空の例".into()),
        ..Source::new("ds_rw_new", SourceKind::Datasheet)
    }
}

#[test]
fn a_new_source_writes_every_field_in_order_and_reads_back() {
    let snap = snapshot();
    let source = full_source();
    let p = plan(
        &snap,
        &[Op::SetSource {
            source: source.clone(),
            dir: None,
        }],
    )
    .unwrap();
    let text = p.changes[0].after.as_deref().unwrap();
    assert_eq!(
        text,
        format!(
            "kind         = \"datasheet\"\nlabel        = \"RW データシート\"\nref          = \"https://example.com/rw.pdf\"\n\
             revision     = \"Rev. C\"\nretrieved    = \"2026-10-15\"\nsha256       = \"{HASH}\"\n\
             distribution = \"public\"\ndate         = \"2026-10-01\"\nby           = \"system-lead\"\nnote         = \"架空の例\"\n"
        )
    );
    assert_eq!(
        fune_store::format::format_text(&p.changes[0].path, text).unwrap(),
        text
    );
    assert_eq!(p.design.sources["ds_rw_new"], source);
    assert_eq!(
        to_cli(&Op::SetSource { source, dir: None }),
        vec![format!(
            "fune source set ds_rw_new --kind datasheet --label \"RW データシート\" --ref https://example.com/rw.pdf \
             --revision \"Rev. C\" --retrieved 2026-10-15 --sha256 {HASH} --distribution public --date 2026-10-01 \
             --by system-lead --note \"架空の例\""
        )]
    );
}

#[test]
fn malformed_hashes_and_dates_are_rejected() {
    let snap = snapshot();
    for source in [
        Source {
            sha256: Some("abc".into()),
            ..full_source()
        },
        Source {
            retrieved: Some("2026/10/15".into()),
            ..full_source()
        },
    ] {
        let p = plan(&snap, &[Op::SetSource { source, dir: None }]);
        assert!(matches!(p, Err(OpError::Rejected { .. })), "{p:?}");
    }
}

/// 値の出典の位置は `from.source` の次の `from.at` の 1 行（docs/model.md §10.2 の 3）。
#[test]
fn the_location_in_the_source_is_its_own_line() {
    let snap = snapshot();
    let op = Op::SetValue {
        part: "HTR_BAT".into(),
        attr: "specific_heat".into(),
        state: None,
        value: Value::Scalar(Scalar::new(1100.0, "J/(kg*K)")),
        from: Some(Provenance {
            source: Some("tbd".into()),
            at: Some("p.4 表 2".into()),
            note: Some("仮の値".into()),
        }),
    };
    let p = plan(&snap, std::slice::from_ref(&op)).unwrap();
    let text = p.changes[0].after.as_deref().unwrap();
    assert!(
        text.contains("[attr.specific_heat]\nvalue       = 1100.0\nunit        = \"J/(kg*K)\"\nfrom.source = \"tbd\"\nfrom.at     = \"p.4 表 2\"\nfrom.note   = \"仮の値\"\n"),
        "{text}"
    );
    let read = &p.design.parts["HTR_BAT"].attr["specific_heat"];
    assert_eq!(
        read.from.as_ref().and_then(|f| f.at.as_deref()),
        Some("p.4 表 2")
    );
    assert_eq!(
        to_cli(&op),
        vec![
            "fune set HTR_BAT.specific_heat \"1100 J/(kg*K)\" --source tbd --at \"p.4 表 2\" --note \"仮の値\""
                .to_string()
        ]
    );
}
