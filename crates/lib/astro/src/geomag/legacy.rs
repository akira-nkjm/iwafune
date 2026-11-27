//! 古い地磁気モデルの係数ファイル（Jensen & Cain 1960、GSFC 12/66）と IGRF の配布ファイルを [`GaussModel`] に直す。
//! I/O はしない（バイト列を受け取る）。AP-8・AE-8 の束縛粒子のマップはこの古い場で B・L を引く前提なので、
//! マップごとの組み合わせ（[`crate::radiation::TrappedModel::field_id`]）がここのモデルを選ぶ。
//!
//! # 係数の規約（NSSDC データセット PT-11B の 2 つのファイル）
//!
//! どちらも行は `n m g h`（Fortran の `2I3` に続く `F11` の欄）で、**`n`・`m` は実際の次数・位数に 1 を足した値**
//! （`2 1` が `g₁⁰`。`m = 0` の行は `h` が空）。基準半径は 6371.2 km。
//!
//! - **GSFC 12/66**（[`GaussModel::from_gsfc1266_bytes`]）: IGRF と同じ Schmidt 準正規化・同じ符号（`g₁⁰ = −30401.2 nT`）。
//!   各行は `g h ġ ḣ g̈ ḧ`（nT・nT/年・nT/年²）で元期は 1960.0。指定した年の係数は
//!   `g(t) = g + ġ·Δt + ½·g̈·Δt²`（Δt = t − 1960。2 階微分は定義どおり ½ を掛ける。第 2 項の寄与は
//!   1970 年で 0.3 nT 以下の係数が大半で、双極子でも 3 nT = 0.01 % に過ぎない）。
//! - **Jensen & Cain 1960**（[`GaussModel::from_jensen_cain_bytes`]）: `g₁⁰ = +30411.2 nT` と**符号が IGRF の逆**で、
//!   Legendre 関数は Chapman & Bartels の「Gauss 規格」（`Pₙᵐ = sinᵐθ [cosⁿ⁻ᵐθ − …]`、先頭項の係数 1）。
//!   Schmidt 準正規化の `g'` への換算は
//!   `g' = −g · (n−m)! / ((2n−1)!! · Sₙₘ)`、`Sₙₘ = √((2−δₘ₀)(n−m)!/(n+m)!)`（`h` も同じ）。
//!   導出: Schmidt の関数は `Sₙₘ` を Ferrers の関数に掛けたもの、Gauss 規格は `(n−m)!/(2n−1)!!` を掛けたものなので、
//!   同じ場を表す係数は両者の比の逆数だけ違う。符号は全体で反転している。この換算は 1960 年の IGRF（DGRF）の係数と
//!   次数 4 まで 5 % 以内で（次数 1 は 0.1 % 以内で）一致することで確かめた（テスト `jensen_cain_matches_dgrf1960`
//!   は配布ファイルがあるときだけ走る）。
//!
//! # IGRF の配布ファイル
//!
//! [`IgrfTable`] は IAGA の `igrf*coeffs.txt`（見出しの行 `g/h n m 1900.0 … 2025.0 2025-30`）を読み、
//! 日付ごとに隣り合う 5 年ごとの係数の直線補間（最後の元期の後は年変化の列）で [`GaussModel`] を返す。

use super::REFERENCE_RADIUS_M;
use super::gauss::{GaussModel, GaussTerm};
use crate::AstroError;
use crate::constants::SECONDS_PER_JULIAN_CENTURY;
use crate::time::Epoch;

/// 1 年の長さ [s]（ユリウス年。`GaussModel::at` が年変化に掛ける年と同じ）。
const SECONDS_PER_YEAR: f64 = SECONDS_PER_JULIAN_CENTURY / 100.0;

fn bad(file: &str, line_no: usize, msg: &str) -> AstroError {
    AstroError::InvalidInput(format!("{file}, line {line_no}: {msg}"))
}

/// 年（小数）から元期。年の始め（1 月 1 日 0 時）で、整数の年だけを受ける（係数ファイルの元期は整数の年）。
fn epoch_of_year(year: f64) -> Epoch {
    Epoch::from_calendar(year.round() as i64, 1, 1, 0, 0, 0.0)
}

/// 1 行の先頭 6 文字（`2I3`）から実際の次数・位数（1 を引いた値）を読む。係数の行でなければ `None`。
fn degree_order(line: &str) -> Option<(u8, u8)> {
    let n = line.get(0..3)?.trim().parse::<u8>().ok()?;
    let m = line.get(3..6)?.trim().parse::<u8>().ok()?;
    (n >= 2 && m >= 1 && m <= n).then(|| (n - 1, m - 1))
}

/// 6 文字目からの `F11` の欄を数値にする（空欄は 0。欄の数が足りなければ残りも 0）。
fn fields<const K: usize>(line: &str) -> Option<[f64; K]> {
    let mut out = [0.0; K];
    for (i, v) in out.iter_mut().enumerate() {
        let from = 6 + 11 * i;
        let Some(text) = line.get(from..(from + 11).min(line.len())) else {
            continue;
        };
        let t = text.trim();
        if !t.is_empty() {
            *v = t.parse().ok()?;
        }
    }
    Some(out)
}

/// 係数の行の並び（先頭の見出しの後ろ。係数の行が途切れたら終わり）。
fn coefficient_rows(text: &str) -> impl Iterator<Item = (usize, &str, (u8, u8))> {
    text.lines()
        .enumerate()
        .skip_while(|(_, l)| l.trim().is_empty())
        .skip(1)
        .filter_map(|(i, l)| degree_order(l).map(|d| (i + 1, l, d)))
}

fn utf8<'a>(bytes: &'a [u8], file: &str) -> Result<&'a str, AstroError> {
    std::str::from_utf8(bytes)
        .map_err(|_| AstroError::InvalidInput(format!("{file} is not ASCII text")))
}

/// 先頭の（空白だけでない）行 `NN 1960.0 …` の元期の年。GSFC のファイルは先頭に空行がある。
fn header_year(text: &str, file: &str) -> Result<f64, AstroError> {
    text.lines()
        .find(|l| !l.trim().is_empty())
        .and_then(|l| l.split_whitespace().nth(1))
        .and_then(|y| y.parse::<f64>().ok())
        .ok_or_else(|| bad(file, 1, "the header has no epoch year"))
}

/// `n!`（次数 10 台までなので f64 で十分）。
fn factorial(n: u8) -> f64 {
    (1..=u32::from(n)).map(f64::from).product()
}

/// 二重階乗 `(2n−1)!! = 1·3·5⋯(2n−1)`。
fn double_factorial_odd(n: u8) -> f64 {
    (1..=u32::from(n)).map(|k| f64::from(2 * k - 1)).product()
}

/// Jensen & Cain の係数（Gauss 規格・符号反転）を Schmidt 準正規化・IGRF の符号に直す倍率 `−(n−m)!/((2n−1)!!·Sₙₘ)`。
pub(super) fn jensen_cain_factor(n: u8, m: u8) -> f64 {
    let delta = if m == 0 { 1.0 } else { 2.0 };
    let schmidt = (delta * factorial(n - m) / factorial(n + m)).sqrt();
    -factorial(n - m) / (double_factorial_odd(n) * schmidt)
}

impl GaussModel {
    /// Jensen & Cain 1960 の係数ファイル（`jensen_cain_62.dat`）から作る。元期は先頭の行の年（1960.0）、年変化は 0、
    /// 基準半径は 6371.2 km。換算は [モジュールの説明](self) の `g' = −g·(n−m)!/((2n−1)!!·Sₙₘ)`。
    pub fn from_jensen_cain_bytes(bytes: &[u8]) -> Result<Self, AstroError> {
        const FILE: &str = "Jensen & Cain coefficient file";
        let text = utf8(bytes, FILE)?;
        let epoch = epoch_of_year(header_year(text, FILE)?);
        let mut terms = Vec::new();
        for (line_no, line, (n, m)) in coefficient_rows(text) {
            let [g, h] = fields::<2>(line).ok_or_else(|| bad(FILE, line_no, "bad number"))?;
            let k = jensen_cain_factor(n, m);
            terms.push(GaussTerm {
                n,
                m,
                g: g * k,
                h: h * k,
                g_dot: 0.0,
                h_dot: 0.0,
            });
        }
        Self::new("Jensen & Cain 1960", epoch, REFERENCE_RADIUS_M, terms)
    }

    /// GSFC 12/66 の係数ファイル（`gsfc66.dat`）から、`year` 年に外挿した場を作る（元期 1960.0 の係数に
    /// `ġ·Δt + ½·g̈·Δt²` を足し終えた**固定の場**。モデルの元期は `year`、年変化は 0）。AP-8 の太陽活動極大のマップは
    /// 1970 年に外挿したこの場で B・L を引く。
    pub fn from_gsfc1266_bytes(bytes: &[u8], year: f64) -> Result<Self, AstroError> {
        const FILE: &str = "GSFC 12/66 coefficient file";
        let text = utf8(bytes, FILE)?;
        let base = header_year(text, FILE)?;
        let dt = year - base;
        let at = |c: f64, d1: f64, d2: f64| c + d1 * dt + 0.5 * d2 * dt * dt;
        let mut terms = Vec::new();
        for (line_no, line, (n, m)) in coefficient_rows(text) {
            let [g, h, gd, hd, gdd, hdd] =
                fields::<6>(line).ok_or_else(|| bad(FILE, line_no, "bad number"))?;
            terms.push(GaussTerm {
                n,
                m,
                g: at(g, gd, gdd),
                h: if m == 0 { 0.0 } else { at(h, hd, hdd) },
                g_dot: 0.0,
                h_dot: 0.0,
            });
        }
        let name = format!("GSFC 12/66 extrapolated to {year}");
        Self::new(&name, epoch_of_year(year), REFERENCE_RADIUS_M, terms)
    }
}

/// IGRF の係数の表（元期ごとの g・h と、最後の元期の後の年変化）。
#[derive(Debug, Clone, PartialEq)]
pub struct IgrfTable {
    /// 元期の年（昇順）
    years: Vec<f64>,
    /// 行ごとの `(次数, 位数, 種別が g か, 元期ごとの値, 年変化)`
    rows: Vec<IgrfRow>,
}

#[derive(Debug, Clone, PartialEq)]
struct IgrfRow {
    n: u8,
    m: u8,
    is_g: bool,
    values: Vec<f64>,
    secular: f64,
}

impl IgrfTable {
    /// 配布ファイル（`igrf14coeffs.txt` など。`#` で始まる行はコメント）から読む。
    pub fn from_bytes(bytes: &[u8]) -> Result<Self, AstroError> {
        const FILE: &str = "IGRF coefficient file";
        let text = utf8(bytes, FILE)?;
        let mut years: Vec<f64> = Vec::new();
        let mut rows = Vec::new();
        for (i, raw) in text.lines().enumerate() {
            let cols: Vec<&str> = raw.split_whitespace().collect();
            if raw.starts_with('#') || cols.len() < 5 {
                continue;
            }
            if !matches!(cols[0], "g" | "h" | "g/h") {
                continue;
            }
            if cols[0] == "g/h" {
                years = cols[3..]
                    .iter()
                    .map_while(|c| c.parse::<f64>().ok())
                    .collect();
                continue;
            }
            rows.push(
                parse_igrf_row(&cols, years.len()).ok_or_else(|| bad(FILE, i + 1, "bad row"))?,
            );
        }
        if years.len() < 2 || rows.is_empty() {
            return Err(bad(FILE, 0, "no epoch header or no coefficients"));
        }
        Ok(Self { years, rows })
    }

    /// 元期の年の範囲 `(最初, 最後)`。最後の元期の後は年変化で外挿する（配布ファイルは 5 年先まで有効とする）。
    pub fn year_range(&self) -> (f64, f64) {
        (self.years[0], self.years[self.years.len() - 1])
    }

    /// 日付 `epoch` の場。隣り合う元期の間は直線補間（その 2 つの元期の係数から `g + ġ·Δt` の形にして返すので、
    /// 返したモデルの `at(epoch)` が補間値になる）。最後の元期より後は年変化の列で外挿。最初の元期より前は `InvalidInput`。
    pub fn model_at(&self, epoch: Epoch) -> Result<GaussModel, AstroError> {
        let last = self.years.len() - 1;
        let start = epoch_of_year(self.years[0]);
        if epoch < start {
            return Err(AstroError::InvalidInput(format!(
                "IGRF starts in {}; {} is earlier",
                self.years[0],
                epoch.to_iso()
            )));
        }
        let k = self
            .years
            .iter()
            .rposition(|&y| epoch_of_year(y) <= epoch)
            .unwrap_or(0)
            .min(last);
        // 隣り合う元期の間の長さ [Julian 年]（`GaussModel::at` が年変化に掛ける年と同じ単位）
        let span = if k < last {
            epoch_of_year(self.years[k + 1]).seconds_since(epoch_of_year(self.years[k]))
                / SECONDS_PER_YEAR
        } else {
            1.0
        };
        let slope = |r: &IgrfRow| {
            if k < last {
                (r.values[k + 1] - r.values[k]) / span
            } else {
                r.secular
            }
        };
        let mut terms: Vec<GaussTerm> = Vec::new();
        for r in &self.rows {
            let t = term_mut(&mut terms, r.n, r.m);
            if r.is_g {
                (t.g, t.g_dot) = (r.values[k], slope(r));
            } else {
                (t.h, t.h_dot) = (r.values[k], slope(r));
            }
        }
        let name = format!("IGRF (epoch {})", self.years[k]);
        GaussModel::new(
            &name,
            epoch_of_year(self.years[k]),
            REFERENCE_RADIUS_M,
            terms,
        )
    }
}

/// `(n, m)` の項（無ければ 0 で足す）。
fn term_mut(terms: &mut Vec<GaussTerm>, n: u8, m: u8) -> &mut GaussTerm {
    let i = terms.iter().position(|t| t.n == n && t.m == m);
    let i = i.unwrap_or_else(|| {
        terms.push(GaussTerm {
            n,
            m,
            g: 0.0,
            h: 0.0,
            g_dot: 0.0,
            h_dot: 0.0,
        });
        terms.len() - 1
    });
    &mut terms[i]
}

fn parse_igrf_row(cols: &[&str], epochs: usize) -> Option<IgrfRow> {
    let is_g = match cols[0] {
        "g" => true,
        "h" => false,
        _ => return None,
    };
    let values: Vec<f64> = cols[3..3 + epochs]
        .iter()
        .map(|c| c.parse().ok())
        .collect::<Option<_>>()?;
    let secular = cols.get(3 + epochs)?.parse().ok()?;
    Some(IgrfRow {
        n: cols[1].parse().ok()?,
        m: cols[2].parse().ok()?,
        is_g,
        values,
        secular,
    })
}
