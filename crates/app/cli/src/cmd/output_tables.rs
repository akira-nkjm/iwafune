//! `fune analyze` の出力の表（KPI の下）。並べる配列と欄と見出しだけをここに書く（表示の設定）。
//! セルの文字列は、語彙の値なら結果の `labels`、量なら結果の `dims` の次元で service の整形
//! （`fune_service::units::quantity_auto`）が作る。CLI は数を整形しない。

use crate::output::{C, Out};
use fune_service::dto::AnalysisRunResult;
use serde_json::Value;

/// 表 1 つ: 解析 ID・配列の場所（JSON Pointer。`""` は出力の最上位を 1 行にする）・（欄, 見出し）の列。
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
            ("category", "区分"),
            ("delta_v", "ΔV"),
            ("share", "割合"),
            ("maneuvers", "マヌーバ"),
        ],
    ),
    (
        "propulsion.dv_budget",
        "",
        &[
            ("total", "合計"),
            ("margin", "マージン"),
            ("total_with_margin", "マージン込み"),
        ],
    ),
    (
        "propulsion.maneuvers",
        "/maneuvers",
        &[
            ("id", "ID"),
            ("method", "方式"),
            ("delta_v", "ΔV"),
            ("burn_time", "噴射時間"),
            ("transfer_time", "遷移時間"),
            ("propellant", "推薬"),
        ],
    ),
    (
        "propulsion.propellant",
        "",
        &[
            ("dry_mass", "ドライ"),
            ("usable_propellant", "使う推薬"),
            ("residual_propellant", "残る推薬"),
            ("loaded_propellant", "積む推薬"),
            ("wet_mass", "ウェット"),
        ],
    ),
];

/// 結果の 1 つの値の文字列（語彙の表示名・整形した量・文字列・一覧）。
fn cell(r: &AnalysisRunResult, pointer: &str, value: &Value) -> C {
    if let Some(l) = r.labels.get(pointer) {
        return C::Text(l.label.clone());
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

/// 解析の出力の表を出す（表の設定が無い解析は何もしない）。
pub fn print(r: &AnalysisRunResult, out: &Out) {
    let Some(output) = &r.output else {
        return;
    };
    for (_, at, columns) in TABLES.iter().filter(|(id, _, _)| *id == r.id) {
        let rows: Vec<(String, &Value)> = match output.pointer(at) {
            Some(Value::Array(items)) => items
                .iter()
                .enumerate()
                .map(|(i, v)| (format!("{at}/{i}"), v))
                .collect(),
            Some(v) if at.is_empty() => vec![(String::new(), v)],
            _ => continue,
        };
        let headers: Vec<&str> = columns.iter().map(|(_, h)| *h).collect();
        let mut t = out.table(&headers);
        for (base, row) in rows {
            t.row(
                columns
                    .iter()
                    .map(|(key, _)| cell(r, &format!("{base}/{key}"), &row[*key]))
                    .collect(),
            );
        }
        t.print();
    }
}
