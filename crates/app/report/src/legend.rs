//! 凡例（系列の線の見本・許容範囲の破線・影の見本）。幅に収まるよう行に詰める。

use crate::palette;
use crate::spec::Chart;
use crate::svg::{LEFT, RIGHT, WIDTH, esc, num};
use std::fmt::Write;

pub(crate) const FONT: f64 = 11.0;
pub(crate) const ROW: f64 = 16.0;

/// 凡例の 1 項目。
pub(crate) struct Entry {
    sample: Sample,
    text: String,
}

enum Sample {
    Line(&'static str, &'static str),
    Limit,
    Shade,
}

pub(crate) fn entries(chart: &Chart) -> Vec<Entry> {
    // 同じ名前の系列は段をまたいでも 1 度だけ載せる（凡例の線は最初の段の型）
    let mut out: Vec<Entry> = Vec::new();
    let mut k = 0;
    for panel in &chart.panels {
        for s in &panel.series {
            k += 1;
            if !out.iter().any(|e| e.text == s.label) {
                let (c, d) = palette::series_style(k - 1);
                out.push(Entry {
                    sample: Sample::Line(c, d),
                    text: s.label.clone(),
                });
            }
        }
        for l in &panel.limits {
            out.push(Entry {
                sample: Sample::Limit,
                text: l.label.clone(),
            });
        }
    }
    if !chart.shade.is_empty() {
        out.push(Entry {
            sample: Sample::Shade,
            text: chart.shade_label.clone(),
        });
    }
    out
}

fn text_width(text: &str) -> f64 {
    text.chars()
        .map(|c| if c.is_ascii() { FONT * 0.56 } else { FONT })
        .sum()
}

/// 凡例を行に詰める（各行の項目と、その行の中の x 位置）。
pub(crate) fn rows(entries: &[Entry]) -> Vec<Vec<(f64, &Entry)>> {
    let mut rows: Vec<Vec<(f64, &Entry)>> = Vec::new();
    let mut x = LEFT;
    for e in entries {
        let w = 28.0 + text_width(&e.text) + 14.0;
        if rows.is_empty() || x + w > WIDTH - RIGHT {
            rows.push(Vec::new());
            x = LEFT;
        }
        if let Some(row) = rows.last_mut() {
            row.push((x, e));
        }
        x += w;
    }
    rows
}

pub(crate) fn draw(s: &mut String, rows: &[Vec<(f64, &Entry)>], top: f64) {
    for (r, row) in rows.iter().enumerate() {
        let y = top + r as f64 * ROW + 8.0;
        for &(x, e) in row {
            match e.sample {
                Sample::Line(color, dash) => {
                    let dash = if dash.is_empty() {
                        String::new()
                    } else {
                        format!(" stroke-dasharray=\"{dash}\"")
                    };
                    let _ = writeln!(
                        s,
                        "<line x1=\"{}\" y1=\"{y}\" x2=\"{}\" y2=\"{y}\" stroke=\"{color}\" stroke-width=\"1.6\"{dash}/>",
                        num(x),
                        num(x + 22.0),
                        y = num(y)
                    );
                }
                Sample::Limit => {
                    let _ = writeln!(
                        s,
                        "<line x1=\"{}\" y1=\"{y}\" x2=\"{}\" y2=\"{y}\" stroke=\"{}\" stroke-width=\"1\" stroke-dasharray=\"5 3\"/>",
                        num(x),
                        num(x + 22.0),
                        palette::LIMIT,
                        y = num(y)
                    );
                }
                Sample::Shade => {
                    let _ = writeln!(
                        s,
                        "<rect x=\"{}\" y=\"{}\" width=\"22\" height=\"9\" fill=\"{}\" stroke=\"{}\" stroke-width=\"0.5\"/>",
                        num(x),
                        num(y - 4.5),
                        palette::SHADE,
                        palette::AXIS
                    );
                }
            }
            let _ = writeln!(
                s,
                "<text x=\"{}\" y=\"{}\">{}</text>",
                num(x + 28.0),
                num(y + 4.0),
                esc(&e.text)
            );
        }
    }
}
