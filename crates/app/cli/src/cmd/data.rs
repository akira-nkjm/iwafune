//! `fune data status|fetch|space-weather`

use crate::args::DataCmd;
use crate::output::{C, MARK_OK, MARK_VIOLATION, Out};
use fune_service::data;
use fune_service::dto::{DataStatus, SpaceWeatherLookup};

/// 外部データのコマンド（プロジェクトが無くても使える）。
pub fn run(cmd: &DataCmd, out: &Out) -> anyhow::Result<u8> {
    let status = match cmd {
        DataCmd::Status => data::data_status()?,
        DataCmd::Fetch { id, from } => {
            let s = data::fetch(id, from.as_deref())?;
            out.ok(format!("Fetched {id} and verified its SHA-256"));
            s
        }
        DataCmd::SpaceWeather { date, activity } => {
            let found = data::space_weather::lookup(date, activity.as_deref())?;
            show_weather(&found, out)?;
            return Ok(0);
        }
    };
    show(&status, out)?;
    Ok(0)
}

fn show(s: &DataStatus, out: &Out) -> anyhow::Result<()> {
    if out.json {
        return out.json(s);
    }
    let mut t = out
        .table(&["ID", "State", "Size", "Loaded", "Location"])
        .numeric(&[2]);
    for item in &s.items {
        let state = match (item.present, item.verified) {
            (false, _) => C::Dim("not fetched".into()),
            (true, Some(true)) => C::Verdict(format!("{MARK_OK} SHA-256 matches"), true),
            (true, _) => C::Verdict(format!("{MARK_VIOLATION} SHA-256 mismatch"), false),
        };
        let size = item
            .size
            .as_ref()
            .map_or(C::Dim("—".into()), |q| C::Num(q.display.clone()));
        let installed = if item.installed {
            C::Verdict("yes".into(), true)
        } else {
            C::Dim(item.install_error.clone().unwrap_or_else(|| "—".into()))
        };
        t.row(vec![
            C::Strong(item.id.clone()),
            state,
            size,
            installed,
            C::Dim(item.path.clone()),
        ]);
    }
    t.print();
    for item in s.items.iter().filter(|i| !i.present) {
        println!("{}", out.dim(format!("Fetch: fune data fetch {}", item.id)));
    }
    Ok(())
}

/// 日付で引いた宇宙天気。出所は API と同じ綴り（`observed`・`preset` など）で出す。
fn show_weather(found: &SpaceWeatherLookup, out: &Out) -> anyhow::Result<()> {
    if out.json {
        return out.json(found);
    }
    let w = &found.weather;
    let fallback = spelling(&found.fallback);
    let rows = [
        ("F10.7 (previous day)", w.f107, spelling(&w.f107_source)),
        ("F10.7 (81-day mean)", w.f107a, spelling(&w.f107_source)),
        ("Ap (daily mean)", w.ap, spelling(&w.ap_source)),
    ];
    let mut t = out.table(&["Value", "", "Source"]).numeric(&[1]);
    for (label, value, from) in rows {
        let from = if from == "preset" {
            format!("preset ({fallback})")
        } else {
            from
        };
        t.row(vec![
            C::Text(label.into()),
            C::Num(value.to_string()),
            C::Dim(from),
        ]);
    }
    t.print();
    match &found.version {
        Some(v) => println!(
            "{}",
            out.dim(format!(
                "Table: {} (fetched {}, SHA-256 {}, observed to {}, predicted to {})",
                v.source_url,
                v.fetched_at.as_deref().unwrap_or("not recorded"),
                v.sha256,
                v.last_observed_date.as_deref().unwrap_or("—"),
                v.last_date
            ))
        ),
        None => println!(
            "{}",
            out.dim("No table (fetch: fune data fetch space_weather)")
        ),
    }
    if let Some(e) = &found.load_error {
        out.warn(e);
    }
    Ok(())
}

/// 列挙の値の API での綴り（`"observed"` など）。
fn spelling<T: serde::Serialize>(value: &T) -> String {
    serde_json::to_value(value)
        .ok()
        .and_then(|v| v.as_str().map(str::to_owned))
        .unwrap_or_default()
}
