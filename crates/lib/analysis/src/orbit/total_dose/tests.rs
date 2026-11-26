//! `orbit.total_dose` のテスト。期待値は手計算（線量 = 線量率 × 期間、余裕 = 許容 / (総線量 × RDM) − 1、対数線形の補間）から決める。

use super::*;

const YEAR: f64 = 365.25 * 86_400.0;

/// 線量率 [Gy/yr] → SI [Gy/s]。
fn per_year(gy: f64) -> Qty<AbsorbedDoseRate> {
    Qty::from_raw(gy / YEAR)
}

fn point(mm: f64, gy_per_year: f64) -> DoseDepthPoint {
    DoseDepthPoint {
        shielding: Qty::from_raw(mm * 1e-3),
        rate: per_year(gy_per_year),
    }
}

fn leg(phase: &str, years: Option<f64>, table: Option<Vec<DoseDepthPoint>>) -> DoseLegInput {
    DoseLegInput {
        phase: Some(phase.to_owned()),
        duration: years.map(|y| Qty::from_raw(y * YEAR)),
        environment: table,
        environment_source: None,
    }
}

fn part(id: &str, mm: Option<f64>, tolerance_gy: Option<f64>) -> DosePartInput {
    DosePartInput {
        part: id.to_owned(),
        shielding: mm.map(|m| Qty::from_raw(m * 1e-3)),
        tolerance: tolerance_gy.map(Qty::from_raw),
    }
}

fn flat(gy_per_year: f64) -> Vec<DoseDepthPoint> {
    vec![point(1.0, gy_per_year), point(10.0, gy_per_year)]
}

fn input(legs: Vec<DoseLegInput>, parts: Vec<DosePartInput>) -> TotalDoseInput {
    TotalDoseInput {
        legs,
        parts,
        default_shielding: None,
        design_margin: None,
    }
}

fn close(a: f64, b: f64) {
    assert!((a - b).abs() <= 1e-12 * b.abs().max(1.0), "{a} vs {b}");
}

/// 一定の線量率 10 Gy/yr を 2 年 → 20 Gy。許容 100 Gy・RDM 2 → 100 / (20 × 2) − 1 = 1.5。
#[test]
fn a_constant_rate_gives_rate_times_duration_and_the_margin_follows() {
    let mut i = input(
        vec![leg("MISSION", Some(2.0), Some(flat(10.0)))],
        vec![part("OBC", Some(5.0), Some(100.0))],
    );
    i.design_margin = Some(2.0);
    let o = TotalDose::run(&i).unwrap();
    let p = &o.parts[0];
    close(p.total.unwrap().value(), 20.0);
    close(p.margin.unwrap().value(), 1.5);
    assert_eq!(p.verdict, Some(Verdict::Ok));
    assert_eq!(o.design_margin_source, DesignMarginSource::Given);
    assert_eq!(o.worst.as_ref().unwrap().part, "OBC");
}

/// 2 フェーズは足す: 10 Gy/yr × 2 yr + 4 Gy/yr × 0.5 yr = 22 Gy。RDM の既定 2 → 100 / 44 − 1。
#[test]
fn phases_add_and_the_default_design_margin_is_two() {
    let i = input(
        vec![
            leg("A", Some(2.0), Some(flat(10.0))),
            leg("B", Some(0.5), Some(flat(4.0))),
        ],
        vec![part("OBC", Some(5.0), Some(100.0))],
    );
    let o = TotalDose::run(&i).unwrap();
    close(o.parts[0].total.unwrap().value(), 22.0);
    close(o.parts[0].margin.unwrap().value(), 100.0 / 44.0 - 1.0);
    assert_eq!(o.design_margin, 2.0);
    assert_eq!(o.design_margin_source, DesignMarginSource::Default);
    assert_eq!(o.parts[0].phases.len(), 2);
}

/// 表の点の上では表の値、中点では幾何平均: 1 mm = 100, 3 mm = 4 Gy/yr → 2 mm で sqrt(400) = 20 Gy/yr（1 年）。
#[test]
fn the_table_is_read_log_linearly_and_not_extrapolated() {
    let table = vec![point(1.0, 100.0), point(3.0, 4.0)];
    let one = |mm| {
        let i = input(
            vec![leg("A", Some(1.0), Some(table.clone()))],
            vec![part("P", Some(mm), None)],
        );
        TotalDose::run(&i).unwrap().parts.remove(0)
    };
    close(one(1.0).total.unwrap().value(), 100.0);
    close(one(3.0).total.unwrap().value(), 4.0);
    close(one(2.0).total.unwrap().value(), 20.0);
    // 表の外は評価しない（端の値を使わない）。警告の判定を出す
    let outside = one(5.0);
    assert_eq!(outside.not_evaluated, Some(DoseNotEvaluated::OutsideTable));
    assert!(outside.total.is_none() && outside.verdict.is_none());
    let o = TotalDose::run(&input(
        vec![leg("A", Some(1.0), Some(table.clone()))],
        vec![part("P", Some(0.5), Some(10.0)), part("Q", Some(2.0), None)],
    ))
    .unwrap();
    assert!(
        o.checks
            .iter()
            .any(|c| c.key == "dose:P" && c.verdict == Verdict::Warning)
    );
}

/// 遮蔽が無く既定も無い部品は評価しない。既定があればそれを使い、出どころを書く。許容線量が無い部品は線量だけ。
#[test]
fn shielding_defaults_and_missing_tolerance() {
    let legs = vec![leg("A", Some(1.0), Some(flat(10.0)))];
    let parts = vec![
        part("BARE", None, Some(50.0)),
        part("NOTOL", Some(2.0), None),
    ];
    let o = TotalDose::run(&input(legs.clone(), parts.clone())).unwrap();
    assert_eq!(
        o.parts[0].not_evaluated,
        Some(DoseNotEvaluated::NoShielding)
    );
    close(o.parts[1].total.unwrap().value(), 10.0);
    assert!(o.parts[1].verdict.is_none() && o.parts[1].margin.is_none());
    assert!(o.checks.iter().any(|c| c.key == "dose:BARE"));
    assert!(!o.checks.iter().any(|c| c.key == "dose:NOTOL"));
    let mut i = input(legs, parts);
    i.default_shielding = Some(Qty::from_raw(0.004));
    let o = TotalDose::run(&i).unwrap();
    assert_eq!(o.parts[0].shielding_source, Some(ShieldingSource::Default));
    close(o.parts[0].total.unwrap().value(), 10.0);
    assert_eq!(o.parts[1].shielding_source, Some(ShieldingSource::Part));
    // 基準の遮蔽は評価した部品のうち最も薄いもの（2 mm）
    close(o.reference_shielding.unwrap().value(), 0.002);
}

/// 期間か環境の表が無い区間は挙げて総線量から除き、警告する。使える区間が無ければ入力の誤り。
#[test]
fn legs_without_a_duration_or_an_environment_are_listed_not_guessed() {
    let i = input(
        vec![
            leg("A", Some(1.0), Some(flat(10.0))),
            leg("NODUR", None, Some(flat(99.0))),
            leg("NOENV", Some(5.0), None),
        ],
        vec![part("P", Some(2.0), None)],
    );
    let o = TotalDose::run(&i).unwrap();
    let status: Vec<_> = o.legs.iter().map(|l| l.status).collect();
    assert_eq!(
        status,
        [
            DoseLegStatus::Evaluated,
            DoseLegStatus::NoDuration,
            DoseLegStatus::NoEnvironment
        ]
    );
    close(o.parts[0].total.unwrap().value(), 10.0);
    assert!(o.checks.iter().any(|c| c.key == "phases_left_out"));
    let none = input(vec![leg("X", None, None)], vec![part("P", Some(2.0), None)]);
    assert!(TotalDose::run(&none).is_err());
}

#[test]
fn bad_tables_are_rejected() {
    let bad = |table| {
        TotalDose::run(&input(
            vec![leg("A", Some(1.0), Some(table))],
            vec![part("P", Some(2.0), None)],
        ))
        .is_err()
    };
    assert!(bad(vec![point(3.0, 1.0), point(1.0, 2.0)]));
    assert!(bad(vec![point(1.0, 0.0)]));
    assert!(bad(vec![]));
}

/// 基準の遮蔽（最も薄い部品）での区間ごとの線量と、余裕が最小の部品。
#[test]
fn the_worst_part_has_the_smallest_margin() {
    let i = input(
        vec![leg(
            "A",
            Some(1.0),
            Some(vec![point(1.0, 100.0), point(3.0, 4.0)]),
        )],
        vec![
            part("THIN", Some(1.0), Some(1000.0)),
            part("THICK", Some(3.0), Some(1000.0)),
        ],
    );
    let o = TotalDose::run(&i).unwrap();
    assert_eq!(o.worst.as_ref().unwrap().part, "THIN");
    close(o.legs[0].dose_at_reference.unwrap().value(), 100.0);
    // 1000 / (100 × 2) − 1 = 4
    close(o.worst.unwrap().margin.value(), 4.0);
}

#[test]
fn a_dose_over_the_tolerance_is_a_violation() {
    let i = input(
        vec![leg("A", Some(1.0), Some(flat(10.0)))],
        vec![part("P", Some(2.0), Some(15.0))],
    );
    let o = TotalDose::run(&i).unwrap();
    // 15 / (10 × 2) − 1 = −0.25
    close(o.parts[0].margin.unwrap().value(), -0.25);
    assert_eq!(o.parts[0].verdict, Some(Verdict::Violation));
}

/// 環境の表の出どころ: 無指定は利用者の表、計算した表なら `ComputedTrapped` が出力と警告の判定に出る。表の無い区間は `None`。
#[test]
fn environment_source_is_reported_per_leg_and_computed_tables_are_flagged() {
    let table = vec![
        DoseDepthPoint {
            shielding: Qty::from_raw(1.0e-3),
            rate: Qty::from_raw(1.0e-6),
        },
        DoseDepthPoint {
            shielding: Qty::from_raw(5.0e-3),
            rate: Qty::from_raw(1.0e-7),
        },
    ];
    let mut computed = leg("transfer", Some(1.0), Some(table.clone()));
    computed.environment_source = Some(EnvironmentSource::ComputedTrapped);
    let legs = vec![
        leg("user", Some(1.0), Some(table)),
        computed,
        leg("none", Some(1.0), None),
    ];
    let out = TotalDose::run(&input(legs, vec![part("a", Some(2.0), Some(100.0))])).expect("run");
    let sources: Vec<_> = out.legs.iter().map(|l| l.environment_source).collect();
    assert_eq!(
        sources,
        [
            Some(EnvironmentSource::UserTable),
            Some(EnvironmentSource::ComputedTrapped),
            None
        ]
    );
    let check = out
        .checks
        .iter()
        .find(|c| c.key == "environment_source")
        .expect("check");
    assert!(
        check.message.contains("transfer") && !check.message.contains("(user"),
        "{}",
        check.message
    );
    assert!(check.message.contains("no solar energetic protons"));
    // 利用者の表だけなら、この判定は出ない
    let table = vec![
        DoseDepthPoint {
            shielding: Qty::from_raw(1.0e-3),
            rate: Qty::from_raw(1.0e-6),
        },
        DoseDepthPoint {
            shielding: Qty::from_raw(5.0e-3),
            rate: Qty::from_raw(1.0e-7),
        },
    ];
    let out = TotalDose::run(&input(
        vec![leg("u", Some(1.0), Some(table))],
        vec![part("a", Some(2.0), None)],
    ))
    .expect("run");
    assert!(out.checks.iter().all(|c| c.key != "environment_source"));
}
