//! 元期と恒星時。秒の基準は J2000.0（2000-01-01T12:00:00）からの経過秒。うるう秒と UTC/TT の差は無視する（概念設計の精度）。

use crate::AstroError;
use crate::constants::{SECONDS_PER_DAY, SECONDS_PER_JULIAN_CENTURY};
use serde::{Deserialize, Serialize};

/// 元期。JSON では ISO 8601 の文字列（例: `"2027-04-01T00:00:00Z"`）。
#[derive(Debug, Clone, Copy, PartialEq, PartialOrd)]
pub struct Epoch {
    /// J2000.0 からの経過秒
    pub j2000_s: f64,
}

impl Epoch {
    /// ISO 8601（`YYYY-MM-DDThh:mm:ss[.fff]Z`）から作る。
    pub fn parse(s: &str) -> Result<Self, AstroError> {
        parse_iso(s).ok_or_else(|| AstroError::InvalidEpoch(s.to_string()))
    }
    /// 暦の日付と時刻（UTC とみなす）から作る。
    pub fn from_calendar(
        year: i64,
        month: u32,
        day: u32,
        hour: u32,
        minute: u32,
        second: f64,
    ) -> Self {
        let days = days_from_civil(year, month, day) - J2000_DAYS_FROM_UNIX;
        let s = f64::from(hour) * 3600.0 + f64::from(minute) * 60.0 + second;
        Self {
            j2000_s: days as f64 * SECONDS_PER_DAY + s - HALF_DAY_S,
        }
    }
    /// ISO 8601 の文字列にする。
    pub fn to_iso(&self) -> String {
        format_iso(self.j2000_s)
    }
    /// `dt_s` 秒後の元期。
    pub fn plus_seconds(&self, dt_s: f64) -> Self {
        Self {
            j2000_s: self.j2000_s + dt_s,
        }
    }
    /// J2000.0 からのユリウス世紀。
    pub fn julian_centuries(&self) -> f64 {
        self.j2000_s / SECONDS_PER_JULIAN_CENTURY
    }
    /// グリニッジ平均恒星時 [rad]。
    pub fn gmst_rad(&self) -> f64 {
        // IAU 1982（Vallado eq. 3-47）。T は J2000 からのユリウス世紀（UT1 ≈ UTC とみなす）。
        let t = self.julian_centuries();
        let gmst_s = GMST_C0_S + t * (GMST_C1_S + t * (GMST_C2_S + t * GMST_C3_S));
        (gmst_s * core::f64::consts::TAU / SECONDS_PER_DAY).rem_euclid(core::f64::consts::TAU)
    }
    /// UTC の年間通日（1 月 1 日 = 1）と、その日の 0 時からの秒。
    pub fn day_of_year_and_seconds(&self) -> (u16, f64) {
        let (days, seconds) = self.day_number_and_seconds();
        let (y, _, _) = civil_from_days(days + J2000_DAYS_FROM_UNIX);
        let doy = days + J2000_DAYS_FROM_UNIX - days_from_civil(y, 1, 1) + 1;
        (doy as u16, seconds)
    }
    /// UTC の日の番号（2000-01-01 = 0。日ごとの表を引く鍵）と、その日の 0 時からの秒。
    pub fn day_number_and_seconds(&self) -> (i64, f64) {
        let since_midnight_2000 = self.j2000_s + HALF_DAY_S;
        let days = (since_midnight_2000 / SECONDS_PER_DAY).floor();
        (days as i64, since_midnight_2000 - days * SECONDS_PER_DAY)
    }
    /// 暦の日付（UTC）の日の番号（2000-01-01 = 0）。
    pub fn day_number_of(year: i64, month: u32, day: u32) -> i64 {
        days_from_civil(year, month, day) - J2000_DAYS_FROM_UNIX
    }
    /// 日の番号（2000-01-01 = 0）→ (年, 月, 日)。
    pub fn calendar_of_day_number(day_number: i64) -> (i64, u32, u32) {
        civil_from_days(day_number + J2000_DAYS_FROM_UNIX)
    }
    /// 2 つの元期の差 `self − other` [s]。
    pub fn seconds_since(&self, other: Epoch) -> f64 {
        self.j2000_s - other.j2000_s
    }
}

impl Serialize for Epoch {
    fn serialize<S: serde::Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        s.serialize_str(&self.to_iso())
    }
}

impl<'de> Deserialize<'de> for Epoch {
    fn deserialize<D: serde::Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        let s = String::deserialize(d)?;
        Self::parse(&s).map_err(serde::de::Error::custom)
    }
}

#[cfg(feature = "ts")]
impl ts_rs::TS for Epoch {
    type WithoutGenerics = Self;
    type OptionInnerType = Self;
    fn name(_: &ts_rs::Config) -> String {
        String::from("string")
    }
    fn inline(cfg: &ts_rs::Config) -> String {
        <Self as ts_rs::TS>::name(cfg)
    }
}

/// IAU 1982 GMST 多項式の係数 [時間秒]（Vallado eq. 3-47）。
const GMST_C0_S: f64 = 67_310.548_41;
const GMST_C1_S: f64 = 876_600.0 * 3_600.0 + 8_640_184.812_866;
const GMST_C2_S: f64 = 0.093_104;
const GMST_C3_S: f64 = -6.2e-6;

/// 1 日 [ms]（ISO 文字列の整数演算用）。
const MS_PER_DAY: i64 = 86_400_000;

/// J2000.0 は正午なので、0 時からの半日 [s]。
const HALF_DAY_S: f64 = 0.5 * SECONDS_PER_DAY;

/// 1970-01-01 から 2000-01-01 までの日数。
const J2000_DAYS_FROM_UNIX: i64 = 10_957;

/// グレゴリオ暦の日付 → 1970-01-01 からの日数（Hinnant の days_from_civil）。
fn days_from_civil(year: i64, month: u32, day: u32) -> i64 {
    let y = if month <= 2 { year - 1 } else { year };
    let era = y.div_euclid(400);
    let yoe = y - era * 400;
    let m = i64::from(month);
    let mp = if m > 2 { m - 3 } else { m + 9 };
    let doy = (153 * mp + 2) / 5 + i64::from(day) - 1;
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    era * 146_097 + doe - 719_468
}

/// 1970-01-01 からの日数 → (年, 月, 日)（Hinnant の civil_from_days）。
fn civil_from_days(z: i64) -> (i64, u32, u32) {
    let z = z + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z - era * 146_097;
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = (doy - (153 * mp + 2) / 5 + 1) as u32;
    let m = if mp < 10 { mp + 3 } else { mp - 9 } as u32;
    let y = yoe + era * 400 + i64::from(m <= 2);
    (y, m, d)
}

fn days_in_month(year: i64, month: u32) -> u32 {
    match month {
        1 | 3 | 5 | 7 | 8 | 10 | 12 => 31,
        4 | 6 | 9 | 11 => 30,
        _ if (year % 4 == 0 && year % 100 != 0) || year % 400 == 0 => 29,
        _ => 28,
    }
}

fn digits(s: &str) -> Option<u32> {
    if s.is_empty() || !s.bytes().all(|b| b.is_ascii_digit()) {
        return None;
    }
    s.parse().ok()
}

/// `YYYY-MM-DDThh:mm:ss[.fff][Z]`（区切りは `T` か空白。日付だけでも可）を解釈する。
fn parse_iso(s: &str) -> Option<Epoch> {
    let s = s.trim();
    let s = s
        .strip_suffix('Z')
        .or_else(|| s.strip_suffix('z'))
        .or_else(|| s.strip_suffix("+00:00"))
        .unwrap_or(s);
    let (date, time) = match s.find(['T', 't', ' ']) {
        Some(i) => (&s[..i], Some(&s[i + 1..])),
        None => (s, None),
    };
    let mut dp = date.splitn(3, '-');
    let (ys, ms, ds) = (dp.next()?, dp.next()?, dp.next()?);
    if ys.len() != 4 || ms.len() != 2 || ds.len() != 2 {
        return None;
    }
    let (year, month, day) = (i64::from(digits(ys)?), digits(ms)?, digits(ds)?);
    if !(1..=12).contains(&month) || day == 0 || day > days_in_month(year, month) {
        return None;
    }
    let (hour, minute, second) = match time {
        None => (0, 0, 0.0),
        Some(t) => parse_clock(t)?,
    };
    Some(Epoch::from_calendar(year, month, day, hour, minute, second))
}

fn parse_clock(t: &str) -> Option<(u32, u32, f64)> {
    let mut tp = t.splitn(3, ':');
    let (hs, mins, ss) = (tp.next()?, tp.next()?, tp.next().unwrap_or("00"));
    if hs.len() != 2 || mins.len() != 2 {
        return None;
    }
    let (hour, minute) = (digits(hs)?, digits(mins)?);
    let (whole, frac) = match ss.split_once('.') {
        Some((w, f)) => (w, Some(f)),
        None => (ss, None),
    };
    if whole.len() != 2 {
        return None;
    }
    let mut second = f64::from(digits(whole)?);
    if let Some(f) = frac {
        digits(f)?;
        second += format!("0.{f}").parse::<f64>().ok()?;
    }
    // 60 秒（うるう秒の表記）は受けるが、時刻は 1 秒ずれたものとして扱う。
    if hour > 23 || minute > 59 || second >= 61.0 {
        return None;
    }
    Some((hour, minute, second))
}

/// J2000 秒 → `YYYY-MM-DDThh:mm:ss[.fff]Z`（ミリ秒に丸め、端数が無ければ省く）。
fn format_iso(j2000_s: f64) -> String {
    if !j2000_s.is_finite() {
        return String::from("invalid");
    }
    let total_ms =
        ((j2000_s + HALF_DAY_S) * 1000.0).round() as i64 + J2000_DAYS_FROM_UNIX * MS_PER_DAY;
    let days = total_ms.div_euclid(MS_PER_DAY);
    let ms_of_day = total_ms.rem_euclid(MS_PER_DAY);
    let (y, m, d) = civil_from_days(days);
    let (hh, rem) = (ms_of_day / 3_600_000, ms_of_day % 3_600_000);
    let (mm, rem) = (rem / 60_000, rem % 60_000);
    let (ss, ms) = (rem / 1000, rem % 1000);
    if ms == 0 {
        format!("{y:04}-{m:02}-{d:02}T{hh:02}:{mm:02}:{ss:02}Z")
    } else {
        format!("{y:04}-{m:02}-{d:02}T{hh:02}:{mm:02}:{ss:02}.{ms:03}Z")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn j2000_is_noon_on_2000_01_01() {
        let e = Epoch::parse("2000-01-01T12:00:00Z").expect("parse");
        assert_eq!(e.j2000_s, 0.0);
        assert_eq!(e.to_iso(), "2000-01-01T12:00:00Z");
    }

    #[test]
    fn round_trip_and_fraction() {
        for s in [
            "2027-04-01T00:00:00Z",
            "1999-12-31T23:59:59.250Z",
            "2024-02-29T06:30:15.001Z",
            "2100-03-01T00:00:00Z",
        ] {
            let e = Epoch::parse(s).expect(s);
            assert_eq!(e.to_iso(), s);
        }
        // 2027-04-01 00:00 は J2000（2000-01-01 12:00）から 9951.5 日。
        let e = Epoch::parse("2027-04-01T00:00:00Z").expect("parse");
        assert!((e.j2000_s - 9_951.5 * 86_400.0).abs() < 1e-6);
        assert_eq!(Epoch::parse("2027-04-01").expect("date only"), e);
    }

    #[test]
    fn day_of_year_and_seconds_of_day() {
        let e = Epoch::parse("2000-06-20T08:03:20Z").expect("epoch");
        assert_eq!(e.day_of_year_and_seconds(), (172, 29_000.0));
        let e = Epoch::parse("2027-01-01T00:00:00Z").expect("epoch");
        assert_eq!(e.day_of_year_and_seconds(), (1, 0.0));
        let e = Epoch::parse("2024-12-31T23:59:59Z").expect("epoch");
        assert_eq!(e.day_of_year_and_seconds(), (366, 86_399.0));
    }

    #[test]
    fn day_numbers_round_trip_through_the_calendar() {
        assert_eq!(Epoch::day_number_of(2000, 1, 1), 0);
        assert_eq!(Epoch::day_number_of(1999, 12, 31), -1);
        let e = Epoch::parse("2003-10-29T07:30:00Z").expect("epoch");
        let (day, sec) = e.day_number_and_seconds();
        assert_eq!(day, Epoch::day_number_of(2003, 10, 29));
        assert_eq!(sec, 27_000.0);
        assert_eq!(Epoch::calendar_of_day_number(day), (2003, 10, 29));
        assert_eq!(Epoch::calendar_of_day_number(-1), (1999, 12, 31));
    }

    #[test]
    fn rejects_bad_strings() {
        for s in [
            "",
            "2027-13-01T00:00:00Z",
            "2027-02-29T00:00:00Z",
            "2027-4-1",
            "x",
            "2027-04-01T25:00:00Z",
        ] {
            assert!(Epoch::parse(s).is_err(), "{s}");
        }
    }

    #[test]
    fn gmst_known_values() {
        // J2000.0: 280.46061837°（Vallado）
        let g0 = Epoch { j2000_s: 0.0 }.gmst_rad().to_degrees();
        assert!((g0 - 280.460_618_37).abs() < 1e-6, "{g0}");
        // Vallado 例 3-5: 1992-08-20 12:14:00 UT1 → GMST = 152.578787810°
        let e = Epoch::parse("1992-08-20T12:14:00Z").expect("parse");
        let g = e.gmst_rad().to_degrees();
        assert!((g - 152.578_787_81).abs() < 1e-4, "{g}");
    }
}
