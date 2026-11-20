//! Typst のソース。モジュール（`iwafune.typ`）は固定のテキスト（`template/iwafune.typ`）で、言語に依存する文言を持たない
//! （文言は `data.json` が持つ）。例の文書は書き出した項目とケースから作る。

use crate::model::{Item, Lang, ReportData};
use std::fmt::Write;

/// Typst のモジュールのファイル名。
pub const MODULE_FILE: &str = "iwafune.typ";

/// 表と図の関数を持つ Typst のモジュール。
pub const MODULE: &str = include_str!("template/iwafune.typ");

/// 書き出した項目をすべて呼ぶ例の文書（`typst compile example.typ` でそのまま PDF になる）。
pub fn example(data: &ReportData) -> String {
    let lang = match data.lang {
        Lang::Ja => "ja",
        Lang::En => "en",
    };
    let mut s = String::new();
    let _ = write!(
        s,
        "// Example document: includes every exported item. Write your own text around the calls.\n\
         // Build it with `typst compile example.typ` (tested with Typst 0.15.1; for Japanese text set a CJK font with `#set text(font: …)`).\n\
         #import \"{}\": *\n\
         #set page(paper: \"a4\", margin: 20mm)\n\
         #set text(lang: \"{lang}\", size: 10pt)\n\
         #set heading(numbering: \"1.1\")\n\n\
         Model revision: #raw(revision)\n",
        MODULE_FILE
    );
    if data.values.iter().any(|v| v.key == "budget.mass.total") {
        s.push_str("\nThe total dry mass is #value(\"budget.mass.total\").\n");
    }
    for item in Item::ALL.into_iter().filter(|i| data.items.contains(i)) {
        section(&mut s, item, data);
    }
    s
}

fn section(s: &mut String, item: Item, data: &ReportData) {
    let title = match item {
        Item::Mass => "Mass",
        Item::ModePower => "Power by mode",
        Item::LinkBudget => "Link budget",
        Item::ThermalResult => "Thermal results",
        Item::ThermalHistory => "Temperature history",
        Item::PowerHistory => "Power and battery history",
    };
    let call = match item {
        Item::Mass => "mass-table",
        Item::ModePower => "mode-power-table",
        Item::LinkBudget => "link-budget",
        Item::ThermalResult => "thermal-result",
        Item::ThermalHistory => "thermal-history",
        Item::PowerHistory => "power-history",
    };
    let _ = writeln!(s, "\n= {title}");
    if item.per_case() && data.cases.is_empty() {
        let _ = writeln!(s, "#{call}()");
    } else if item.per_case() {
        for case in &data.cases {
            let _ = writeln!(s, "#{call}(case: \"{case}\")");
        }
    } else {
        let _ = writeln!(s, "#{call}()");
    }
}
