//! 地上局（`conditions/ground/<ID>.toml`）の Op の解釈。足す・消す・名前を変える。値の書き換えは設定の書き込み（`SetSetting`）。
//! 消すときは、通信の解析の `stations` に挙げられているか、要求の判定のキーが名指していれば拒む（`stations` が空なら全局を使うので、
//! どの局も参照されていない）。名前の変更は、`stations` と要求の判定のキーを書き換える。

use super::check_keys::refuse_if_checked_by_requirements;
use super::others::new_flat_id;
use crate::error::OpError;
use crate::op::Op;
use fune_model::Design;
use fune_model::design::{GroundStationSettings, Referent};
use fune_store::Edits;
use fune_store::layout;
use fune_store::write::{ground, refs};

pub(super) fn interpret(edits: &mut Edits, design: &Design, op: &Op) -> Result<(), OpError> {
    match op {
        Op::CreateGroundStation {
            id,
            label,
            latitude,
            longitude,
            altitude,
            min_elevation,
        } => {
            new_station_id(design, id)?;
            let station = GroundStationSettings {
                id: id.clone(),
                label: label.clone(),
                latitude: latitude.clone(),
                longitude: longitude.clone(),
                altitude: altitude.clone(),
                min_elevation: min_elevation.clone(),
                links: Default::default(),
            };
            Ok(edits.create(&layout::ground_file(id), ground::new_ground_text(&station)?)?)
        }
        Op::RemoveGroundStation { id } => {
            existing_station(design, id)?;
            let sites = design.references(Referent::Station(id));
            refuse_if_checked_by_requirements("Ground station", id, &sites, "deleted")?;
            if !sites.is_empty() {
                return Err(OpError::invalid(format!(
                    "Ground station `{id}` is listed in `stations` of analysis/comm.toml and cannot be deleted. \
                     Remove it from analysis.comm.stations first (fune set analysis.comm.stations …)"
                )));
            }
            edits.remove(&layout::ground_file(id))?;
            Ok(())
        }
        Op::RenameGroundStation { id, to } => {
            existing_station(design, id)?;
            new_station_id(design, to)?;
            // 要求の判定のキー（`margin:<回線>@<局>`・`link_closes:<局>`）は名前の変更で書き換わる（`refs::rename_station`）
            Ok(refs::rename_station(edits, design, id, to)?)
        }
        _ => Err(OpError::invalid("Not a ground station op")),
    }
}

fn existing_station(design: &Design, id: &str) -> Result<(), OpError> {
    if design.conditions.ground.contains_key(id) {
        Ok(())
    } else {
        Err(OpError::not_found(format!("Ground station `{id}`")))
    }
}

/// 新しい地上局の ID として使えるか（ID の規則と、大文字小文字を無視した一意性）。
fn new_station_id(design: &Design, id: &str) -> Result<(), OpError> {
    new_flat_id(
        design.conditions.ground.keys().map(String::as_str),
        id,
        "Ground station",
    )
}
