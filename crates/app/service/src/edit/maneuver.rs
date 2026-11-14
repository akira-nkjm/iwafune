//! `fune maneuver add|set` に打った引数 → `CreateManeuver`・`SetManeuver`（`POST /ops/from_command` の `maneuver_add`・
//! `maneuver_set`）。**マヌーバの旗の読み方はここだけ**（逆の向き = 編集の欄の初期値は `query::maneuvers::maneuver_text`）。

use super::text::{given, invalid, quantity_from_text};
use crate::dto::ManeuverText;
use crate::error::Result;
use fune_model::design::{Maneuver, ManeuverConstraints, ManeuverTarget};
use fune_model::value::Quantity;
use fune_ops::Op;

/// 打った引数 → `CreateManeuver`（`create`）か `SetManeuver`。
pub fn maneuver_op(t: &ManeuverText, create: bool) -> Result<Op> {
    let maneuver = maneuver_of(t)?;
    Ok(if create {
        Op::CreateManeuver { maneuver }
    } else {
        Op::SetManeuver { maneuver }
    })
}

fn quantity(text: &Option<String>) -> Result<Option<Quantity>> {
    given(text).map(quantity_from_text).transpose()
}

/// 真偽の打った文字列（`true` / `false`）。
fn flag(text: &Option<String>) -> Result<Option<bool>> {
    given(text)
        .map(|t| match t.trim() {
            "true" => Ok(true),
            "false" => Ok(false),
            other => Err(invalid(format!("true か false にしてください（{other}）"))),
        })
        .transpose()
}

fn maneuver_of(t: &ManeuverText) -> Result<Maneuver> {
    let owned = |x: &Option<String>| given(x).map(|s| s.trim().to_owned());
    Ok(Maneuver {
        id: t.id.trim().into(),
        label: given(&t.label).map(str::to_owned),
        category: t.category.trim().parse().map_err(invalid)?,
        method: t.method.trim().parse().map_err(invalid)?,
        thruster: owned(&t.thruster).map(Into::into),
        after: owned(&t.after),
        via_altitude: quantity(&t.via_altitude)?,
        target: ManeuverTarget {
            altitude: quantity(&t.altitude)?,
            perigee_altitude: quantity(&t.perigee_altitude)?,
            apogee_altitude: quantity(&t.apogee_altitude)?,
            inclination: quantity(&t.inclination)?,
        },
        constraints: ManeuverConstraints {
            sunlit_only: flag(&t.sunlit_only)?,
            mode: owned(&t.mode).map(Into::into),
        },
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::query::maneuvers::maneuver_text;

    fn text() -> ManeuverText {
        ManeuverText {
            id: "RAISE".into(),
            category: "insertion".into(),
            method: "low_thrust".into(),
            thruster: Some("EPT".into()),
            altitude: Some("600 km".into()),
            inclination: Some(" 97.8 deg".into()),
            sunlit_only: Some("true".into()),
            after: Some(String::new()),
            ..ManeuverText::default()
        }
    }

    #[test]
    fn typed_text_becomes_a_maneuver_and_back() {
        let Op::SetManeuver { maneuver } = maneuver_op(&text(), false).unwrap() else {
            panic!("SetManeuver ではありません");
        };
        assert_eq!(maneuver.target.altitude, Some(Quantity::new(600.0, "km")));
        assert_eq!(maneuver.constraints.sunlit_only, Some(true));
        // 空の文字列は書かない
        assert_eq!(maneuver.after, None);
        let back = maneuver_op(&maneuver_text(&maneuver), true).unwrap();
        assert_eq!(back, Op::CreateManeuver { maneuver });
    }

    #[test]
    fn bad_text_is_rejected() {
        let mut t = text();
        t.method = "warp".into();
        assert!(maneuver_op(&t, false).is_err());
        let mut t = text();
        t.sunlit_only = Some("yes".into());
        assert!(maneuver_op(&t, false).is_err());
        let mut t = text();
        t.altitude = Some("six hundred".into());
        assert!(maneuver_op(&t, false).is_err());
    }
}
