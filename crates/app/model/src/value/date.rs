//! 暦の日付の形（`2026-10-15`）。出典の `date`・`retrieved` の検査が使う。
//!
//! 暦の計算の正本は `sat_astro::time`（fune-model は sat-astro に依存しないので、ここは形の判定だけを持つ。
//! 日付だけの文字列で `sat_astro::time::Epoch::parse` と判定が一致することを fune-projection のテストが確かめる）。

/// `YYYY-MM-DD`（4 桁の年・2 桁の月・2 桁の日）で、グレゴリオ暦にある日か。
pub fn is_calendar_date(text: &str) -> bool {
    let b = text.as_bytes();
    if b.len() != 10 || b[4] != b'-' || b[7] != b'-' {
        return false;
    }
    let num = |r: std::ops::Range<usize>| -> Option<u32> {
        let s = text.get(r)?;
        s.bytes()
            .all(|c| c.is_ascii_digit())
            .then(|| s.parse().ok())?
    };
    let (Some(year), Some(month), Some(day)) = (num(0..4), num(5..7), num(8..10)) else {
        return false;
    };
    (1..=12).contains(&month) && day >= 1 && day <= days_in_month(year, month)
}

fn days_in_month(year: u32, month: u32) -> u32 {
    match month {
        1 | 3 | 5 | 7 | 8 | 10 | 12 => 31,
        4 | 6 | 9 | 11 => 30,
        _ if (year.is_multiple_of(4) && !year.is_multiple_of(100)) || year.is_multiple_of(400) => {
            29
        }
        _ => 28,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn accepts_real_days_only() {
        assert!(is_calendar_date("2026-10-15"));
        assert!(is_calendar_date("2024-02-29"));
        assert!(!is_calendar_date("2026-02-29"));
        assert!(!is_calendar_date("1900-02-29"));
        assert!(is_calendar_date("2000-02-29"));
        assert!(!is_calendar_date("2026-13-01"));
        assert!(!is_calendar_date("2026-1-5"));
        assert!(!is_calendar_date("2026/10/15"));
        assert!(!is_calendar_date("2026-10-15T00:00:00Z"));
        assert!(!is_calendar_date("Oct 2026"));
    }
}
