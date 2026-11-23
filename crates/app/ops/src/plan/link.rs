//! 通信の回線（`analysis/comm.toml` の `[link.<名前>]`）の Op の解釈。足す・消す・名前を変える。値の書き換えは設定の書き込み
//! （`SetSetting`）。足すときは読み込める最小の 3 つ（向き・部品・帯域）を書く。部品・帯域の実在は最後の検査（`ref.part`・`ref.band`）が見る。
//! 消すときは、要求の判定のキーが名指していれば拒む（`data_downlink` がその回線なら外す）。名前の変更はそのキーも書き換える。

use super::check_keys::refuse_if_checked_by_requirements;
use super::others::new_flat_id;
use crate::error::OpError;
use crate::op::Op;
use fune_model::Design;
use fune_model::design::Referent;
use fune_store::Edits;
use fune_store::write::{link, refs};

pub(super) fn interpret(edits: &mut Edits, design: &Design, op: &Op) -> Result<(), OpError> {
    match op {
        Op::CreateCommLink {
            id,
            direction,
            part,
            band,
        } => {
            new_link_id(design, id)?;
            for (what, value) in [("part", part), ("band", band)] {
                if value.trim().is_empty() {
                    return Err(OpError::invalid(format!("The {what} of the link is empty")));
                }
            }
            Ok(link::create_link(edits, id, *direction, part, band)?)
        }
        Op::RemoveCommLink { id } => {
            existing_link(design, id)?;
            refuse_if_checked_by_requirements(
                "Link",
                id,
                &design.references(Referent::Link(id)),
                "deleted",
            )?;
            link::remove_link(edits, id)?;
            Ok(())
        }
        Op::RenameCommLink { id, to } => {
            existing_link(design, id)?;
            new_link_id(design, to)?;
            Ok(refs::rename_link(edits, design, id, to)?)
        }
        _ => Err(OpError::invalid("Not a link op")),
    }
}

fn existing_link(design: &Design, id: &str) -> Result<(), OpError> {
    let found = design
        .analysis
        .comm
        .as_ref()
        .is_some_and(|c| c.links.contains_key(id));
    if found {
        Ok(())
    } else {
        Err(OpError::not_found(format!("Link `{id}`")))
    }
}

/// 新しい回線の名前として使えるか（ID の規則と、大文字小文字を無視した一意性）。
fn new_link_id(design: &Design, id: &str) -> Result<(), OpError> {
    let existing = design
        .analysis
        .comm
        .iter()
        .flat_map(|c| c.links.keys().map(String::as_str));
    new_flat_id(existing, id, "Link")
}
