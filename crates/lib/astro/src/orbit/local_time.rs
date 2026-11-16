//! 交点の地方時（LTAN・LTDN）と RAAN の換算。
//!
//! 地方時は**真太陽時**: 太陽の赤経には天体の位置のモデル（[`crate::ephem::sun_direction_eci`]。
//! DE440 を登録していればそれ）をそのまま使う。平均太陽時との差は均時差（最大 ±16 分）。
//! 交点の地方時 = 12 h + (Ω − α☉)·12/π。降交点は昇交点の反対側なので LTDN = LTAN ± 12 h。

use crate::AstroError;
use crate::ephem::{CentralBody, sun_direction_eci};
use crate::time::Epoch;
use core::f64::consts::{PI, TAU};

fn check_hours(name: &str, hours: f64) -> Result<(), AstroError> {
    if hours.is_finite() && (0.0..24.0).contains(&hours) {
        Ok(())
    } else {
        Err(AstroError::InvalidInput(format!(
            "{name} must be a time in hours from 0 up to (not including) 24 ({hours})"
        )))
    }
}

/// 時刻 `epoch` の太陽の赤経 [rad]。
fn sun_right_ascension(epoch: Epoch) -> f64 {
    let s = sun_direction_eci(epoch);
    s.y.atan2(s.x)
}

/// 昇交点の地方時 `ltan_hours`（0〜24 h 未満）になる RAAN [rad]（0〜2π）。地球を回る軌道。
pub fn raan_from_ltan(epoch: Epoch, ltan_hours: f64) -> Result<f64, AstroError> {
    raan_from_ltan_about(CentralBody::Earth, epoch, ltan_hours)
}

/// 中心天体 `central` を回る軌道で、昇交点の地方時が `ltan_hours` になる RAAN [rad]（0〜2π）。
///
/// 対応するのは地球だけ。地球以外では、軌道の RAAN が ECI（J2000 平均赤道）基準なのに対して、
/// その天体の地方時は天体自身の赤道で測るので、太陽の向きを ECI のまま使うと誤った値になる。
/// そのため地球以外は `InvalidInput`（天体の赤道基準の RAAN を扱えるようにするまで）。
pub fn raan_from_ltan_about(
    central: CentralBody,
    epoch: Epoch,
    ltan_hours: f64,
) -> Result<f64, AstroError> {
    check_hours("LTAN", ltan_hours)?;
    require_earth(central)?;
    Ok((sun_right_ascension(epoch) + (ltan_hours - 12.0) * PI / 12.0).rem_euclid(TAU))
}

/// 降交点の地方時 `ltdn_hours`（0〜24 h 未満）になる RAAN [rad]（0〜2π）。LTAN = LTDN ± 12 h。
pub fn raan_from_ltdn(epoch: Epoch, ltdn_hours: f64) -> Result<f64, AstroError> {
    raan_from_ltdn_about(CentralBody::Earth, epoch, ltdn_hours)
}

/// 中心天体 `central` を回る軌道の、降交点の地方時 `ltdn_hours` に対する RAAN [rad]（対応天体は
/// [`raan_from_ltan_about`] と同じ）。
pub fn raan_from_ltdn_about(
    central: CentralBody,
    epoch: Epoch,
    ltdn_hours: f64,
) -> Result<f64, AstroError> {
    check_hours("LTDN", ltdn_hours)?;
    raan_from_ltan_about(central, epoch, (ltdn_hours + 12.0) % 24.0)
}

fn require_earth(central: CentralBody) -> Result<(), AstroError> {
    if central == CentralBody::Earth {
        Ok(())
    } else {
        Err(AstroError::InvalidInput(
            "RAAN conversion from local time only supports orbits around Earth (RAAN for other bodies is relative to ECI and does not match local time on the body's equator)".into(),
        ))
    }
}

/// RAAN `raan_rad` の昇交点の地方時 [h]（0〜24 h 未満）。
pub fn ltan_from_raan(epoch: Epoch, raan_rad: f64) -> f64 {
    (12.0 + (raan_rad - sun_right_ascension(epoch)) * 12.0 / PI).rem_euclid(24.0)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::constants::R_EARTH;
    use crate::orbit::{OrbitSpec, sso_inclination_rad};
    use sat_units::typed::Qty;

    fn epoch() -> Epoch {
        Epoch::parse("2027-04-01T00:00:00Z").expect("epoch")
    }

    #[test]
    fn noon_ltan_is_the_sun_right_ascension() {
        let raan = raan_from_ltan(epoch(), 12.0).expect("12 h");
        assert!((raan - sun_right_ascension(epoch()).rem_euclid(TAU)).abs() < 1e-12);
        // 6 時間ずれると 90°
        let r18 = raan_from_ltan(epoch(), 18.0).expect("18 h");
        assert!(((r18 - raan).rem_euclid(TAU) - PI / 2.0).abs() < 1e-12);
    }

    #[test]
    fn ltdn_and_ltan_are_twelve_hours_apart() {
        let a = raan_from_ltdn(epoch(), 10.5).expect("LTDN 10:30");
        let b = raan_from_ltan(epoch(), 22.5).expect("LTAN 22:30");
        assert!((a - b).abs() < 1e-12);
        let c = raan_from_ltdn(epoch(), 22.5).expect("LTDN 22:30");
        let d = raan_from_ltan(epoch(), 10.5).expect("LTAN 10:30");
        assert!((c - d).abs() < 1e-12);
    }

    #[test]
    fn rejects_out_of_range_hours() {
        for h in [-0.1, 24.0, f64::NAN, f64::INFINITY] {
            assert!(raan_from_ltan(epoch(), h).is_err());
            assert!(raan_from_ltdn(epoch(), h).is_err());
        }
        assert!(raan_from_ltan(epoch(), 0.0).is_ok());
    }

    /// 伝搬した状態の昇交点の位置から求めた地方時が、指定した LTAN に戻る。
    /// SSO は平均太陽に追従するので、30 日後でも真太陽時の LTAN は均時差の変化（数分）しか動かない。
    #[test]
    fn round_trip_with_propagated_node() {
        let e = epoch();
        let i = sso_inclination_rad(R_EARTH + 550e3, 0.0);
        let raan = raan_from_ltan(e, 10.5).expect("LTAN 10:30");
        let orbit = OrbitSpec::circular(
            e,
            Qty::from_raw(550e3),
            Qty::from_raw(i),
            Qty::from_raw(raan),
        );
        // 円軌道で近地点引数・平均近点角が 0 なので、元期の位置が昇交点
        let r = orbit.state_at(0.0).expect("state").r_eci;
        let node_ra = r.y.atan2(r.x);
        assert!((ltan_from_raan(e, node_ra) - 10.5).abs() < 1e-9);
        let t = 30.0 * crate::constants::SECONDS_PER_DAY;
        let later = orbit.mean_elements_at(t).expect("elements").raan_rad;
        let ltan = ltan_from_raan(e.plus_seconds(t), later);
        assert!((ltan - 10.5).abs() < 0.25, "{ltan}");
    }
}
