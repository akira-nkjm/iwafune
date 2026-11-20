//! 書ける設定が解析に効くか、効かないことが検査で分かること（#162）: 伝搬の種類の写し替えと、使わない設定の警告。

use fune_model::Severity;
use fune_model::design::{PropagatorKind, UncertaintySide};
use fune_model::{Design, Resolved};
use fune_projection::{Context, MissingInputs};
use sat_astro::orbit::Propagator;
use std::path::Path;

fn demo() -> Design {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../../fixtures/demo-6u");
    fune_store::load(&root)
        .expect("the demo should load")
        .design
}

fn propagator_of(kind: PropagatorKind) -> Propagator {
    let mut design = demo();
    design.conditions.orbit.as_mut().expect("orbit").propagator = Some(kind);
    let resolved: Resolved = fune_model::resolve(&design);
    let ctx = Context::bare(&design, &resolved);
    ctx.orbit(&mut MissingInputs::new())
        .expect("orbit")
        .propagator
}

/// kepler と j2 は書いたとおりに伝搬する。numerical は解析に使わず J2 になる（使わないことは check が知らせる）。
#[test]
fn propagator_kinds_map_to_the_astro_propagator() {
    assert_eq!(propagator_of(PropagatorKind::Kepler), Propagator::Kepler);
    assert_eq!(propagator_of(PropagatorKind::J2), Propagator::J2);
    assert_eq!(propagator_of(PropagatorKind::Numerical), Propagator::J2);
}

fn codes(design: &Design) -> Vec<Severity> {
    fune_model::check(design)
        .into_iter()
        .filter(|d| d.code == "setting.not_applied")
        .map(|d| d.severity)
        .collect()
}

/// 使わない値を書くと `check` が知らせる: numerical は warning。side は解析に効くので、nominal 以外でもここには出ない。
#[test]
fn unapplied_settings_are_reported_by_check() {
    let mut design = demo();
    assert!(codes(&design).is_empty());
    design.conditions.orbit.as_mut().unwrap().propagator = Some(PropagatorKind::Numerical);
    assert_eq!(codes(&design), [Severity::Warning]);
    design.cases.values_mut().next().unwrap().side = Some(UncertaintySide::Max);
    assert_eq!(codes(&design), [Severity::Warning]);
}

fn side_findings(design: &Design) -> usize {
    fune_model::check(design)
        .iter()
        .filter(|d| d.code == "case.side_without_bounds")
        .count()
}

/// min / max の側を読むケースがあるのに、その側を持つ値が設計に無ければ info。デモはホイールの電力が持つので出ない。
#[test]
fn side_without_any_bound_is_reported() {
    let mut design = demo();
    assert_eq!(side_findings(&design), 0);
    for item in design.catalog.values_mut() {
        for a in item.attr.values_mut() {
            match &mut a.value {
                fune_model::value::Value::ByState(m) => {
                    for s in m.values_mut() {
                        s.value.min = None;
                        s.value.max = None;
                    }
                }
                fune_model::value::Value::Scalar(s) => {
                    s.min = None;
                    s.max = None;
                }
                _ => {}
            }
        }
    }
    // demo の side は max が 2 件・min が 1 件
    assert_eq!(side_findings(&design), 3);
}
