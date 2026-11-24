//! 承知済みの組（`analysis/structure.toml` の `[[acknowledged]]`）の Op の解釈。足す・取り消す。
//! 部品と形態の実在・理由があること・干渉の組が別々の部品であることは、ここで先に確かめる（適用後の `check` も同じ内容を見る）。
//! 部品・形態の名前の変更は参照の索引（`RefSite::Acknowledged*`）が書き換える。

use crate::error::OpError;
use crate::op::Op;
use fune_model::Design;
use fune_model::design::{AcknowledgedFinding, AnalysisSystem, FindingKind};
use fune_store::Edits;
use fune_store::layout;
use fune_store::write::acknowledged;

pub(super) fn interpret(edits: &mut Edits, design: &Design, op: &Op) -> Result<(), OpError> {
    let file = layout::analysis_file(AnalysisSystem::Structure);
    match op {
        Op::Acknowledge {
            finding,
            first,
            second,
            configuration,
            reason,
        } => {
            let entry = AcknowledgedFinding {
                finding: *finding,
                first: first.clone(),
                second: second.clone(),
                configuration: configuration.clone(),
                reason: reason.trim().to_owned(),
            };
            validate(design, &entry)?;
            if entry.reason.is_empty() {
                return Err(OpError::invalid("An acknowledged pair needs a reason"));
            }
            Ok(acknowledged::acknowledge(edits, &file, &entry)?)
        }
        Op::Unacknowledge {
            finding,
            first,
            second,
            configuration,
        } => {
            let target = AcknowledgedFinding {
                finding: *finding,
                first: first.clone(),
                second: second.clone(),
                configuration: configuration.clone(),
                reason: String::new(),
            };
            if !acknowledged::unacknowledge(edits, &file, &target)? {
                return Err(OpError::not_found(format!(
                    "Acknowledged {finding} pair `{first}` / `{second}`{}",
                    configuration
                        .as_deref()
                        .map(|c| format!(" in configuration `{c}`"))
                        .unwrap_or_default()
                )));
            }
            Ok(())
        }
        _ => Err(OpError::invalid("This op is not handled here")),
    }
}

/// 部品と形態が実在し、干渉の組が別々の部品であること。
fn validate(design: &Design, entry: &AcknowledgedFinding) -> Result<(), OpError> {
    for part in [&entry.first, &entry.second] {
        if !design.parts.contains_key(part.as_str()) {
            return Err(OpError::not_found(format!("Part `{part}`")));
        }
    }
    if let Some(c) = &entry.configuration
        && !design.configurations.contains_key(c.as_str())
    {
        return Err(OpError::not_found(format!("Configuration `{c}`")));
    }
    if entry.finding == FindingKind::Interference && entry.first == entry.second {
        return Err(OpError::invalid(format!(
            "An overlap is between two different parts, but both are `{}`",
            entry.first
        )));
    }
    Ok(())
}
