//! 惑星の位置（JPL/Standish の近似 Kepler 要素、表 1: 1800〜2050 年）。
//! 係数は `data/planets_standish_t1.txt` を埋め込み、最初に使うときに 1 度だけ解釈する。

use super::{Body, obliquity_j2000};
use crate::constants::AU;
use crate::earth::ecliptic_to_equatorial;
use crate::kepler::{perifocal_to_eci, solve_kepler};
use crate::math::Vec3;
use crate::time::Epoch;
use std::sync::OnceLock;

const TABLE_TEXT: &str = include_str!("../../data/planets_standish_t1.txt");

/// J2000 の要素と、ユリウス世紀あたりの変化率。
#[derive(Debug, Clone, Copy)]
struct Elements {
    /// a [AU], e, I, L, ϖ, Ω [rad]
    at_j2000: [f64; 6],
    /// 世紀あたりの変化率
    rate: [f64; 6],
}

fn parse_line(line: &str) -> Option<(&str, Elements)> {
    let mut tok = line.split_whitespace();
    let name = tok.next()?;
    let mut v = [0.0; 12];
    for slot in &mut v {
        *slot = tok.next()?.parse().ok()?;
    }
    let mut at_j2000 = [0.0; 6];
    let mut rate = [0.0; 6];
    at_j2000.copy_from_slice(&v[..6]);
    rate.copy_from_slice(&v[6..]);
    Some((name, Elements { at_j2000, rate }))
}

fn table() -> &'static [(&'static str, Elements)] {
    static TABLE: OnceLock<Vec<(&'static str, Elements)>> = OnceLock::new();
    TABLE.get_or_init(|| {
        TABLE_TEXT
            .lines()
            .map(str::trim)
            .filter(|l| !l.is_empty() && !l.starts_with('#'))
            .filter_map(parse_line)
            .collect()
    })
}

fn elements(name: &str) -> Option<Elements> {
    table().iter().find(|(n, _)| *n == name).map(|(_, e)| *e)
}

/// 日心黄道直交座標 [AU]（J2000 平均黄道・分点）。
fn heliocentric_ecliptic_au(el: &Elements, t: f64) -> Vec3 {
    let p: [f64; 6] = core::array::from_fn(|i| el.at_j2000[i] + el.rate[i] * t);
    let [a, e, inc, mean_long, peri_long, node] = p;
    let m = (mean_long - peri_long).rem_euclid(core::f64::consts::TAU);
    let ea = solve_kepler(m, e);
    let x_orb = a * (ea.cos() - e);
    let y_orb = a * (1.0 - e * e).sqrt() * ea.sin();
    perifocal_to_eci(node, inc, peri_long - node).mul_vec(Vec3::new(x_orb, y_orb, 0.0))
}

fn table_name(body: Body) -> &'static str {
    match body {
        Body::Mercury => "Mercury",
        Body::Venus => "Venus",
        Body::Mars => "Mars",
        Body::Jupiter => "Jupiter",
        Body::Saturn => "Saturn",
        Body::Uranus => "Uranus",
        Body::Neptune => "Neptune",
        // 太陽・月はここに来ない（呼び出し側が振り分ける）。
        Body::Sun | Body::Moon => "",
    }
}

/// 惑星の地心位置（ECI = J2000 平均赤道）[m]。表に無いとき（埋め込みデータの破損）は NaN。
pub(super) fn position_eci_m(body: Body, epoch: Epoch) -> Vec3 {
    let t = epoch.julian_centuries();
    let (Some(planet), Some(earth)) = (elements(table_name(body)), elements("Earth")) else {
        return Vec3::new(f64::NAN, f64::NAN, f64::NAN);
    };
    let geo = heliocentric_ecliptic_au(&planet, t) - heliocentric_ecliptic_au(&earth, t);
    ecliptic_to_equatorial(geo * AU, obliquity_j2000())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn embedded_table_has_all_rows() {
        for name in [
            "Mercury", "Venus", "Earth", "Mars", "Jupiter", "Saturn", "Uranus", "Neptune",
        ] {
            assert!(elements(name).is_some(), "{name}");
        }
    }

    #[test]
    fn earth_heliocentric_distance_is_about_one_au() {
        let earth = elements("Earth").expect("Earth");
        for t in [-0.5, 0.0, 0.26, 0.5] {
            let r = heliocentric_ecliptic_au(&earth, t).norm();
            assert!((r - 1.0).abs() < 0.02, "{r}");
        }
    }
}
