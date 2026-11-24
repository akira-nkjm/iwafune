//! 日付つきの惑星間遷移（マヌーバの `[transfer]`）の日付の読み方と検査。
//!
//! 暦（`sat_astro::ephem`）と日付の読み方（`sat_astro::time::Epoch`）を持つのは sat-astro で、fune-model は
//! sat-astro に依存しない。そのため、日付が読めるか・暦の範囲か・出発が到着より前か・遷移の両端が兄弟の天体か
//! の検査はここが持つ（`fune check` の診断は service が model の検査にこれを足す。`transfer` が書かれているか・
//! 目標の天体があるかは model の `check::maneuvers`）。射影（`propulsion::maneuver`）も同じ読み方を使う。

use crate::orbit_central::{central_body, design_orbit_central};
use fune_model::design::{EntityKey, Maneuver};
use fune_model::{Design, Diagnostic, Target};
use sat_astro::ephem::{CentralBody, covers};
use sat_astro::time::Epoch;

/// 読んだ出発・到着の日付。
pub type Dates = (Epoch, Epoch);

/// `[transfer]` の日付を読む（どちらかが書かれていない・読めないときは `Err`。理由は英語の 1 文と、そのキー）。
pub fn parse(m: &Maneuver) -> Option<Result<Dates, (&'static str, String)>> {
    let tr = m.transfer.as_ref()?;
    let read =
        |key: &'static str, text: &Option<String>| -> Result<Epoch, (&'static str, String)> {
            let text = text
                .as_deref()
                .map(str::trim)
                .filter(|t| !t.is_empty())
                .ok_or((key, format!("The {key} date is not set")))?;
            Epoch::parse(text).map_err(|e| (key, format!("Cannot parse the {key} date ({e})")))
        };
    Some(read("departure", &tr.departure).and_then(|d| Ok((d, read("arrival", &tr.arrival)?))))
}

/// `[[transfer.flybys]]` の天体と日付を読む（日付の順の並びのまま。読めない通過は `Err` に理由の英語の 1 文）。
pub fn parse_flybys(m: &Maneuver) -> Vec<Result<(CentralBody, Epoch), String>> {
    let Some(tr) = m.transfer.as_ref() else {
        return Vec::new();
    };
    tr.flybys
        .iter()
        .enumerate()
        .map(|(i, f)| {
            let text = f.date.trim();
            Epoch::parse(text)
                .map(|d| (central_body(f.body), d))
                .map_err(|e| format!("Cannot parse the date of flyby {} ({e})", i + 1))
        })
        .collect()
}

/// マヌーバの前の中心天体（`after` の鎖をたどり、目標の天体を書いたマヌーバで替わる。始めは最初の軌道の中心天体）。
pub fn central_before(design: &Design, m: &Maneuver) -> CentralBody {
    let mut chain = Vec::new();
    let mut at = m.after.as_deref();
    while let Some(prev) = at.and_then(|a| design.maneuvers.get(a)) {
        if chain.len() > design.maneuvers.len() {
            break; // 循環（`maneuver.cycle` が誤りにする）
        }
        chain.push(prev);
        at = prev.after.as_deref();
    }
    // 始めの軌道の中心天体は、設計の軌道の読み手（`context`）に聞く（出どころ ごとに決め方が違う）
    let initial = design_orbit_central(design);
    chain
        .iter()
        .rev()
        .fold(initial, |c, p| p.target.body.map_or(c, central_body))
}

/// 日付つきの遷移の診断（暦を要るものと日付の読み方）。`transfer` の無いマヌーバは見ない。
pub fn check(design: &Design) -> Vec<Diagnostic> {
    let mut out = Vec::new();
    for m in design.maneuvers.values() {
        if let Some(parsed) = parse(m) {
            check_one(design, m, parsed, &mut out);
        }
    }
    out
}

/// マヌーバ 1 つ（`parsed` は [`parse`] の結果）。
fn check_one(
    design: &Design,
    m: &Maneuver,
    parsed: Result<Dates, (&'static str, String)>,
    out: &mut Vec<Diagnostic>,
) {
    let id = m.id.as_str();
    let entity = EntityKey::Maneuver(id.to_owned());
    let mut push = |code: &str, field: &str, message: String| {
        let (path, keys) = if field.is_empty() {
            ("transfer".to_owned(), vec!["transfer"])
        } else {
            (format!("transfer.{field}"), vec!["transfer", field])
        };
        out.push(
            Diagnostic::error(code, message)
                .with_target(Target::maneuver(id, Some(&path)))
                .at(design.source_map.locate(&entity, &keys)),
        );
    };
    let (departure, arrival) = match parsed {
        Ok(dates) => dates,
        // 欠けている日付は model の検査（`maneuver.transfer_dates`）が誤りにする
        Err((key, why)) if why.starts_with("Cannot") => {
            push("maneuver.transfer_epoch", key, why);
            return;
        }
        Err(_) => return,
    };
    for (key, epoch) in [("departure", departure), ("arrival", arrival)] {
        if !covers(epoch) {
            push(
                "maneuver.transfer_ephemeris",
                key,
                format!(
                    "The {key} date {} is outside the range of the ephemeris (1800 to 2050 without DE440)",
                    epoch.to_iso()
                ),
            );
        }
    }
    if arrival.seconds_since(departure) <= 0.0 {
        push(
            "maneuver.transfer_order",
            "arrival",
            "The arrival date must be after the departure date".to_owned(),
        );
    }
    if let Some(to) = m.target.body.map(central_body) {
        let from = central_before(design, m);
        if !(from != to && from.parent().is_some() && from.parent() == to.parent()) {
            push(
                "maneuver.transfer_bodies",
                "",
                format!(
                    "Dated transfers need two bodies orbiting the same parent (planet to planet); {from:?} to {to:?} is not supported"
                ),
            );
        }
    }
    check_flybys(design, m, (departure, arrival), out);
}

/// 途中のスイングバイの診断（`transfer.flybys`）: 日付が読める・暦の範囲・出発と到着の間で順に増える・
/// 通過の天体が出発・到着の天体と同じ親を回る惑星で、隣り合う天体と別。
fn check_flybys(
    design: &Design,
    m: &Maneuver,
    (departure, arrival): Dates,
    out: &mut Vec<Diagnostic>,
) {
    let flybys = parse_flybys(m);
    let flyby_count = flybys.len();
    if flyby_count == 0 {
        return;
    }
    let id = m.id.as_str();
    let entity = EntityKey::Maneuver(id.to_owned());
    let mut push = |code: &str, message: String| {
        out.push(
            Diagnostic::error(code, message)
                .with_target(Target::maneuver(id, Some("transfer.flybys")))
                .at(design.source_map.locate(&entity, &["transfer", "flybys"])),
        );
    };
    let mut nodes = vec![(central_before(design, m), departure)];
    for (i, f) in flybys.into_iter().enumerate() {
        match f {
            Ok((body, date)) if covers(date) => nodes.push((body, date)),
            Ok((_, date)) => push(
                "maneuver.flyby_ephemeris",
                format!(
                    "The date {} of flyby {} is outside the range of the ephemeris (1800 to 2050 without DE440)",
                    date.to_iso(),
                    i + 1
                ),
            ),
            Err(why) => push("maneuver.flyby_epoch", why),
        }
    }
    if let Some(to) = m.target.body.map(central_body) {
        nodes.push((to, arrival));
    }
    if nodes.len() < 2 + flyby_count {
        return; // 読めない通過があれば、順序・天体の検査はしない
    }
    for pair in nodes.windows(2) {
        for (code, message) in leg_problems(pair[0], pair[1]) {
            push(code, message);
        }
    }
}

/// 隣り合う節点（出発・通過・到着の天体と日）`(a, ta)` → `(b, tb)` の問題（日付が増えない・同じ親を回る別々の惑星でない）。
fn leg_problems(
    (a, ta): (CentralBody, Epoch),
    (b, tb): (CentralBody, Epoch),
) -> Vec<(&'static str, String)> {
    let mut out = Vec::new();
    if tb.seconds_since(ta) <= 0.0 {
        out.push((
            "maneuver.flyby_order",
            format!(
                "The flyby dates must increase strictly between the departure and arrival dates ({} is not before {})",
                ta.to_iso(),
                tb.to_iso()
            ),
        ));
    }
    if !(a != b && a.parent().is_some() && a.parent() == b.parent()) {
        out.push((
            "maneuver.flyby_bodies",
            format!(
                "Consecutive bodies of a swing-by transfer must be different planets orbiting the same parent; {a:?} followed by {b:?} is not supported"
            ),
        ));
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use fune_model::design::{
        CentralBodyKind, ManeuverCategory, ManeuverConstraints, ManeuverMethod, ManeuverTarget,
        ManeuverTransfer,
    };

    fn maneuver(
        id: &str,
        after: Option<&str>,
        body: CentralBodyKind,
        tr: (&str, &str),
    ) -> Maneuver {
        Maneuver {
            id: id.into(),
            label: None,
            category: ManeuverCategory::Insertion,
            method: ManeuverMethod::Impulsive,
            thruster: None,
            after: after.map(str::to_owned),
            via_altitude: None,
            delta_v: None,
            target: ManeuverTarget {
                body: Some(body),
                ..ManeuverTarget::default()
            },
            constraints: ManeuverConstraints::default(),
            transfer: Some(ManeuverTransfer {
                departure: Some(tr.0.to_owned()),
                arrival: Some(tr.1.to_owned()),
                departure_by: None,
                flybys: Vec::new(),
            }),
        }
    }

    fn codes(m: Maneuver) -> Vec<String> {
        let mut d = Design::default();
        d.maneuvers.insert(m.id.clone(), m);
        check(&d).into_iter().map(|x| x.code).collect()
    }

    const GOOD: (&str, &str) = ("2028-12-10T00:00:00Z", "2029-09-25T00:00:00Z");

    #[test]
    fn a_valid_earth_to_mars_transfer_passes() {
        assert!(codes(maneuver("M", None, CentralBodyKind::Mars, GOOD)).is_empty());
    }

    #[test]
    fn bad_dates_are_reported_with_their_own_codes() {
        let c = |tr| codes(maneuver("M", None, CentralBodyKind::Mars, tr));
        assert_eq!(c(("soon", GOOD.1)), ["maneuver.transfer_epoch"]);
        assert_eq!(c((GOOD.1, GOOD.0)), ["maneuver.transfer_order"]);
        assert_eq!(c((GOOD.1, GOOD.1)), ["maneuver.transfer_order"]);
        assert_eq!(
            c((GOOD.0, "2300-01-01T00:00:00Z")),
            ["maneuver.transfer_ephemeris"]
        );
    }

    #[test]
    fn only_sibling_bodies_are_supported() {
        // 地球 → 月（親子）と、同じ天体どうし
        assert_eq!(
            codes(maneuver("M", None, CentralBodyKind::Moon, GOOD)),
            ["maneuver.transfer_bodies"]
        );
        assert_eq!(
            codes(maneuver("M", None, CentralBodyKind::Earth, GOOD)),
            ["maneuver.transfer_bodies"]
        );
    }

    /// 出発の天体は、前のマヌーバが替えた中心天体（火星に着いたあとの火星 → 地球は兄弟、火星 → 火星は誤り）。
    #[test]
    fn the_departure_body_follows_the_previous_maneuvers() {
        let mut d = Design::default();
        let a = maneuver("A", None, CentralBodyKind::Mars, GOOD);
        let b = maneuver("B", Some("A"), CentralBodyKind::Earth, GOOD);
        let c = maneuver("C", Some("A"), CentralBodyKind::Mars, GOOD);
        for m in [&a, &b, &c] {
            d.maneuvers.insert(m.id.clone(), m.clone());
        }
        assert_eq!(central_before(&d, &a), CentralBody::Earth);
        assert_eq!(central_before(&d, &b), CentralBody::Mars);
        let diags: Vec<_> = check(&d).into_iter().map(|x| (x.path(), x.code)).collect();
        assert_eq!(
            diags,
            [(
                Some("maneuvers.C.transfer".to_owned()),
                "maneuver.transfer_bodies".to_owned()
            )]
        );
    }

    fn with_flybys(flybys: Vec<(CentralBodyKind, &str)>) -> Maneuver {
        let mut m = maneuver("M", None, CentralBodyKind::Mars, GOOD);
        m.transfer.as_mut().unwrap().flybys = flybys
            .into_iter()
            .map(|(body, date)| fune_model::design::ManeuverFlyby {
                body,
                date: date.to_owned(),
                min_altitude: None,
                powered: None,
            })
            .collect();
        m
    }

    /// 通過の検査: 日付の順（出発と到着の間で順に増える）・読めない日付・暦の範囲・隣り合う天体は別の同じ親の惑星。
    #[test]
    fn flybys_are_checked_for_order_dates_and_bodies() {
        use CentralBodyKind::{Earth, Moon, Venus};
        let ok = "2029-02-01T00:00:00Z";
        assert!(codes(with_flybys(vec![(Venus, ok)])).is_empty());
        // 出発より前・到着より後・逆順
        assert_eq!(
            codes(with_flybys(vec![(Venus, "2028-11-01T00:00:00Z")])),
            ["maneuver.flyby_order"]
        );
        assert_eq!(
            codes(with_flybys(vec![(Venus, "2029-10-01T00:00:00Z")])),
            ["maneuver.flyby_order"]
        );
        assert_eq!(
            codes(with_flybys(vec![
                (Venus, "2029-03-01T00:00:00Z"),
                (Earth, ok)
            ])),
            ["maneuver.flyby_order"]
        );
        assert_eq!(
            codes(with_flybys(vec![(Venus, "soon")])),
            ["maneuver.flyby_epoch"]
        );
        assert_eq!(
            codes(with_flybys(vec![(Venus, "2300-01-01T00:00:00Z")])),
            ["maneuver.flyby_ephemeris"]
        );
        // 出発（地球）と同じ天体が続く・月は別の親・到着（火星）と同じ天体が続く
        assert_eq!(
            codes(with_flybys(vec![(Earth, ok)])),
            ["maneuver.flyby_bodies"]
        );
        assert_eq!(
            codes(with_flybys(vec![(Moon, ok)])),
            ["maneuver.flyby_bodies", "maneuver.flyby_bodies"]
        );
        assert_eq!(
            codes(with_flybys(vec![(CentralBodyKind::Mars, ok)])),
            ["maneuver.flyby_bodies"]
        );
    }
}
