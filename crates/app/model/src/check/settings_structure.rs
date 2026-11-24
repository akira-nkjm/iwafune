//! `analysis/structure.toml` の承知済みの組（`[[acknowledged]]`）の検査: 部品と形態が実在すること、理由があること、
//! 干渉の組が同じ部品どうしでないこと、同じ組・同じ形態の重複。承知した組が今の所見に当たるか（古い承知）は解析の結果が要るので、ここでは見ない
//! （service の `fune check` が解析を回して知らせる）。

use super::Ctx;
use super::settings::scope;
use crate::design::{AcknowledgedFinding, AnalysisSystem, FindingKind};

/// 承知済みの組の検査。
pub(super) fn check_structure(ctx: &mut Ctx) {
    let design = ctx.design;
    let Some(structure) = &design.analysis.structure else {
        return;
    };
    let mut s = scope(ctx, AnalysisSystem::Structure.setting_path());
    for (i, a) in structure.acknowledged.iter().enumerate() {
        let at = i.to_string();
        let keys = |key: &'static str| ["acknowledged", at.as_str(), key];
        for (key, part) in [("first", &a.first), ("second", &a.second)] {
            s.reference(
                &keys(key),
                design.parts.contains_key(part.as_str()),
                "ref.part",
                format!("Part `{part}`"),
            );
        }
        if let Some(c) = &a.configuration {
            s.reference(
                &keys("configuration"),
                design.configurations.contains_key(c.as_str()),
                "ref.configuration",
                format!("Configuration `{c}`"),
            );
        }
        if a.reason.trim().is_empty() {
            s.error(
                &keys("reason"),
                "acknowledged.reason",
                "An acknowledged pair needs a reason".into(),
            );
        }
        if a.finding == FindingKind::Interference && a.first == a.second {
            s.error(
                &keys("second"),
                "acknowledged.same_part",
                format!(
                    "An overlap is between two different parts, but both are `{}`",
                    a.first
                ),
            );
        }
        if structure.acknowledged[..i]
            .iter()
            .any(|b: &AcknowledgedFinding| b.same_target(a))
        {
            s.warning(
                &["acknowledged", at.as_str()],
                "acknowledged.duplicate",
                format!(
                    "The pair `{}` / `{}` ({}) is acknowledged more than once for the same configuration",
                    a.first, a.second, a.finding
                ),
            );
        }
    }
}
