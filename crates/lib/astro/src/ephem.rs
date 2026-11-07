//! 天体の位置（太陽・月・惑星。解析式: 太陽は VSOP87A の切り詰め、月は Meeus Ch.47、惑星は Standish の表）。
//! **天体の位置はここだけ。** DE440 を登録していれば（[`install_de440`]）期間内はそちらを使う。
//!
//! 解析式の精度（DE440 との比較、J2000 ±50 年）: 太陽の方向 最悪 0.0006°、月の方向 最悪 0.005°。
//! 惑星（Standish 表 1、1800〜2050 年）は火星で 10⁵ km 級。第三体摂動の用途には十分。
//! 返すのは幾何位置（光行差・光行時間は掛けない。力と食の判定に要るのは幾何位置）。
//! 時刻は UTC と TT を区別しない（差 ≈ 69 s は太陽の方向で 0.0008°）。

mod central;
mod de440;
mod meeus_moon;
pub mod orientation;
mod planets;
mod vsop87a_earth;

pub use central::CentralBody;
pub use de440::{De440, body_id as de440_body_id};

use crate::AstroError;
use std::sync::OnceLock;

/// 登録した DE440（[`install_de440`]）。**sat-astro で唯一のプロセス全体の状態。**
/// 天体の位置は全解析が使う純関数で、表（20.7 MB）を引数で全経路に通すと API が崩れるため、
/// 一度だけ設定できる `OnceLock` に置く。登録後に中身は変わらないので、結果は呼び出し順に依らない。
static DE440: OnceLock<De440> = OnceLock::new();

/// DE440 を登録する（プロセスで 1 回だけ。2 回目は `InvalidInput`）。以後 1849〜2150 年は DE440 を使う。
pub fn install_de440(table: De440) -> Result<(), AstroError> {
    DE440
        .set(table)
        .map_err(|_| AstroError::InvalidInput("DE440 はすでに登録されています".into()))
}

/// 天体の位置の出所。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(rename_all = "snake_case")]
pub enum EphemerisSource {
    /// JPL DE440（登録済みで、期間内）
    De440,
    /// 解析式（VSOP87A・Meeus・Standish）
    Analytic,
}

/// `epoch` で [`position_eci_m`] が使う暦。
pub fn source_at(epoch: Epoch) -> EphemerisSource {
    match DE440.get() {
        Some(t) if t.covers(epoch) => EphemerisSource::De440,
        _ => EphemerisSource::Analytic,
    }
}

fn de440_id(body: Body) -> i32 {
    use de440::body_id as id;
    match body {
        Body::Sun => id::SUN,
        Body::Moon => id::MOON,
        Body::Mercury => id::MERCURY_BARYCENTER,
        Body::Venus => id::VENUS_BARYCENTER,
        Body::Mars => id::MARS_BARYCENTER,
        Body::Jupiter => id::JUPITER_BARYCENTER,
        Body::Saturn => id::SATURN_BARYCENTER,
        Body::Uranus => id::URANUS_BARYCENTER,
        Body::Neptune => id::NEPTUNE_BARYCENTER,
    }
}

pub use crate::body::Body;
use crate::constants::AU;
use crate::earth::{ecliptic_to_equatorial, mean_obliquity_rad, of_date_to_j2000};
use crate::math::Vec3;
use crate::time::Epoch;
use serde::{Deserialize, Serialize};

/// 地心から天体への位置ベクトル（ECI = J2000 平均赤道）[m]。
///
/// DE440 を登録していて（[`install_de440`]）期間内なら DE440、それ以外は解析式（[`analytic_position_eci_m`]）。
pub fn position_eci_m(body: Body, epoch: Epoch) -> Vec3 {
    if let Some(p) = DE440
        .get()
        .and_then(|t| t.geocentric_position_m(de440_id(body), epoch))
    {
        return p;
    }
    analytic_position_eci_m(body, epoch)
}

/// 解析式だけによる地心の位置 [m]（DE440 を登録していても使わない。精度の比較用）。
pub fn analytic_position_eci_m(body: Body, epoch: Epoch) -> Vec3 {
    match body {
        Body::Sun => sun_position_eci_m(epoch),
        Body::Moon => moon_position_eci_m(epoch),
        planet => planets::position_eci_m(planet, epoch),
    }
}

/// 地心から太陽への単位ベクトル（ECI）。
pub fn sun_direction_eci(epoch: Epoch) -> Vec3 {
    position_eci_m(Body::Sun, epoch)
        .normalized()
        .unwrap_or(Vec3::X)
}

/// 中心天体 `central` から太陽への単位ベクトル（ECI）。地球なら [`sun_direction_eci`] と同じ。
pub fn sun_direction_from(central: CentralBody, epoch: Epoch) -> Vec3 {
    central
        .relative_position_m(Body::Sun, epoch)
        .normalized()
        .unwrap_or(Vec3::X)
}

/// J2000 の黄道傾斜角 ε₀ [rad]。
fn obliquity_j2000() -> f64 {
    mean_obliquity_rad(Epoch { j2000_s: 0.0 })
}

/// `Σ_it T^it · Σ_n A·cos(B + C·T)` を Horner 法で畳む（VSOP87）。
fn vsop87_series(series: &[&[vsop87a_earth::Term]; 6], t_millennia: f64) -> f64 {
    series.iter().rev().fold(0.0, |acc, terms| {
        let sum: f64 = terms
            .iter()
            .map(|&(a, b, c)| a * (b + c * t_millennia).cos())
            .sum();
        acc * t_millennia + sum
    })
}

/// 太陽の地心位置 [m]（VSOP87A の地球日心位置の符号反転。J2000 黄道 → 赤道）。
fn sun_position_eci_m(epoch: Epoch) -> Vec3 {
    let t = epoch.julian_centuries() / 10.0; // VSOP87 はユリウス千年
    let earth = Vec3::new(
        vsop87_series(&vsop87a_earth::EARTH_X, t),
        vsop87_series(&vsop87a_earth::EARTH_Y, t),
        vsop87_series(&vsop87a_earth::EARTH_Z, t),
    );
    ecliptic_to_equatorial(-earth * AU, obliquity_j2000())
}

/// 度の T 多項式を Horner 法で畳み、ラジアンにする。
fn polynomial_angle_rad(coefficients: &[f64], t: f64) -> f64 {
    let degrees = coefficients.iter().rev().fold(0.0, |acc, &c| acc * t + c);
    (degrees % 360.0).to_radians()
}

/// 月の地心位置 [m]（Meeus Ch.47。黄経・黄緯・距離は of date → 歳差で J2000 へ回す）。
fn moon_position_eci_m(epoch: Epoch) -> Vec3 {
    use meeus_moon::*;
    let t = epoch.julian_centuries();
    let linear = |c: [f64; 2]| (c[0] + c[1] * t).to_radians();
    let lp = polynomial_angle_rad(&MEAN_LONGITUDE_DEG, t);
    let d = polynomial_angle_rad(&MEAN_ELONGATION_DEG, t);
    let m = polynomial_angle_rad(&SUN_MEAN_ANOMALY_DEG, t);
    let mp = polynomial_angle_rad(&MOON_MEAN_ANOMALY_DEG, t);
    let f = polynomial_angle_rad(&ARGUMENT_OF_LATITUDE_DEG, t);
    let (a1, a2, a3) = (
        linear(VENUS_ARGUMENT_DEG),
        linear(JUPITER_ARGUMENT_DEG),
        linear(SIDEREAL_ARGUMENT_DEG),
    );
    let ecc = 1.0 + (ECCENTRICITY_FACTOR[0] + ECCENTRICITY_FACTOR[1] * t) * t;
    let e_pow = |k: i32| match k.abs() {
        1 => ecc,
        2 => ecc * ecc,
        _ => 1.0,
    };
    let arg = |(cd, cm, cmp, cf): (i32, i32, i32, i32)| {
        f64::from(cd) * d + f64::from(cm) * m + f64::from(cmp) * mp + f64::from(cf) * f
    };
    let add_lon = ADDITIVE_LONGITUDE_DEG[0] * a1.sin()
        + ADDITIVE_LONGITUDE_DEG[1] * (lp - f).sin()
        + ADDITIVE_LONGITUDE_DEG[2] * a2.sin();
    let add_lat = ADDITIVE_LATITUDE_DEG[0] * lp.sin()
        + ADDITIVE_LATITUDE_DEG[1] * a3.sin()
        + ADDITIVE_LATITUDE_DEG[2] * (a1 - f).sin()
        + ADDITIVE_LATITUDE_DEG[3] * (a1 + f).sin()
        + ADDITIVE_LATITUDE_DEG[4] * (lp - mp).sin()
        + ADDITIVE_LATITUDE_DEG[5] * (lp + mp).sin();
    // 小さい項から足す（丸め誤差の積み上がりを抑える）。
    let (lon_deg, radius_m) = TERMS_LONGITUDE_RADIUS.iter().rev().fold(
        (add_lon, MEAN_DISTANCE_M),
        |(lon, r), &(cd, cm, cmp, cf, cl, cr)| {
            let (s, c) = arg((cd, cm, cmp, cf)).sin_cos();
            let k = e_pow(cm);
            (lon + cl * k * s, r + cr * k * c)
        },
    );
    let lat_deg = TERMS_LATITUDE
        .iter()
        .rev()
        .fold(add_lat, |lat, &(cd, cm, cmp, cf, cb)| {
            lat + cb * e_pow(cm) * arg((cd, cm, cmp, cf)).sin()
        });
    let lambda = lp + lon_deg.to_radians();
    let beta = lat_deg.to_radians();
    let ecl = Vec3::from_spherical(lambda, beta) * radius_m;
    of_date_to_j2000(
        ecliptic_to_equatorial(ecl, mean_obliquity_rad(epoch)),
        epoch,
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    fn deg_between(a: Vec3, b: Vec3) -> f64 {
        a.angle_to(b).to_degrees()
    }

    /// Meeus 例 25.b（VSOP87 全項）: 1992-10-13 0h TD の太陽の幾何黄経（of date, FK5）199.907347°、
    /// 黄緯 ≈ 0、距離 0.99760775 AU。of date → J2000 に回して方向を比べる。
    #[test]
    fn sun_matches_meeus_example_25b() {
        let e = Epoch::parse("1992-10-13T00:00:00Z").expect("元期");
        let r = sun_position_eci_m(e);
        let dist_au = r.norm() / AU;
        assert!((dist_au - 0.997_607_75).abs() < 1e-5, "{dist_au}");
        let l = 199.907_347_f64.to_radians();
        let ecl = Vec3::new(l.cos(), l.sin(), 0.0);
        let expected = of_date_to_j2000(ecliptic_to_equatorial(ecl, mean_obliquity_rad(e)), e);
        let err = deg_between(r, expected);
        assert!(err < 0.002, "太陽の方向の誤差 {err}°");
    }

    /// Meeus 例 47.a: 1992-04-12 0h TD の月 λ = 133.162655°, β = −3.229126°, Δ = 368409.7 km（of date、章動なし）。
    #[test]
    fn moon_matches_meeus_example_47a() {
        let e = Epoch::parse("1992-04-12T00:00:00Z").expect("元期");
        let r = moon_position_eci_m(e);
        assert!((r.norm() / 1e3 - 368_409.7).abs() < 1.0, "{}", r.norm());
        let (l, b) = (133.162_655_f64.to_radians(), (-3.229_126_f64).to_radians());
        let ecl = Vec3::new(b.cos() * l.cos(), b.cos() * l.sin(), b.sin());
        let expected = of_date_to_j2000(ecliptic_to_equatorial(ecl, mean_obliquity_rad(e)), e);
        let err = deg_between(r, expected);
        assert!(err < 1e-3, "月の方向の誤差 {err}°");
    }

    /// 春分（2027-03-20 20:25 UTC）の太陽は of date の黄経 0。J2000 系では歳差（27.2 年 × 50.29″ ≈ 0.380°）の分だけ
    /// 黄経が負にずれ、赤緯 ≈ asin(sin ε · sin(−0.380°)) ≈ −0.00264 rad になる。
    #[test]
    fn sun_near_equinox_is_on_x_axis() {
        let e = Epoch::parse("2027-03-20T20:25:00Z").expect("元期");
        let s = sun_direction_eci(e);
        assert!((s.z + 0.002_64).abs() < 5e-4, "{s:?}");
        assert!(deg_between(s, Vec3::X) < 0.5, "{s:?}");
    }

    /// テストでは DE440 を登録しない（プロセス全体の状態なので、並列のほかのテストに効いてしまう）。
    #[test]
    fn analytic_is_used_without_de440() {
        let e = Epoch::parse("2027-04-01T00:00:00Z").expect("元期");
        assert_eq!(source_at(e), EphemerisSource::Analytic);
        assert_eq!(
            position_eci_m(Body::Moon, e),
            analytic_position_eci_m(Body::Moon, e)
        );
    }

    #[test]
    fn gm_values_are_ordered() {
        assert!(Body::Sun.gm() > Body::Jupiter.gm());
        assert!((Body::Moon.gm() / crate::constants::MU_EARTH - 1.0 / 81.3).abs() < 1e-4);
    }

    /// 惑星: 2000-01-01 の地心距離が既知値（JPL Horizons）に近い。
    #[test]
    fn planets_distance_sanity() {
        let e = Epoch { j2000_s: 0.0 };
        // 木星 ≈ 4.62 AU、火星 ≈ 1.85 AU、金星 ≈ 1.14 AU（2000-01-01 12:00 TT）
        for (body, expected_au, tol) in [
            (Body::Jupiter, 4.62, 0.03),
            (Body::Mars, 1.85, 0.03),
            (Body::Venus, 1.14, 0.03),
        ] {
            let d = position_eci_m(body, e).norm() / AU;
            assert!((d - expected_au).abs() < tol, "{body:?}: {d} AU");
        }
    }
}
