//! 足りない入力の直し方の表示（`fune analyze` などの "To fix"）。候補を決めるのは service の `fixes` で、ここは並べるだけ
//! （GUI のダイアログと同じ候補・同じ入力の例・同じ選べる値を出す）。

use crate::output::Out;
use fune_service::dto::{Choice, Fix, MissingInput, RefKind};

/// 選べる値を、最初のこの数だけ並べる（残りは件数で示す）。
const SHOWN: usize = 5;

/// 足りない入力の直し方（service の `fixes` が決めた候補のコマンドと、入力の例・選べる値）。`<…>` は打つ前に置き換える。
pub(super) fn print_fixes(missing: &[MissingInput], out: &Out) {
    if missing.iter().all(|m| m.fixes.is_empty()) {
        return;
    }
    println!("{}", out.dim("To fix (replace the <…> parts):"));
    for m in missing {
        for (i, fix) in m.fixes.iter().enumerate() {
            let head = if i == 0 { m.path.as_str() } else { "" };
            println!("  {head:<28} {}", fix.command());
            for line in hints(fix) {
                println!("  {:<28} {}", "", out.dim(line));
            }
        }
    }
}

/// 候補に付く補足の行（カタログ品の側の書き先・入力の例・選べる値・足し方）。
fn hints(fix: &Fix) -> Vec<String> {
    match fix {
        Fix::SetValue {
            catalog,
            example,
            choices,
            ..
        } => {
            let mut v = value_hints(example.as_deref(), choices);
            v.extend(
                catalog
                    .iter()
                    .map(|c| format!("or on the catalog item: {}", c.command)),
            );
            v
        }
        Fix::SetValueOnPart {
            example, choices, ..
        } => value_hints(example.as_deref(), choices),
        Fix::PickRef {
            what,
            choices,
            add_command,
            ..
        } => ref_hints(*what, choices, add_command.as_deref()),
        _ => Vec::new(),
    }
}

/// 値を打つ候補の補足: 入力の例、閉じた語彙なら選べる値。
fn value_hints(example: Option<&str>, choices: &[Choice]) -> Vec<String> {
    let mut v = Vec::new();
    if !choices.is_empty() {
        v.push(format!("one of: {}", listed(choices)));
    }
    v.extend(example.map(|e| format!("example: {e}")));
    v
}

/// 参照を選ぶ候補の補足: 選べるもの（先頭だけ）。無ければその旨と足し方。
fn ref_hints(what: RefKind, choices: &[Choice], add: Option<&str>) -> Vec<String> {
    if !choices.is_empty() {
        return vec![format!("choose from: {}", listed(choices))];
    }
    let noun = match what {
        RefKind::Part => "part",
        RefKind::CatalogItem => "catalog item",
        RefKind::CommLink => "link",
        RefKind::Maneuver => "maneuver with a dated transfer",
    };
    let mut v = vec![format!("no {noun} to choose from yet")];
    v.extend(add.map(|a| format!("add one: {a}")));
    v
}

/// 選べる値の並び（先頭の [`SHOWN`] 件と、残りの件数）。表示名が値と違えば `値 (表示名)`。
fn listed(choices: &[Choice]) -> String {
    let mut parts: Vec<String> = choices
        .iter()
        .take(SHOWN)
        .map(|c| {
            let name = c.label_en.as_deref().unwrap_or(&c.label);
            if name == c.value {
                c.value.clone()
            } else {
                format!("{} ({name})", c.value)
            }
        })
        .collect();
    if choices.len() > SHOWN {
        parts.push(format!("… {} more", choices.len() - SHOWN));
    }
    parts.join(", ")
}
