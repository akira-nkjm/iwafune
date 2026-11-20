//! 折れ線グラフの SVG（手書きのテキスト。印刷向けに白地・細い線）。
//! 横軸を共有する縦に並んだ段（panel）、影（蝕）の区間、許容範囲の破線、凡例を描く。
//! 座標は小数 2 桁に固定し、系列・目盛りは入力の順に書くので、同じ入力なら同じバイト列になる。

use crate::legend::{self, FONT};
use crate::model::{PanelSummary, SeriesSummary};
use crate::palette;
pub use crate::spec::{Chart, Limit, Panel, Series};
use crate::svg::{LEFT, RIGHT, WIDTH, esc, num};
use crate::ticks::Axis;
use std::fmt::Write;

const TOP: f64 = 10.0;
const PLOT_H: f64 = 220.0;
const PANEL_GAP: f64 = 22.0;
const AXIS_BLOCK: f64 = 40.0;
const FAMILY: &str = "Noto Sans JP, Noto Sans, DejaVu Sans, Arial, sans-serif";

/// 各段の系列の要約（`data.json` に入れる）。
pub fn summarize(chart: &Chart) -> Vec<PanelSummary> {
    chart
        .panels
        .iter()
        .map(|p| PanelSummary {
            y_label: p.y_label.clone(),
            series: p.series.iter().filter_map(summary).collect(),
        })
        .collect()
}

fn summary(s: &Series) -> Option<SeriesSummary> {
    let ys: Vec<f64> = s.y.iter().copied().filter(|v| v.is_finite()).collect();
    let min = ys.iter().copied().reduce(f64::min)?;
    let max = ys.iter().copied().reduce(f64::max)?;
    Some(SeriesSummary {
        label: s.label.clone(),
        points: ys.len(),
        min,
        max,
    })
}

/// SVG の文字列を作る。
pub fn render(chart: &Chart) -> String {
    let entries = legend::entries(chart);
    let rows = legend::rows(&entries);
    let n = chart.panels.len().max(1) as f64;
    let plot_total = n * PLOT_H + (n - 1.0) * PANEL_GAP;
    let height = TOP + plot_total + AXIS_BLOCK + rows.len() as f64 * legend::ROW + 6.0;
    let x_axis = x_axis(chart);
    let mut s = String::new();
    let _ = write!(
        s,
        "<svg xmlns=\"http://www.w3.org/2000/svg\" viewBox=\"0 0 {w} {h}\" width=\"{w}\" height=\"{h}\" font-family=\"{FAMILY}\" font-size=\"{FONT}\" fill=\"{t}\">\n<rect width=\"{w}\" height=\"{h}\" fill=\"#FFFFFF\"/>\n",
        w = num(WIDTH),
        h = num(height),
        t = palette::TEXT
    );
    for (i, panel) in chart.panels.iter().enumerate() {
        let top = TOP + i as f64 * (PLOT_H + PANEL_GAP);
        let last = i + 1 == chart.panels.len();
        let base: usize = chart.panels[..i].iter().map(|p| p.series.len()).sum();
        draw_panel(&mut s, (i, base), panel, chart, &x_axis, (top, last));
    }
    let x_label_y = TOP + plot_total + 34.0;
    let _ = writeln!(
        s,
        "<text x=\"{}\" y=\"{}\" text-anchor=\"middle\">{}</text>",
        num(LEFT + (WIDTH - LEFT - RIGHT) / 2.0),
        num(x_label_y),
        esc(&chart.x_label)
    );
    legend::draw(&mut s, &rows, TOP + plot_total + AXIS_BLOCK + 4.0);
    s.push_str("</svg>\n");
    s
}

/// 横軸（全系列と影の範囲から）。
fn x_axis(chart: &Chart) -> Axis {
    let mut lo = f64::INFINITY;
    let mut hi = f64::NEG_INFINITY;
    for s in chart.panels.iter().flat_map(|p| &p.series) {
        for &x in s.x.iter().filter(|x| x.is_finite()) {
            lo = lo.min(x);
            hi = hi.max(x);
        }
    }
    if !lo.is_finite() {
        return Axis::nice(0.0, 1.0, 6);
    }
    let a = Axis::nice(lo, hi, 7);
    // データの範囲を超えて軸を伸ばさない（時間は 0 から終わりまで）
    Axis {
        lo: lo.max(a.lo),
        hi: hi.min(a.hi).max(lo + a.step * 1e-9),
        step: a.step,
    }
}

fn y_axis(panel: &Panel) -> Axis {
    let mut lo = f64::INFINITY;
    let mut hi = f64::NEG_INFINITY;
    let limits = panel
        .limits
        .iter()
        .flat_map(|l| l.lo.into_iter().chain(l.hi));
    let points = panel
        .series
        .iter()
        .flat_map(|s| s.y.iter().copied())
        .filter(|v| v.is_finite());
    for v in points.chain(limits) {
        lo = lo.min(v);
        hi = hi.max(v);
    }
    if !lo.is_finite() {
        return Axis::nice(0.0, 1.0, 5);
    }
    Axis::nice(lo, hi, 5)
}

/// 1 段の座標の変換（データ → 画面）。
struct Frame {
    x0: f64,
    x1: f64,
    top: f64,
    xa: Axis,
    ya: Axis,
}

impl Frame {
    fn px(&self, x: f64) -> f64 {
        self.x0 + (x - self.xa.lo) / (self.xa.hi - self.xa.lo) * (self.x1 - self.x0)
    }

    fn py(&self, y: f64) -> f64 {
        self.top + PLOT_H - (y - self.ya.lo) / (self.ya.hi - self.ya.lo) * PLOT_H
    }
}

fn draw_panel(
    s: &mut String,
    (i, base): (usize, usize),
    panel: &Panel,
    chart: &Chart,
    xa: &Axis,
    (top, last): (f64, bool),
) {
    let f = Frame {
        x0: LEFT,
        x1: WIDTH - RIGHT,
        top,
        xa: xa.clone(),
        ya: y_axis(panel),
    };
    let _ = writeln!(
        s,
        "<clipPath id=\"c{i}\"><rect x=\"{}\" y=\"{}\" width=\"{}\" height=\"{}\"/></clipPath>",
        num(f.x0),
        num(top),
        num(f.x1 - f.x0),
        num(PLOT_H)
    );
    draw_shade(s, &f, &chart.shade);
    draw_y_grid(s, &f);
    draw_x_ticks(s, &f, last);
    draw_limits(s, &f, &panel.limits);
    for (k, series) in panel.series.iter().enumerate() {
        draw_series(s, i, base + k, series, &f);
    }
    let _ = writeln!(
        s,
        "<rect x=\"{}\" y=\"{}\" width=\"{}\" height=\"{}\" fill=\"none\" stroke=\"{}\" stroke-width=\"0.8\"/>",
        num(f.x0),
        num(top),
        num(f.x1 - f.x0),
        num(PLOT_H),
        palette::AXIS
    );
    let cy = top + PLOT_H / 2.0;
    let _ = writeln!(
        s,
        "<text transform=\"rotate(-90 {cx} {cy})\" x=\"{cx}\" y=\"{cy}\" text-anchor=\"middle\">{}</text>",
        esc(&panel.y_label),
        cx = num(13.0),
        cy = num(cy)
    );
}

fn draw_shade(s: &mut String, f: &Frame, shade: &[(f64, f64)]) {
    for &(a, b) in shade {
        let (a, b) = (f.px(a.max(f.xa.lo)), f.px(b.min(f.xa.hi)));
        if b > a {
            let _ = writeln!(
                s,
                "<rect x=\"{}\" y=\"{}\" width=\"{}\" height=\"{}\" fill=\"{}\"/>",
                num(a),
                num(f.top),
                num(b - a),
                num(PLOT_H),
                palette::SHADE
            );
        }
    }
}

fn draw_y_grid(s: &mut String, f: &Frame) {
    for t in f.ya.ticks() {
        let _ = writeln!(
            s,
            "<line x1=\"{}\" y1=\"{y}\" x2=\"{}\" y2=\"{y}\" stroke=\"{}\" stroke-width=\"0.5\"/>\n<text x=\"{}\" y=\"{}\" text-anchor=\"end\">{}</text>",
            num(f.x0),
            num(f.x1),
            palette::GRID,
            num(f.x0 - 5.0),
            num(f.py(t) + 4.0),
            esc(&f.ya.label(t)),
            y = num(f.py(t))
        );
    }
}

fn draw_limits(s: &mut String, f: &Frame, limits: &[Limit]) {
    for limit in limits {
        for v in limit.lo.into_iter().chain(limit.hi) {
            let _ = writeln!(
                s,
                "<line x1=\"{}\" y1=\"{y}\" x2=\"{}\" y2=\"{y}\" stroke=\"{}\" stroke-width=\"1\" stroke-dasharray=\"5 3\"/>",
                num(f.x0),
                num(f.x1),
                palette::LIMIT,
                y = num(f.py(v))
            );
        }
    }
}

fn draw_x_ticks(s: &mut String, f: &Frame, last: bool) {
    let bottom = f.top + PLOT_H;
    let xa = &f.xa;
    for t in xa.ticks() {
        if t < xa.lo - xa.step * 1e-9 || t > xa.hi + xa.step * 1e-9 {
            continue;
        }
        let x = f.px(t);
        let _ = writeln!(
            s,
            "<line x1=\"{x}\" y1=\"{}\" x2=\"{x}\" y2=\"{}\" stroke=\"{}\" stroke-width=\"0.8\"/>",
            num(bottom),
            num(bottom + 4.0),
            palette::AXIS,
            x = num(x)
        );
        if last {
            let _ = writeln!(
                s,
                "<text x=\"{}\" y=\"{}\" text-anchor=\"middle\">{}</text>",
                num(x),
                num(bottom + 16.0),
                esc(&xa.label(t))
            );
        }
    }
}

fn draw_series(s: &mut String, panel: usize, k: usize, series: &Series, f: &Frame) {
    let (color, dash) = palette::series_style(k);
    let points: Vec<String> = series
        .x
        .iter()
        .zip(&series.y)
        .filter(|(x, y)| x.is_finite() && y.is_finite())
        .map(|(&x, &y)| format!("{},{}", num(f.px(x)), num(f.py(y))))
        .collect();
    if points.is_empty() {
        return;
    }
    let dash = if dash.is_empty() {
        String::new()
    } else {
        format!(" stroke-dasharray=\"{dash}\"")
    };
    let _ = writeln!(
        s,
        "<polyline clip-path=\"url(#c{panel})\" fill=\"none\" stroke=\"{color}\" stroke-width=\"1.2\"{dash} points=\"{}\"/>",
        points.join(" ")
    );
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample() -> Chart {
        Chart {
            x_label: "Time [h]".into(),
            panels: vec![Panel {
                y_label: "Power [W]".into(),
                series: vec![Series {
                    label: "Load <a>".into(),
                    x: vec![0.0, 1.0, 2.0],
                    y: vec![1.0, 3.0, f64::NAN],
                }],
                limits: vec![Limit {
                    label: "Limit".into(),
                    lo: None,
                    hi: Some(4.0),
                }],
            }],
            shade: vec![(0.5, 1.5)],
            shade_label: "Eclipse".into(),
        }
    }

    #[test]
    fn render_is_deterministic_and_escapes_text() {
        let a = render(&sample());
        assert_eq!(a, render(&sample()));
        assert!(a.starts_with("<svg "));
        assert!(a.contains("Load &lt;a&gt;"));
        // 有限でない点は線に入らない
        assert!(a.contains("points=\""));
        assert!(!a.contains("NaN"));
    }

    #[test]
    fn summary_skips_non_finite_points() {
        let s = summarize(&sample());
        assert_eq!(s[0].series[0].points, 2);
        assert_eq!(s[0].series[0].max, 3.0);
    }

    #[test]
    fn num_trims_zeros() {
        assert_eq!(num(1.5), "1.5");
        assert_eq!(num(2.0), "2");
        assert_eq!(num(-0.001), "0");
    }
}
