//! 承知済みの組の射影（`structure.interference` と `structure.cone_clearance` に共通）。
//! 設計の `[[acknowledged]]`（`analysis/structure.toml`）から、その所見の種類で、いま見ている配置に効くものだけを解析の入力にする。
//! 形態を指した承知は、その形態を使っているときだけ効く（形態を指さない承知は、基準の配置を含めどの配置にも効く）。

use crate::context::Context;
use fune_model::design::FindingKind;
use sat_analysis::structure::acknowledged::Acknowledgement;

/// `kind` の所見の承知のうち、`ctx` の配置（解析が使う形態。無ければ基準）に効くもの。
pub fn acknowledgements(ctx: &Context<'_>, kind: FindingKind) -> Vec<Acknowledgement> {
    let Some(structure) = &ctx.design.analysis.structure else {
        return Vec::new();
    };
    let current = ctx.configuration().map(|(id, _)| id);
    structure
        .acknowledged
        .iter()
        .filter(|a| a.finding == kind)
        .filter(|a| {
            a.configuration
                .as_deref()
                .is_none_or(|c| Some(c) == current)
        })
        .map(|a| Acknowledgement {
            first: a.first.clone(),
            second: a.second.clone(),
            reason: a.reason.clone(),
            configuration: a.configuration.clone(),
        })
        .collect()
}
