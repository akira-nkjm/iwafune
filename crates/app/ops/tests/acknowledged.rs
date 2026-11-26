//! 承知済みの組（`analysis/structure.toml` の `[[acknowledged]]`）の編集（Op → ファイルの差分）。
//! fixtures/demo-6u に対して計画を立て、書かれた本文・正規形・名前の変更での書き換え・参照が残る削除の拒否を確かめる。

use fune_model::check;
use fune_model::design::FindingKind;
use fune_ops::{Op, OpError, plan};
use fune_store::format::format_text;
use fune_store::{Snapshot, load};
use std::path::PathBuf;

const FILE: &str = "analysis/structure.toml";

fn snapshot() -> Snapshot {
    load(&PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../../fixtures/demo-6u")).unwrap()
}

fn ack(finding: FindingKind, first: &str, second: &str, config: Option<&str>, why: &str) -> Op {
    Op::Acknowledge {
        finding,
        first: first.into(),
        second: second.into(),
        configuration: config.map(str::to_owned),
        reason: why.into(),
    }
}

fn unack(finding: FindingKind, first: &str, second: &str, config: Option<&str>) -> Op {
    Op::Unacknowledge {
        finding,
        first: first.into(),
        second: second.into(),
        configuration: config.map(str::to_owned),
    }
}

fn text_of(p: &fune_ops::Plan) -> String {
    p.changes
        .iter()
        .find(|c| c.path == FILE)
        .and_then(|c| c.after.clone())
        .unwrap_or_else(|| panic!("{FILE} not changed"))
}

/// 書かれた本文が正規形（`fune fmt --check` に通る）で、設計の検査に誤りが無い。
fn assert_clean(p: &fune_ops::Plan) {
    for c in &p.changes {
        if let Some(after) = &c.after {
            assert_eq!(&format_text(&c.path, after).unwrap(), after, "{}", c.path);
        }
    }
    assert!(check(&p.design).iter().all(|d| !d.is_error()));
}

use FindingKind::{ConeClearance, Interference};

/// デモが承知済みにしている組の数（ヒンジの縁 1 組とパネルの角 12 組。テストが足す組はその後ろに並ぶ）。
fn demo_count() -> usize {
    snapshot()
        .design
        .analysis
        .structure
        .unwrap()
        .acknowledged
        .len()
}

fn list_of(p: &fune_ops::Plan) -> &[fune_model::design::AcknowledgedFinding] {
    &p.design.analysis.structure.as_ref().unwrap().acknowledged
}

#[test]
fn acknowledging_writes_records_in_the_canonical_form() {
    let ops = [
        ack(Interference, "STT", "CAM", None, "Both look the same way"),
        ack(
            ConeClearance,
            "SS_PX",
            "SAP_PY",
            Some("STOWED"),
            "Folded paddle",
        ),
    ];
    let p = plan(&snapshot(), &ops).unwrap();
    let text = text_of(&p);
    assert!(
        text.contains(
            "[[acknowledged]]\nfinding = \"interference\"\nfirst   = \"STT\"\nsecond  = \"CAM\"\nreason  = \"Both look the same way\"\n"
        ),
        "{text}"
    );
    assert!(text.contains("configuration = \"STOWED\""), "{text}");
    // 既存の表はそのまま（承知は末尾に足す）
    assert!(text.contains("[material.al6061_t6]"));
    assert_clean(&p);
    let before = demo_count();
    assert_eq!(list_of(&p).len(), before + 2);
    assert_eq!(
        list_of(&p)[before + 1].configuration.as_deref(),
        Some("STOWED")
    );
}

/// 同じ組・同じ形態（干渉は並びを問わない）への承知は、新しい行を足さずに理由を書き換える。形態が違えば別の行。
#[test]
fn acknowledging_again_replaces_the_reason_and_a_configuration_makes_a_separate_entry() {
    let before = demo_count();
    let ops = [
        ack(Interference, "STT", "CAM", None, "first"),
        ack(Interference, "CAM", "STT", None, "second"),
        ack(Interference, "CAM", "STT", Some("STOWED"), "stowed only"),
    ];
    let p = plan(&snapshot(), &ops).unwrap();
    assert_eq!(list_of(&p).len(), before + 2, "{:?}", list_of(&p));
    assert_eq!(list_of(&p)[before].reason, "second");
    assert_eq!(
        list_of(&p)[before + 1].configuration.as_deref(),
        Some("STOWED")
    );
    assert_clean(&p);
    // デモがすでに承知している組（ヒンジの縁）も、並びを変えて書けば理由を書き換える
    let hinge = [ack(Interference, "SP_PY", "SAP_PY", None, "new reason")];
    let p = plan(&snapshot(), &hinge).unwrap();
    assert_eq!(list_of(&p).len(), before);
    assert_eq!(list_of(&p)[0].reason, "new reason");
}

/// 円錐の組は順がある: 担い手と部品を入れ替えた承知は別の組。
#[test]
fn a_cone_pair_is_ordered() {
    let ops = [
        ack(ConeClearance, "SS_PX", "SAP_PY", None, "a"),
        ack(ConeClearance, "SAP_PY", "SS_PX", None, "b"),
    ];
    let p = plan(&snapshot(), &ops).unwrap();
    assert_eq!(list_of(&p).len(), demo_count() + 2);
}

#[test]
fn unacknowledging_removes_the_entry_and_the_empty_list() {
    let snap = snapshot();
    let add = ack(Interference, "STT", "CAM", None, "Both look the same way");
    let remove = unack(Interference, "CAM", "STT", None);
    let p = plan(&snap, &[add.clone(), remove.clone()]).unwrap();
    // 足して消したので、ファイルは元に戻る
    assert!(p.changes.iter().all(|c| c.path != FILE), "{:?}", p.changes);
    // 無い承知を取り消すのは拒む。形態が違えば別の承知
    let err = plan(&snap, std::slice::from_ref(&remove)).unwrap_err();
    assert!(matches!(err, OpError::NotFound { .. }), "{err:?}");
    let err = plan(
        &snap,
        &[add, unack(Interference, "STT", "CAM", Some("STOWED"))],
    )
    .unwrap_err();
    assert!(matches!(err, OpError::NotFound { .. }), "{err:?}");
    // 一覧が空になれば、キーごと消える（デモの承知をすべて取り消す）
    let all: Vec<Op> = list_of_demo()
        .iter()
        .map(|a| unack(a.finding, &a.first, &a.second, None))
        .collect();
    let p = plan(&snap, &all).unwrap();
    assert!(!text_of(&p).contains("acknowledged"));
    assert_clean(&p);
}

fn list_of_demo() -> Vec<fune_model::design::AcknowledgedFinding> {
    snapshot().design.analysis.structure.unwrap().acknowledged
}

#[test]
fn bad_acknowledgements_are_refused() {
    let snap = snapshot();
    for op in [
        ack(Interference, "NOPE", "STT", None, "x"),
        ack(Interference, "STT", "CAM", Some("NOPE"), "x"),
        ack(Interference, "STT", "STT", None, "x"),
        ack(Interference, "STT", "CAM", None, "   "),
    ] {
        assert!(plan(&snap, std::slice::from_ref(&op)).is_err(), "{op:?}");
    }
}

/// 部品・形態の名前の変更は承知の行を書き換える（形態を指した行も）。
#[test]
fn renames_follow_into_the_entries() {
    let snap = snapshot();
    let mut ops = vec![ack(
        ConeClearance,
        "SS_PX",
        "SAP_PY",
        Some("STOWED"),
        "Folded paddle",
    )];
    ops.push(Op::RenamePart {
        id: "SAP_PY".into(),
        to: "PADDLE_PY".into(),
    });
    ops.push(Op::RenameConfiguration {
        id: "STOWED".into(),
        to: "FOLDED".into(),
    });
    let p = plan(&snap, &ops).unwrap();
    let text = text_of(&p);
    assert!(
        !text.contains("SAP_PY") && !text.contains("STOWED"),
        "{text}"
    );
    assert!(
        text.contains("\"PADDLE_PY\"") && text.contains("\"FOLDED\""),
        "{text}"
    );
    assert_clean(&p);
    let list = list_of(&p);
    // デモのヒンジの縁（`SAP_PY` と `SP_PY`）の行も、新しい名前に書き換わる
    assert_eq!(
        (list[0].first.as_str(), list[0].second.as_str()),
        ("PADDLE_PY", "SP_PY")
    );
    let last = list.last().unwrap();
    assert_eq!(last.second, "PADDLE_PY");
    assert_eq!(last.configuration.as_deref(), Some("FOLDED"));
    // 名前を変えたあとも、新しい名前で取り消せる
    ops.push(unack(Interference, "SP_PY", "PADDLE_PY", None));
    assert!(plan(&snap, &ops).is_ok());
}

/// 承知が指している部品・形態を消すのは拒む（先に承知を取り消す）。形態は、打ち上げ条件やフェーズが選んでいないものを使う。
#[test]
fn removing_a_part_or_configuration_that_an_entry_names_is_refused() {
    let snap = snapshot();
    let create = Op::CreateConfiguration {
        id: "SPARE".into(),
        label: None,
        label_en: None,
        description: None,
    };
    let add = ack(Interference, "SS_MX", "CAM", Some("SPARE"), "x");
    // 承知が無ければ形態は消せる（この形態を指す参照は承知だけ）
    let ok = [
        create.clone(),
        Op::RemoveConfiguration { id: "SPARE".into() },
    ];
    assert!(plan(&snap, &ok).is_ok());
    // 部品は適用後の検査が拒む
    let err = plan(
        &snap,
        &[
            create.clone(),
            add.clone(),
            Op::RemovePart { id: "SS_MX".into() },
        ],
    )
    .unwrap_err();
    let OpError::Rejected { diagnostics } = err else {
        panic!("expected a rejection: {err:?}");
    };
    assert!(
        diagnostics.iter().any(|d| d.code.starts_with("ref.")),
        "{diagnostics:?}"
    );
    // 形態は、指している場所を挙げて先に拒む
    let err = plan(
        &snap,
        &[
            create.clone(),
            add.clone(),
            Op::RemoveConfiguration { id: "SPARE".into() },
        ],
    )
    .unwrap_err();
    let OpError::Invalid { message } = err else {
        panic!("expected a refusal: {err:?}");
    };
    assert!(
        message.contains("`[[acknowledged]]` in analysis/structure.toml"),
        "{message}"
    );

    // 先に取り消せば消せる
    let ops = [
        create,
        add,
        unack(Interference, "SS_MX", "CAM", Some("SPARE")),
        Op::RemoveConfiguration { id: "SPARE".into() },
    ];
    assert!(plan(&snap, &ops).is_ok());
}
