use super::*;
use crate::dto::{CheckView, FillBasis};
use sat_units::typed::{Power, Temperature};
use serde_json::json;

fn check(verdict: Verdict) -> CheckView {
    CheckView {
        key: "range:N".into(),
        label: "動作温度範囲".into(),
        label_en: None,
        verdict,
        message: String::new(),
        message_en: None,
        at: "/checks/0".into(),
        scope: None,
    }
}

#[test]
fn worst_values_skip_non_finite_and_take_the_named_side() {
    let w = Qty::<Power>::from_raw;
    let lo = smallest([w(3.0), w(f64::NAN), w(-5.09), w(17.4)]).unwrap();
    assert_eq!(
        lo,
        Measured {
            si: -5.09,
            kind: "Power"
        }
    );
    let hi = largest([Fraction::new(0.14), Fraction::new(1.0), Fraction::new(0.33)]).unwrap();
    assert_eq!(
        hi,
        Measured {
            si: 1.0,
            kind: "Fraction"
        }
    );
    assert!(largest(Vec::<Qty<Temperature>>::new()).is_none());
    assert!(one(Decibel::new(f64::INFINITY)).is_none());
    assert_eq!(one(Decibel::new(3.05)).unwrap().kind, "Decibel");
}

/// どの KPI のキーも、ちょうど 1 つの解析が出す（キーは系を通して一意）。
#[test]
fn every_kpi_comes_from_exactly_one_analysis() {
    let mut seen = std::collections::HashSet::new();
    for info in sat_analysis::catalog::catalog() {
        for key in keys(&info.id) {
            assert!(seen.insert(*key), "{key:?} が 2 つの解析にある");
        }
        let defs = defs(&info.id);
        assert!(
            defs.iter()
                .all(|d| !d.label.is_empty() && !d.label_en.is_empty())
        );
    }
    assert_eq!(seen.len(), 25, "KpiKey のすべての値が表にある");
}

/// 判定を数える KPI: 数と OK でない数、塗りは OK の割合（手で数えた 9 件中 4 件）。
#[test]
fn counting_kpi_tallies_checks_and_fills_with_the_ok_share() {
    let mut checks = vec![check(Verdict::Ok); 5];
    checks.extend([
        check(Verdict::Violation),
        check(Verdict::Warning),
        check(Verdict::Violation),
        check(Verdict::Violation),
    ]);
    let output = json!({"cases": [], "envelope": [], "checks": []});
    let out = kpis(
        "thermal.steady",
        Some(&output),
        &checks,
        KpiContext::default(),
    );
    let keys: Vec<KpiKey> = out.iter().map(|k| k.key).collect();
    assert_eq!(
        keys,
        [
            KpiKey::MaxTemperature,
            KpiKey::MinTemperature,
            KpiKey::NodesInRange
        ]
    );
    let range = &out[2];
    assert_eq!(
        range.tally,
        Some(KpiTally {
            checked: 9,
            flagged: 4
        })
    );
    let fill = range.fill.unwrap();
    assert_eq!(fill.basis, FillBasis::OkShare);
    assert!((fill.ratio - 5.0 / 9.0).abs() < 1e-12);
    assert_eq!(range.verdict, Some(Verdict::Violation));
    // 節点が無ければ最高・最低の値は無い（判定は解析の判定のまま）
    assert!(out[0].value.is_none() && out[0].tally.is_none());
    assert_eq!(out[0].verdict, Some(Verdict::Violation));
}

/// 比の KPI は値そのもので塗り（0〜1 に収める）、判定の対象でない KPI は判定を持たない。
#[test]
fn ratio_kpis_fill_with_their_value_and_info_kpis_have_no_verdict() {
    let ratio = view(
        KpiKey::StorageMargin,
        Some(Measured {
            si: -0.036,
            kind: "Fraction",
        }),
        Some(Verdict::Violation),
        KpiTally {
            checked: 1,
            flagged: 1,
        },
    );
    assert_eq!(ratio.value.as_ref().unwrap().display, "-3.60 %");
    let fill = ratio.fill.unwrap();
    assert_eq!((fill.ratio, fill.basis), (0.0, FillBasis::Value));
    assert_eq!(ratio.verdict, Some(Verdict::Violation));
    assert!(ratio.tally.is_none());
    let info = view(
        KpiKey::DataPerDay,
        Some(Measured {
            si: 2.2132224e10,
            kind: "Information",
        }),
        Some(Verdict::Violation),
        KpiTally {
            checked: 1,
            flagged: 1,
        },
    );
    assert_eq!(info.value.unwrap().display, "2.77 GB");
    assert!(info.verdict.is_none() && info.fill.is_none());
}

/// 出力が無い・型に戻せないときは値を出さない（KPI の並びは残す）。
#[test]
fn missing_or_foreign_outputs_give_no_values() {
    assert!(kpis("power.energy_balance", None, &[], KpiContext::default()).is_empty());
    let out = kpis(
        "power.energy_balance",
        Some(&json!({"unexpected": true})),
        &[],
        KpiContext::default(),
    );
    assert_eq!(out.len(), 2);
    assert!(out.iter().all(|k| k.value.is_none()));
    assert!(
        kpis(
            "power.solar_sunlit",
            Some(&json!({})),
            &[],
            KpiContext::default()
        )
        .is_empty()
    );
}
