//! 表示: 表（comfy-table）・色（owo-colors）・状態の行（✔ ✖ ⚠）・JSON。
//! 見た目の規則はここだけで決める（見出しは淡色、数値は右寄せ、単位は値と一緒に表示）。

use crate::args::ColorChoice;
use comfy_table::{
    Attribute, Cell, CellAlignment, Color, ContentArrangement, LineStyle, Table, TableStyle,
};
use owo_colors::{OwoColorize, Stream};
use std::io::IsTerminal;

/// 表の線: 見出しの下の横線だけ（縦線・外枠・行の区切りは引かない）。
const STYLE: TableStyle =
    TableStyle::new().header_separator(LineStyle::none().fill('─').junction('─'));

/// 出力の設定。
#[derive(Debug, Clone, Copy)]
pub struct Out {
    pub json: bool,
    pub color: bool,
}

impl Out {
    /// 引数と環境（NO_COLOR・端末かどうか）から決める。
    pub fn new(json: bool, choice: ColorChoice) -> Self {
        let color = match choice {
            ColorChoice::Always => true,
            ColorChoice::Never => false,
            ColorChoice::Auto => {
                std::env::var_os("NO_COLOR").is_none_or(|v| v.is_empty())
                    && std::io::stdout().is_terminal()
            }
        };
        owo_colors::set_override(color);
        Self { json, color }
    }

    /// JSON を出す（整形付き）。
    pub fn json<T: serde::Serialize>(&self, value: &T) -> anyhow::Result<()> {
        println!("{}", serde_json::to_string_pretty(value)?);
        Ok(())
    }

    /// 成功の行（✔）。
    pub fn ok(&self, msg: impl std::fmt::Display) {
        println!(
            "{} {msg}",
            "✔".if_supports_color(Stream::Stdout, |t| t.green())
        );
    }

    /// 警告の行（⚠）。標準エラーに出す。
    pub fn warn(&self, msg: impl std::fmt::Display) {
        eprintln!(
            "{} {msg}",
            "⚠".if_supports_color(Stream::Stderr, |t| t.yellow())
        );
    }

    /// 失敗の行（✖）。標準エラーに出す。
    pub fn fail(&self, msg: impl std::fmt::Display) {
        eprintln!(
            "{} {msg}",
            "✖".if_supports_color(Stream::Stderr, |t| t.red())
        );
    }

    /// 見出し（太字）。
    pub fn heading(&self, text: &str) {
        println!("{}", text.if_supports_color(Stream::Stdout, |t| t.bold()));
    }

    /// 淡色の補足。
    pub fn dim(&self, text: impl std::fmt::Display) -> String {
        text.if_supports_color(Stream::Stdout, |t| t.dimmed())
            .to_string()
    }

    /// 表を作る（見出しは淡色）。
    pub fn table(&self, headers: &[&str]) -> TableOut {
        let mut table = Table::new();
        table
            .load_style(STYLE)
            .set_content_arrangement(ContentArrangement::Disabled);
        if self.color {
            table.enforce_styling();
        } else {
            table.force_no_tty();
        }
        table.set_header(
            headers
                .iter()
                .map(|h| Cell::new(h).add_attribute(Attribute::Dim)),
        );
        TableOut {
            table,
            numeric: vec![false; headers.len()],
            color: self.color,
        }
    }

    /// 差分の本文を色付きで出す。
    pub fn diff(&self, text: &str) {
        for line in text.lines() {
            let styled = if line.starts_with("+++") || line.starts_with("---") {
                line.if_supports_color(Stream::Stdout, |t| t.bold())
                    .to_string()
            } else if line.starts_with('+') {
                line.if_supports_color(Stream::Stdout, |t| t.green())
                    .to_string()
            } else if line.starts_with('-') {
                line.if_supports_color(Stream::Stdout, |t| t.red())
                    .to_string()
            } else if line.starts_with("@@") {
                line.if_supports_color(Stream::Stdout, |t| t.cyan())
                    .to_string()
            } else {
                line.to_owned()
            };
            println!("{styled}");
        }
    }
}

/// 組み立て中の表。
pub struct TableOut {
    table: Table,
    numeric: Vec<bool>,
    color: bool,
}

/// 表のセル。
pub enum C {
    /// ふつうの文字列
    Text(String),
    /// 数値（右寄せ）。単位を含めた表示文字列を渡す
    Num(String),
    /// 淡色（未設定・補足）
    Dim(String),
    /// 強調（ID など）
    Strong(String),
    /// 判定の色付き（true = 緑、false = 赤）
    Verdict(String, bool),
}

impl TableOut {
    /// 数値の列（右寄せ）を指定する。
    pub fn numeric(mut self, columns: &[usize]) -> Self {
        for &c in columns {
            if let Some(n) = self.numeric.get_mut(c) {
                *n = true;
            }
        }
        self
    }

    /// 1 行足す。
    pub fn row(&mut self, cells: Vec<C>) {
        let row: Vec<Cell> = cells
            .into_iter()
            .enumerate()
            .map(|(i, c)| self.cell(i, c))
            .collect();
        self.table.add_row(row);
    }

    fn cell(&self, column: usize, c: C) -> Cell {
        let right = matches!(c, C::Num(_)) || self.numeric.get(column).copied().unwrap_or(false);
        let align = if right {
            CellAlignment::Right
        } else {
            CellAlignment::Left
        };
        let cell = match c {
            C::Text(s) => Cell::new(s),
            C::Num(s) => Cell::new(s),
            C::Dim(s) => Cell::new(s).add_attribute(Attribute::Dim),
            C::Strong(s) => Cell::new(s).add_attribute(Attribute::Bold),
            C::Verdict(s, good) if self.color => {
                Cell::new(s).fg(if good { Color::Green } else { Color::Red })
            }
            C::Verdict(s, _) => Cell::new(s),
        };
        cell.set_alignment(align)
    }

    /// 表を標準出力に出す。
    pub fn print(self) {
        println!("{}", self.table);
    }
}
