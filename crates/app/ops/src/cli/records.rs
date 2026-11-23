//! 取付点・解析ケース・出典・ハーネス・マヌーバ・地上局の Op のコマンドと、定義の名前の変更（`fune attr rename`・`fune kind rename`）。

use super::text::Cmd;
use crate::op::Op;
use fune_model::design::{Case, Harness, Maneuver, Pose, Source};
use fune_model::value::{list_to_text, quantity_to_text, quaternion_to_text, vector_to_text};

/// 取付点・ケース・出典・ハーネスの Op のコマンド。該当しなければ None。
pub(super) fn record_op(op: &Op) -> Option<Vec<String>> {
    Some(vec![match op {
        Op::SetMountPoint { part, point, pose } => mount_point(part, point, pose.as_ref()),
        Op::SetCase { case } => case_cmd(case),
        Op::RemoveCase { id } => Cmd::new(&["fune", "case", "rm", id]).done(),
        Op::SetSource { source, dir } => source_cmd(source, dir.as_deref()),
        Op::RemoveSource { id } => Cmd::new(&["fune", "source", "rm", id]).done(),
        Op::MoveSource { id, dir } => Cmd::new(&["fune", "source", "mv", id, dir]).done(),
        Op::SetHarness { harness } => harness_cmd(harness),
        Op::RemoveHarness { id } => Cmd::new(&["fune", "harness", "rm", id]).done(),
        Op::CreateManeuver { maneuver } => maneuver_cmd("add", maneuver),
        Op::SetManeuver { maneuver } => maneuver_cmd("set", maneuver),
        Op::RemoveManeuver { id } => Cmd::new(&["fune", "maneuver", "rm", id]).done(),
        Op::CreateGroundStation { .. } => ground_add_cmd(op),
        Op::RemoveGroundStation { id } => Cmd::new(&["fune", "ground", "rm", id]).done(),
        Op::RenameGroundStation { id, to } => {
            Cmd::new(&["fune", "ground", "rename", id, to]).done()
        }
        Op::RenameAttribute { id, to } => Cmd::new(&["fune", "attr", "rename", id, to]).done(),
        Op::RenameKind { id, to } => Cmd::new(&["fune", "kind", "rename", id, to]).done(),
        _ => return None,
    }])
}

/// `fune ground add <ID> [--label …] [--latitude …] [--longitude …] [--altitude …] [--min-elevation …]`
fn ground_add_cmd(op: &Op) -> String {
    let Op::CreateGroundStation {
        id,
        label,
        latitude,
        longitude,
        altitude,
        min_elevation,
    } = op
    else {
        return String::new();
    };
    let text = |q: &Option<fune_model::value::Quantity>| q.as_ref().map(quantity_to_text);
    Cmd::new(&["fune", "ground", "add", id])
        .flag("label", label.as_deref())
        .flag("latitude", text(latitude).as_deref())
        .flag("longitude", text(longitude).as_deref())
        .flag("altitude", text(altitude).as_deref())
        .flag("min-elevation", text(min_elevation).as_deref())
        .done()
}

fn mount_point(part: &str, point: &str, pose: Option<&Pose>) -> String {
    let cmd = Cmd::new(&["fune", "part", "point", part, point]);
    let Some(pose) = pose else {
        return cmd.switch("rm", true).done();
    };
    let pos = pose.position.as_ref().map(vector_to_text);
    let ori = pose.orientation.as_ref().map(quaternion_to_text);
    cmd.flag("position", pos.as_deref())
        .flag("orientation", ori.as_deref())
        .done()
}

fn case_cmd(case: &Case) -> String {
    let beta = case.beta.as_ref().map(quantity_to_text);
    let orbits = case.orbits.map(|o| o.to_string());
    let sun_distance = case.sun_distance.as_ref().map(quantity_to_text);
    Cmd::new(&["fune", "case", "set", case.id.as_str()])
        .flag("label", case.label.as_deref())
        .flag("description", case.description.as_deref())
        .flag("mode", case.mode.as_ref().map(|m| m.as_str()))
        .flag("phase", case.phase.as_ref().map(|p| p.as_str()))
        .flag("side", case.side.map(|s| s.as_str()))
        .flag("thermal-case", case.thermal_case.as_deref())
        .flag("beta", beta.as_deref())
        .flag("orbits", orbits.as_deref())
        .flag("sun-distance", sun_distance.as_deref())
        .done()
}

fn source_cmd(source: &Source, dir: Option<&str>) -> String {
    Cmd::new(&["fune", "source", "set", source.id.as_str()])
        .flag("kind", Some(source.kind.as_str()))
        .flag("label", source.label.as_deref())
        .flag("ref", source.reference.as_deref())
        .flag("revision", source.revision.as_deref())
        .flag("retrieved", source.retrieved.as_deref())
        .flag("sha256", source.sha256.as_deref())
        .flag("distribution", source.distribution.map(|d| d.as_str()))
        .flag("date", source.date.as_deref())
        .flag("by", source.by.as_deref())
        .flag("note", source.note.as_deref())
        .flag("subdir", dir)
        .done()
}

fn harness_cmd(h: &Harness) -> String {
    let nets = (!h.nets.is_empty()).then(|| list_to_text(h.nets.iter().map(|n| n.as_str())));
    let cores = h.cores.map(|c| c.to_string());
    let length = h.length.as_ref().map(quantity_to_text);
    Cmd::new(&["fune", "harness", "set", h.id.as_str()])
        .flag("from", Some(&h.from.to_string()))
        .flag("to", Some(&h.to.to_string()))
        .flag("nets", nets.as_deref())
        .flag("wire", h.wire.as_deref())
        .flag("cores", cores.as_deref())
        .flag("length", length.as_deref())
        .flag("label", h.label.as_deref())
        .done()
}

/// `fune maneuver add|set <ID> --category … --method … --altitude "600 km" …`（旗は `fune_service` の `ManeuverText` と 1 対 1）。
fn maneuver_cmd(verb: &str, m: &Maneuver) -> String {
    let q = |x: &Option<fune_model::value::Quantity>| x.as_ref().map(quantity_to_text);
    let t = &m.target;
    let sunlit = m.constraints.sunlit_only.map(|b| b.to_string());
    let duty = m.constraints.duty.map(|d| d.to_string());
    Cmd::new(&["fune", "maneuver", verb, m.id.as_str()])
        .flag("category", Some(m.category.as_str()))
        .flag("method", Some(m.method.as_str()))
        .flag("label", m.label.as_deref())
        .flag("thruster", m.thruster.as_ref().map(|p| p.as_str()))
        .flag("after", m.after.as_deref())
        .flag("via-altitude", q(&m.via_altitude).as_deref())
        .flag("body", t.body.map(|b| b.as_str()))
        .flag("altitude", q(&t.altitude).as_deref())
        .flag("perigee-altitude", q(&t.perigee_altitude).as_deref())
        .flag("apogee-altitude", q(&t.apogee_altitude).as_deref())
        .flag("inclination", q(&t.inclination).as_deref())
        .flag("sunlit-only", sunlit.as_deref())
        .flag("mode", m.constraints.mode.as_ref().map(|x| x.as_str()))
        .flag("duty", duty.as_deref())
        .flag("steering", m.constraints.steering.map(|s| s.as_str()))
        .flag(
            "depart",
            m.transfer.as_ref().and_then(|t| t.departure.as_deref()),
        )
        .flag(
            "arrive",
            m.transfer.as_ref().and_then(|t| t.arrival.as_deref()),
        )
        .done()
}
