//! `fune maneuver add|set` に打った引数 → `CreateManeuver`・`SetManeuver`（`POST /ops/from_command` の `maneuver_add`・
//! `maneuver_set`）。**マヌーバの旗の読み方はここだけ**（逆の向き = 編集の欄の初期値は `query::maneuvers::maneuver_text`）。

use super::text::{bool_from_text as flag, fraction_from_text, given, invalid, quantity_from_text};
use crate::dto::ManeuverText;
use crate::error::Result;
use fune_model::design::{
    DepartureBy, Maneuver, ManeuverConstraints, ManeuverTarget, ManeuverTransfer,
};
use fune_model::value::Quantity;
use fune_ops::Op;
use sat_astro::time::Epoch;

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
        delta_v: quantity(&t.delta_v)?,
        target: ManeuverTarget {
            body: given(&t.body)
                .map(|b| b.trim().parse())
                .transpose()
                .map_err(invalid)?,
            altitude: quantity(&t.altitude)?,
            perigee_altitude: quantity(&t.perigee_altitude)?,
            apogee_altitude: quantity(&t.apogee_altitude)?,
            inclination: quantity(&t.inclination)?,
        },
        constraints: ManeuverConstraints {
            sunlit_only: flag(&t.sunlit_only)?,
            mode: owned(&t.mode).map(Into::into),
            duty: given(&t.duty).map(fraction_from_text).transpose()?,
            steering: given(&t.steering)
                .map(|s| s.trim().parse().map_err(invalid))
                .transpose()?,
        },
        transfer: transfer_of(t)?,
    })
}

/// `--depart`・`--arrive`（両方か、どちらも書かない）と `--departure-by`（日付と一緒にだけ）→ `[transfer]`。日付の読み方は `sat_astro::time::Epoch::parse` だけ。
/// 日付どうしの順・暦の範囲・天体の組は検査（`fune check`）が見る。
fn transfer_of(t: &ManeuverText) -> Result<Option<ManeuverTransfer>> {
    let date = |flag: &str, text: &Option<String>| -> Result<Option<String>> {
        given(text)
            .map(|s| {
                let s = s.trim();
                Epoch::parse(s)
                    .map(|_| s.to_owned())
                    .map_err(|e| invalid(format!("--{flag}: {e}")))
            })
            .transpose()
    };
    let departure_by = given(&t.departure_by)
        .map(|s| s.trim().parse::<DepartureBy>().map_err(invalid))
        .transpose()?;
    match (date("depart", &t.depart)?, date("arrive", &t.arrive)?) {
        (None, None) if departure_by.is_some() => Err(invalid(
            "--departure-by needs --depart and --arrive (it belongs to a dated transfer)",
        )),
        (None, None) => Ok(None),
        (Some(departure), Some(arrival)) => Ok(Some(ManeuverTransfer {
            departure: Some(departure),
            arrival: Some(arrival),
            departure_by,
        })),
        _ => Err(invalid("--depart and --arrive must be given together")),
    }
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
            duty: Some("0.5".into()),
            steering: Some("inertial".into()),
            after: Some(String::new()),
            body: Some("moon".into()),
            ..ManeuverText::default()
        }
    }

    #[test]
    fn typed_text_becomes_a_maneuver_and_back() {
        let Op::SetManeuver { maneuver } = maneuver_op(&text(), false).unwrap() else {
            panic!("not SetManeuver");
        };
        assert_eq!(maneuver.target.altitude, Some(Quantity::new(600.0, "km")));
        assert_eq!(maneuver.constraints.sunlit_only, Some(true));
        assert_eq!(maneuver.constraints.duty, Some(0.5));
        assert_eq!(
            maneuver.constraints.steering,
            Some(fune_model::design::ManeuverSteering::Inertial)
        );
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
        t.duty = Some("half".into());
        assert!(maneuver_op(&t, false).is_err());
        let mut t = text();
        t.steering = Some("sideways".into());
        assert!(maneuver_op(&t, false).is_err());
        let mut t = text();
        t.altitude = Some("six hundred".into());
        assert!(maneuver_op(&t, false).is_err());
        let mut t = text();
        t.body = Some("pluto".into());
        assert!(maneuver_op(&t, false).is_err());
    }

    #[test]
    fn dates_come_in_pairs_and_round_trip() {
        let mut t = text();
        t.depart = Some(" 2028-12-10T00:00:00Z ".into());
        assert!(maneuver_op(&t, false).is_err(), "only one date");
        t.arrive = Some("2029-09-25T00:00:00Z".into());
        let Op::SetManeuver { maneuver } = maneuver_op(&t, false).unwrap() else {
            panic!("not SetManeuver");
        };
        let tr = maneuver.transfer.clone().unwrap();
        assert_eq!(tr.departure.as_deref(), Some("2028-12-10T00:00:00Z"));
        assert_eq!(tr.arrival.as_deref(), Some("2029-09-25T00:00:00Z"));
        let back = maneuver_op(&maneuver_text(&maneuver), true).unwrap();
        assert_eq!(back, Op::CreateManeuver { maneuver });
        // 空の文字列は書かない（編集で消す）
        t.depart = Some(String::new());
        t.arrive = Some(String::new());
        let Op::SetManeuver { maneuver } = maneuver_op(&t, false).unwrap() else {
            panic!("not SetManeuver");
        };
        assert_eq!(maneuver.transfer, None);
        t.depart = Some("next Tuesday".into());
        t.arrive = Some("2029-09-25T00:00:00Z".into());
        assert!(maneuver_op(&t, false).is_err());
    }
}
