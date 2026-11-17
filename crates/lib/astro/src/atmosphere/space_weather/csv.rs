//! CelesTrak の SW-All（CSV）の読み込み。
//!
//! 1 行目が見出しで、列は見出しの名前で探す（並びが変わっても読める）。公開されている列の並びは
//! `DATE,BSRN,ND,KP1..KP8,KP_SUM,AP1..AP8,AP_AVG,CP,C9,ISN,F10.7_OBS,F10.7_ADJ,F10.7_DATA_TYPE,
//! F10.7_OBS_CENTER81,F10.7_OBS_LAST81,F10.7_ADJ_CENTER81,F10.7_ADJ_LAST81`。
//! 使う列は `DATE`（`YYYY-MM-DD`）・`AP1`〜`AP8`（3 時間の ap）・`AP_AVG`（日平均 Ap）・`F10.7_OBS`（観測された
//! F10.7 [sfu]）・`F10.7_DATA_TYPE`（`OBS`・`INT`・`PRD`・`PRM`）・`F10.7_OBS_CENTER81`・`F10.7_OBS_LAST81`。
//! 空欄は値が無いものとする（予測の行は 3 時間の値などが空欄）。負の値も無いものとする。

use super::{FluxQualifier, SpaceWeatherDay};
use crate::time::Epoch;

/// SW-All を読めなかった理由。
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum SpaceWeatherParseError {
    /// 見出しに要る列が無い（CSV でない内容を含む）。
    #[error("Space weather table header has no {0} column")]
    MissingColumn(&'static str),
    /// ある行を読めない。
    #[error("Cannot read line {line} of the space weather table: {reason}")]
    Row { line: usize, reason: String },
    /// 日付の行が 1 つも無い。
    #[error("Space weather table has no date rows")]
    Empty,
}

/// 使う列の位置。
struct Columns {
    date: usize,
    ap_3h: Option<[usize; 8]>,
    ap_daily: usize,
    f107_obs: usize,
    qualifier: usize,
    center81: usize,
    last81: Option<usize>,
}

impl Columns {
    fn from_header(header: &str) -> Result<Self, SpaceWeatherParseError> {
        let names: Vec<&str> = header.split(',').map(str::trim).collect();
        let find = |name: &str| names.iter().position(|n| *n == name);
        let need =
            |name: &'static str| find(name).ok_or(SpaceWeatherParseError::MissingColumn(name));
        const AP: [&str; 8] = ["AP1", "AP2", "AP3", "AP4", "AP5", "AP6", "AP7", "AP8"];
        let ap_3h: Option<Vec<usize>> = AP.iter().map(|n| find(n)).collect();
        Ok(Self {
            date: need("DATE")?,
            ap_3h: ap_3h.and_then(|v| v.try_into().ok()),
            ap_daily: need("AP_AVG")?,
            f107_obs: need("F10.7_OBS")?,
            qualifier: need("F10.7_DATA_TYPE")?,
            center81: need("F10.7_OBS_CENTER81")?,
            last81: find("F10.7_OBS_LAST81"),
        })
    }
}

/// CSV のバイト列 → 日付の順の行。日付が増えていなければ誤り。
pub(super) fn parse(bytes: &[u8]) -> Result<Vec<SpaceWeatherDay>, SpaceWeatherParseError> {
    let text = String::from_utf8_lossy(bytes);
    let text = text.trim_prefix('\u{feff}');
    let mut lines = text
        .lines()
        .enumerate()
        .filter(|(_, l)| !l.trim().is_empty());
    let (_, header) = lines.next().ok_or(SpaceWeatherParseError::Empty)?;
    let columns = Columns::from_header(header)?;
    let mut days: Vec<SpaceWeatherDay> = Vec::new();
    for (i, line) in lines {
        let row_error = |reason: String| SpaceWeatherParseError::Row {
            line: i + 1,
            reason,
        };
        let day = parse_row(&columns, line).map_err(row_error)?;
        if let Some(prev) = days.last()
            && prev.day_number >= day.day_number
        {
            return Err(row_error("Date is not after the previous row".into()));
        }
        days.push(day);
    }
    if days.is_empty() {
        return Err(SpaceWeatherParseError::Empty);
    }
    Ok(days)
}

fn parse_row(c: &Columns, line: &str) -> Result<SpaceWeatherDay, String> {
    let fields: Vec<&str> = line.split(',').map(str::trim).collect();
    let field = |i: usize| fields.get(i).copied().unwrap_or("");
    let number = |i: usize| -> Result<Option<f64>, String> {
        let s = field(i);
        if s.is_empty() {
            return Ok(None);
        }
        let v: f64 = s.parse().map_err(|_| format!("not a number: {s:?}"))?;
        Ok((v.is_finite() && v >= 0.0).then_some(v))
    };
    let ap_3h = match c.ap_3h {
        Some(cols) => {
            let values: Result<Vec<Option<f64>>, String> =
                cols.iter().map(|&i| number(i)).collect();
            let values: Option<Vec<f64>> = values?.into_iter().collect();
            values.and_then(|v| v.try_into().ok())
        }
        None => None,
    };
    Ok(SpaceWeatherDay {
        day_number: date(field(c.date))?,
        qualifier: qualifier(field(c.qualifier))?,
        ap_3h,
        ap_daily: number(c.ap_daily)?,
        f107_obs: number(c.f107_obs)?,
        f107_obs_center81: number(c.center81)?,
        f107_obs_last81: c.last81.map(number).transpose()?.flatten(),
    })
}

/// `YYYY-MM-DD` → 日の番号。
fn date(s: &str) -> Result<i64, String> {
    if s.len() != 10 {
        return Err(format!("not a date: {s:?}"));
    }
    let epoch = Epoch::parse(s).map_err(|_| format!("not a date: {s:?}"))?;
    Ok(epoch.day_number_and_seconds().0)
}

fn qualifier(s: &str) -> Result<FluxQualifier, String> {
    match s {
        "OBS" => Ok(FluxQualifier::Observed),
        "INT" => Ok(FluxQualifier::Interpolated),
        "PRD" => Ok(FluxQualifier::DailyPredicted),
        "PRM" => Ok(FluxQualifier::MonthlyPredicted),
        _ => Err(format!("unknown F10.7_DATA_TYPE: {s:?}")),
    }
}
