//! マヌーバ → 表示（実行の順。`Design::ordered_maneuvers`）。

use super::values::authored_quantity;
use super::views::file_of;
use crate::dto::{ManeuverTargetView, ManeuverView};

use crate::dto::ManeuverText;
use fune_model::Target;
use fune_model::design::{Design, EntityKey, Maneuver, ManeuverTarget};
use fune_model::value::{Quantity, quantity_to_text};
use std::collections::BTreeMap;

/// 編集の欄（`ManeuverText` の欄）→ 対象のキー（`Target::maneuver` の `field`。検査が付ける対象と同じ）。
const FIELDS: &[(&str, &str)] = &[
    ("category", "category"),
    ("method", "method"),
    ("label", "label"),
    ("thruster", "thruster"),
    ("after", "after"),
    ("via_altitude", "via_altitude"),
    ("delta_v", "delta_v"),
    ("body", "target.body"),
    ("altitude", "target.altitude"),
    ("perigee_altitude", "target.perigee_altitude"),
    ("apogee_altitude", "target.apogee_altitude"),
    ("inclination", "target.inclination"),
    ("sunlit_only", "constraints.sunlit_only"),
    ("mode", "constraints.mode"),
    ("duty", "constraints.duty"),
    ("steering", "constraints.steering"),
    ("depart", "transfer.departure"),
    ("arrive", "transfer.arrival"),
    ("departure_by", "transfer.departure_by"),
];

/// マヌーバの一覧（実行の順）。
pub(crate) fn maneuvers(design: &Design) -> Vec<ManeuverView> {
    design
        .ordered_maneuvers()
        .into_iter()
        .zip(1u32..)
        .map(|(m, order)| view(design, m, order))
        .collect()
}

fn view(design: &Design, m: &Maneuver, order: u32) -> ManeuverView {
    let id = m.id.as_str();
    ManeuverView {
        id: id.to_owned(),
        order,
        label: m.label.clone().unwrap_or_else(|| id.to_owned()),
        category: m.category,
        method: m.method,
        thruster: m.thruster.as_ref().map(ToString::to_string),
        after: m.after.clone(),
        body: m.target.body,
        via_altitude: m
            .via_altitude
            .as_ref()
            .map(|q| authored_quantity(q, Some("Length"))),
        delta_v: m
            .delta_v
            .as_ref()
            .map(|q| authored_quantity(q, Some("Velocity"))),
        target: m
            .target
            .entries()
            .into_iter()
            .filter_map(|(key, q)| {
                Some(ManeuverTargetView {
                    key: key.to_owned(),
                    value: authored_quantity(q?, Some(ManeuverTarget::dimension(key))),
                })
            })
            .collect(),
        constraints: m
            .constraints
            .keys()
            .into_iter()
            .map(str::to_owned)
            .collect(),
        text: maneuver_text(m),
        field_paths: FIELDS
            .iter()
            .map(|(field, key)| {
                let target = Target::maneuver(id, Some(key));
                ((*field).to_owned(), target.to_string())
            })
            .collect::<BTreeMap<_, _>>(),
        file: file_of(design, EntityKey::Maneuver(id.to_owned())),
    }
}
/// 今のマヌーバ → 打った引数（`edit::maneuver_op` で読み直すと同じマヌーバになる。GUI の編集の欄の初期値）。
pub(crate) fn maneuver_text(m: &Maneuver) -> ManeuverText {
    let q = |x: &Option<Quantity>| x.as_ref().map(quantity_to_text);
    let t = &m.target;
    ManeuverText {
        id: m.id.to_string(),
        category: m.category.as_str().to_owned(),
        method: m.method.as_str().to_owned(),
        label: m.label.clone(),
        thruster: m.thruster.as_ref().map(ToString::to_string),
        after: m.after.clone(),
        via_altitude: q(&m.via_altitude),
        delta_v: q(&m.delta_v),
        body: t.body.map(|b| b.as_str().to_owned()),
        altitude: q(&t.altitude),
        perigee_altitude: q(&t.perigee_altitude),
        apogee_altitude: q(&t.apogee_altitude),
        inclination: q(&t.inclination),
        sunlit_only: m.constraints.sunlit_only.map(|b| b.to_string()),
        mode: m.constraints.mode.as_ref().map(ToString::to_string),
        duty: m.constraints.duty.map(|d| d.to_string()),
        steering: m.constraints.steering.map(|s| s.as_str().to_owned()),
        depart: m.transfer.as_ref().and_then(|t| t.departure.clone()),
        arrive: m.transfer.as_ref().and_then(|t| t.arrival.clone()),
        departure_by: m
            .transfer
            .as_ref()
            .and_then(|t| t.departure_by)
            .map(|d| d.as_str().to_owned()),
    }
}
