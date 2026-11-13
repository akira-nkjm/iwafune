//! `fune maneuver ls`（読み取り）。

use super::common::Handled;
use crate::output::{C, Out};
use fune_service::{Workspace, query};

/// `fune maneuver ls`（実行の順。HTTP の `GET /maneuvers` と同じ関数）
pub fn ls(ws: &Workspace, cmd: &crate::args::ManeuverCmd, out: &Out) -> Handled {
    if !matches!(cmd, crate::args::ManeuverCmd::Ls) {
        return Ok(None);
    }
    let maneuvers = query::maneuvers(ws)?;
    if out.json {
        out.json(&maneuvers)?;
        return Ok(Some(0));
    }
    let mut t = out.table(&["#", "ID", "名前", "区分", "方式", "スラスタ", "目標"]);
    for m in &maneuvers {
        let target: Vec<String> = m
            .target
            .iter()
            .map(|x| format!("{} {}", x.key, x.value.display))
            .collect();
        t.row(vec![
            C::Num(m.order.to_string()),
            C::Strong(m.id.clone()),
            C::Text(m.label.clone()),
            C::Text(query::enum_label("maneuver_category", &m.category)),
            C::Text(query::enum_label("maneuver_method", &m.method)),
            C::Text(m.thruster.clone().unwrap_or_default()),
            C::Dim(target.join("、")),
        ]);
    }
    t.print();
    Ok(Some(0))
}
