//! `fune analyze` の出力の表（KPI の下）。並べる配列と欄と見出しだけをここに書く（表示の設定）。
//! セルの文字列は、語彙の値なら結果の `labels`、量なら結果の `dims` の次元で service の整形
//! （`fune_service::units::quantity_auto`）が作る。CLI は数を整形しない。

use crate::output::{C, Out};
use fune_service::dto::AnalysisRunResult;
use fune_service::query;
use serde_json::Value;

/// 表 1 つ: 解析 ID・配列の場所（JSON Pointer。`""` は出力の最上位を 1 行にする。`/cases/*/table` のように `*` を 1 つ含めると、
/// `*` の配列の要素ごとに行を並べて 1 つの表にし、欄の名前が `^` で始まる列は、その行を含む要素（`/cases/<i>`）の値を出す）・（欄, 見出し）の列。
type Table = (
    &'static str,
    &'static str,
    &'static [(&'static str, &'static str)],
);

const TABLES: &[Table] = &[
    (
        "propulsion.dv_budget",
        "/rows",
        &[
            ("category", "Category"),
            ("delta_v", "ΔV"),
            ("share", "Share"),
            ("maneuvers", "Maneuvers"),
        ],
    ),
    (
        "propulsion.dv_budget",
        "",
        &[
            ("total", "Total"),
            ("margin", "Margin"),
            ("total_with_margin", "With margin"),
        ],
    ),
    (
        "propulsion.maneuvers",
        "/maneuvers",
        &[
            ("id", "ID"),
            ("method", "Method"),
            ("delta_v", "ΔV"),
            ("burn_time", "Burn time"),
            ("transfer_time", "Transfer time"),
            ("propellant", "Propellant"),
        ],
    ),
    (
        "propulsion.propellant",
        "",
        &[
            ("dry_mass", "Dry"),
            ("usable_propellant", "Usable propellant"),
            ("residual_propellant", "Residual propellant"),
            ("loaded_propellant", "Loaded propellant"),
            ("wet_mass", "Wet"),
        ],
    ),
    (
        "orbit.trapped_flux",
        "/cases/*/spectrum",
        &[
            ("^model", "Map"),
            ("energy_mev", "E > [MeV]"),
            ("mean_flux", "Mean flux"),
            ("peak_flux", "Peak flux"),
            ("fluence", "Fluence"),
            ("time_fraction", "Time with flux"),
        ],
    ),
    (
        "orbit.trapped_dose",
        "/cases/*/table",
        &[
            ("^solar", "Solar activity"),
            ("shielding", "Aluminium"),
            ("protons", "Protons"),
            ("electrons", "Electrons"),
            ("bremsstrahlung", "Bremsstrahlung"),
            ("total", "Total"),
        ],
    ),
    (
        "propulsion.propellant",
        "/tanks",
        &[
            ("id", "Tank"),
            ("loaded", "Loaded"),
            ("capacity", "Capacity"),
            ("volume_margin", "Volume margin"),
        ],
    ),
];

/// 結果の 1 つの値の文字列（語彙の表示名・整形した量・文字列・一覧）。
fn cell(r: &AnalysisRunResult, pointer: &str, value: &Value) -> C {
    if let Some(l) = r.labels.get(pointer) {
        return C::Text(query::english(&l.label, Some(&l.label_en)).to_owned());
    }
    match (value, r.dims.get(pointer)) {
        (Value::Number(n), Some(dim)) => {
            C::Num(fune_service::units::quantity_auto(n.as_f64().unwrap_or(f64::NAN), dim).display)
        }
        (Value::String(s), _) => C::Text(s.clone()),
        (Value::Array(items), _) => C::Text(
            items
                .iter()
                .filter_map(Value::as_str)
                .collect::<Vec<_>>()
                .join(" → "),
        ),
        (Value::Null, _) => C::Dim("—".into()),
        (other, _) => C::Text(other.to_string()),
    }
}

/// 表の行の元: （行の値の場所, 行を含む要素の場所）。`*` を含む場所は要素ごとに展開する。
fn row_sources<'a>(output: &'a Value, at: &str) -> Vec<(String, &'a Value, String)> {
    let (before, after) = match at.split_once("/*") {
        Some((b, a)) => (b, Some(a)),
        None => (at, None),
    };
    let Some(after) = after else {
        return match output.pointer(at) {
            Some(Value::Array(items)) => items
                .iter()
                .enumerate()
                .map(|(i, v)| (format!("{at}/{i}"), v, String::new()))
                .collect(),
            Some(v) if at.is_empty() => vec![(String::new(), v, String::new())],
            _ => Vec::new(),
        };
    };
    let Some(Value::Array(parents)) = output.pointer(before) else {
        return Vec::new();
    };
    let mut rows = Vec::new();
    for (p, parent) in parents.iter().enumerate() {
        let parent_at = format!("{before}/{p}");
        if let Some(Value::Array(items)) = parent.pointer(after) {
            rows.extend(
                items
                    .iter()
                    .enumerate()
                    .map(|(i, v)| (format!("{parent_at}{after}/{i}"), v, parent_at.clone())),
            );
        }
    }
    rows
}

/// 解析の出力の表を出す（表の設定が無い解析は何もしない）。
pub fn print(r: &AnalysisRunResult, out: &Out) {
    let Some(output) = &r.output else {
        return;
    };
    for (_, at, columns) in TABLES.iter().filter(|(id, _, _)| *id == r.id) {
        let rows = row_sources(output, at);
        if rows.is_empty() {
            continue;
        }
        let headers: Vec<&str> = columns.iter().map(|(_, h)| *h).collect();
        let mut t = out.table(&headers);
        for (base, row, parent_at) in rows {
            t.row(
                columns
                    .iter()
                    .map(|(key, _)| match key.strip_prefix('^') {
                        Some(k) => {
                            let parent = output.pointer(&parent_at).unwrap_or(&Value::Null);
                            cell(r, &format!("{parent_at}/{k}"), &parent[k])
                        }
                        None => cell(r, &format!("{base}/{key}"), &row[*key]),
                    })
                    .collect(),
            );
        }
        t.print();
    }
}
