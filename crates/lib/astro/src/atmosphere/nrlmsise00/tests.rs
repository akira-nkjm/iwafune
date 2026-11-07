//! 原典 C に付属する 17 ケースの検証出力（`data/nrlmsise00_reference.txt`）との突き合わせ。

use super::*;

const REFERENCE: &str = include_str!("../../../data/nrlmsise00_reference.txt");
const RELATIVE_TOLERANCE: f64 = 1e-14;

struct Record {
    input: Nrlmsise00Input,
    expected: [f64; 11],
}

fn records() -> Vec<Record> {
    REFERENCE
        .lines()
        .map(str::trim)
        .filter(|l| !l.is_empty() && !l.starts_with('#'))
        .map(|line| {
            let f: Vec<f64> = line
                .split_whitespace()
                .map(|t| t.parse::<f64>().expect("数値"))
                .collect();
            assert_eq!(f.len(), 21, "列数: {line}");
            let geomagnetic = if f[9] == 1.0 {
                GeomagneticActivity::ThreeHourlyAp([100.0; 7])
            } else {
                GeomagneticActivity::DailyAp(f[8])
            };
            let mut expected = [0.0; 11];
            expected.copy_from_slice(&f[10..21]);
            Record {
                input: Nrlmsise00Input {
                    day_of_year: f[0] as u16,
                    seconds_of_day_ut: f[1],
                    altitude_m: f[2] * 1000.0,
                    latitude_deg: f[3],
                    longitude_deg: f[4],
                    local_solar_time_h: f[5],
                    weather: SpaceWeather {
                        f107_81day_avg: f[6],
                        f107_daily: f[7],
                        geomagnetic,
                    },
                },
                expected,
            }
        })
        .collect()
}

#[test]
fn reproduces_the_reference_output_of_the_original_implementation() {
    let records = records();
    assert_eq!(records.len(), 17);
    let mut compared = 0;
    for (case, r) in records.iter().enumerate() {
        let (d, t_inf, t_alt) = evaluate_cgs_for_tests(&r.input);
        let actual = [
            d[0], d[1], d[2], d[3], d[4], d[5], d[6], d[7], d[8], t_inf, t_alt,
        ];
        for (i, (&got, &want)) in actual.iter().zip(r.expected.iter()).enumerate() {
            assert!(got.is_finite(), "case {case} 量 {i}: {got}");
            if want == 0.0 {
                assert_eq!(got, 0.0, "case {case} 量 {i}");
                continue;
            }
            compared += 1;
            let rel = (got - want).abs() / want.abs();
            assert!(
                rel <= RELATIVE_TOLERANCE,
                "case {case} 量 {i}: {got:e} vs {want:e}（{rel:e}）"
            );
        }
    }
    assert!(compared >= 150, "{compared}");
}

#[test]
fn si_output_converts_units() {
    let r = &records()[0];
    let out = evaluate(&r.input);
    // 400 km, 原典: 4.0747e-15 g/cm³ = 4.0747e-12 kg/m³
    assert!((out.total_mass_density_kg_m3 / (r.expected[5] * 1e3) - 1.0).abs() < 1e-14);
    assert!((out.o_m3 / (r.expected[1] * 1e6) - 1.0).abs() < 1e-14);
    assert!((out.exospheric_temperature_k - r.expected[9]).abs() < 1e-9);
}

#[test]
fn below_ground_is_clamped_and_finite() {
    let input = Nrlmsise00Input::from_geodetic(
        172,
        29_000.0,
        -5_000.0,
        60.0,
        -70.0,
        SpaceWeather::nominal(),
    );
    assert_eq!(input.altitude_m, 0.0);
    assert!(evaluate(&input).total_mass_density_kg_m3.is_finite());
}
