//! 系別のモード表示が「失敗」かどうかの判定（`fune mode view` の終了コード）。HTTP は `SystemModesView` のまま返すので、この関数は CLI が使う。

use crate::dto::{ModeValuesView, SystemModeView, SystemModesView};
use sat_analysis::spec::Verdict;

/// モード表示が失敗か: 入力が足りない、またはどれかの判定（日照中・蝕中の合計、電源の収支、AOCS の各解析）が違反。
/// 警告は失敗にしない。`--json` でも表でも同じ判定を使う。
pub fn fails(view: &SystemModesView) -> bool {
    !view.missing.is_empty() || view.modes.iter().any(mode_violates)
}

fn mode_violates(m: &SystemModeView) -> bool {
    let violates = |v: Option<Verdict>| v == Some(Verdict::Violation);
    let values = |v: &ModeValuesView| violates(v.verdict);
    values(&m.sunlit)
        || values(&m.eclipse)
        || m.power.as_ref().is_some_and(|p| violates(p.verdict))
        || m.aocs.as_ref().is_some_and(|a| {
            [&a.rw, &a.unloading, &a.sensor_fov]
                .iter()
                .any(|s| violates(s.verdict))
        })
}

#[cfg(test)]
mod tests {
    use super::fails;
    use crate::Workspace;
    use crate::dto::{SystemModesRequest, SystemModesView};
    use crate::system_modes::system_modes;
    use fune_projection::MissingInput;
    use sat_analysis::spec::{System, Verdict};

    fn demo_view() -> SystemModesView {
        let root =
            std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../../fixtures/demo-6u");
        let ws = Workspace::open(root).unwrap();
        let req = SystemModesRequest {
            ops: vec![],
            case: None,
            analyzed: false,
            summary: Default::default(),
        };
        let mut view = system_modes(&ws, System::Power, &req).unwrap();
        assert!(!view.modes.is_empty());
        for m in &mut view.modes {
            m.sunlit.verdict = Some(Verdict::Ok);
            m.eclipse.verdict = Some(Verdict::Warning);
        }
        view
    }

    #[test]
    fn ok_and_warning_do_not_fail() {
        assert!(!fails(&demo_view()));
    }

    #[test]
    fn a_violation_in_either_side_fails() {
        let mut v = demo_view();
        v.modes[0].sunlit.verdict = Some(Verdict::Violation);
        assert!(fails(&v));
        let mut v = demo_view();
        v.modes[0].eclipse.verdict = Some(Verdict::Violation);
        assert!(fails(&v));
    }

    #[test]
    fn missing_inputs_fail() {
        let mut v = demo_view();
        v.missing.push(MissingInput {
            fixes: Vec::new(),
            part: None,
            path: "p".into(),
            target: None,
            reason: "r".into(),
        });
        assert!(fails(&v));
    }
}
