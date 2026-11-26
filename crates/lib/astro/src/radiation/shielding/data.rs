//! SHIELDOSE-2 の線量データベース（陽子 PROTBAS2・電子と制動放射 ELBRBAS2。Seltzer, NISTIR 5477, 1994）の読み取り。
//!
//! sat-astro はファイル I/O をしないので、ファイルは呼び出し側が読み込んで [`ShieldingData::from_bytes`] に渡す。
//!
//! # ファイルの形式
//!
//! 平文。`#` で始まる行はコメント、空行は無視。`tag <名前> <文字列>` が元のデータベースの見出し（`proton`・`electron` の 2 つ）。
//! `array <名前> <次元 1> <次元 2> …` の後ろに、次元の積と**ちょうど同じ個数**の数が空白区切りで続く（最後の添字が最も速く変わる）。
//! 最後の行は `end`。名前・次元・個数・見出しの有無はすべて厳密に確かめ、1 つでも合わなければ読まない。
//!
//! | 配列 | 次元 | 意味 |
//! |---|---|---|
//! | `ep`, `rp` | 133 | 陽子の運動エネルギー [MeV] と CSDA 飛程 [g/cm²]（アルミ） |
//! | `tepn`, `fepn` | 28 | 陽子の核相互作用で中性子へ行くエネルギーの割合の表 |
//! | `tp` | 49 | 陽子の核の表の縮約エネルギー [MeV] |
//! | `zrp` | 51 | 深さ / 飛程の格子 |
//! | `dalp` | 2×49×51 | 陽子の深さ線量（[核相互作用なし, あり][E][深さ/飛程]） |
//! | `dratp` | 11×49×51 | 検出器 / アルミの線量比（[検出器][E][深さ/飛程]） |
//! | `ee`, `re`, `ye` | 81 | 電子のエネルギー [MeV]・CSDA 飛程・制動放射の収率 |
//! | `te`, `ar`, `rs` | 14 | 電子の縮約エネルギー・吸収のスケール・外挿飛程との比 |
//! | `bs`, `zre`, `zs`, `zb` | 34・51・37・47 | 電子・制動放射の深さの格子 |
//! | `dale`, `dalb` | 14×34・14×37 | 電子・制動放射の深さ線量（[E][深さ]） |
//! | `drate`, `dratb` | 11×2×14×51・11×2×14×47 | 検出器の比（[検出器][0 = 半無限・1 = 有限の板][E][深さ]） |
//!
//! 元の Fortran の `LMAXS` に当たる 34 点目（`bs` の 2.0、`dale` の 1e-7）は元のデータの文に含まれているので、そのまま持つ。

use crate::AstroError;
use std::collections::HashMap;

/// 検出器の種類の数（アルミ・黒鉛・シリコン・空気・骨・CaF₂・GaAs・LiF・SiO₂・組織・水）。
pub const DETECTOR_COUNT: usize = 11;

/// 必須の配列の名前と次元。
const REQUIRED: &[(&str, &[usize])] = &[
    ("ep", &[133]),
    ("rp", &[133]),
    ("tepn", &[28]),
    ("fepn", &[28]),
    ("tp", &[49]),
    ("zrp", &[51]),
    ("dalp", &[2, 49, 51]),
    ("dratp", &[DETECTOR_COUNT, 49, 51]),
    ("ee", &[81]),
    ("re", &[81]),
    ("ye", &[81]),
    ("te", &[14]),
    ("ar", &[14]),
    ("rs", &[14]),
    ("bs", &[34]),
    ("zre", &[51]),
    ("zs", &[37]),
    ("zb", &[47]),
    ("dale", &[14, 34]),
    ("dalb", &[14, 37]),
    ("drate", &[DETECTOR_COUNT, 2, 14, 51]),
    ("dratb", &[DETECTOR_COUNT, 2, 14, 47]),
];

/// 読み込んだ配列 1 つ。
#[derive(Debug, Clone, PartialEq)]
struct Array {
    dims: Vec<usize>,
    values: Vec<f64>,
}

/// 読み込んだ SHIELDOSE-2 のデータベース。
#[derive(Debug, Clone, PartialEq)]
pub struct ShieldingData {
    proton_tag: String,
    electron_tag: String,
    arrays: HashMap<String, Array>,
}

fn bad(msg: impl Into<String>) -> AstroError {
    AstroError::InvalidInput(format!("SHIELDOSE-2 data: {}", msg.into()))
}

/// `array` 行の次の数の列を `count` 個読む。
fn read_values(
    lines: &mut std::iter::Peekable<impl Iterator<Item = (usize, String)>>,
    name: &str,
    count: usize,
) -> Result<Vec<f64>, AstroError> {
    let mut values = Vec::with_capacity(count);
    while values.len() < count {
        let (no, line) = lines
            .next()
            .ok_or_else(|| bad(format!("array {name} ends early")))?;
        for tok in line.split_whitespace() {
            let v: f64 = tok
                .parse()
                .map_err(|_| bad(format!("line {no}: `{tok}` is not a number")))?;
            if !v.is_finite() {
                return Err(bad(format!("line {no}: `{tok}` is not finite")));
            }
            values.push(v);
        }
    }
    if values.len() != count {
        return Err(bad(format!(
            "array {name} has {} values, expected {count}",
            values.len()
        )));
    }
    Ok(values)
}

/// `array <名前> <次元…>` の行の残り（`words`）と、続く数の列を読む。名前と次元は [`REQUIRED`] と照合する。
fn read_array<'a>(
    mut words: impl Iterator<Item = &'a str>,
    lines: &mut std::iter::Peekable<impl Iterator<Item = (usize, String)>>,
    no: usize,
) -> Result<(String, Array), AstroError> {
    let name = words
        .next()
        .ok_or_else(|| bad(format!("line {no}: no name")))?;
    let dims = words
        .map(|w| w.parse::<usize>())
        .collect::<Result<Vec<_>, _>>()
        .map_err(|_| bad(format!("line {no}: bad dimensions")))?;
    let expected = REQUIRED
        .iter()
        .find(|(n, _)| *n == name)
        .ok_or_else(|| bad(format!("line {no}: unknown array `{name}`")))?;
    if dims != expected.1 {
        return Err(bad(format!(
            "line {no}: array {name} has dimensions {dims:?}, expected {:?}",
            expected.1
        )));
    }
    let values = read_values(lines, name, dims.iter().product())?;
    Ok((name.to_owned(), Array { dims, values }))
}

impl ShieldingData {
    /// ファイルの中身から作る。形式・名前・次元・個数・見出しが合わなければ `InvalidInput`。
    pub fn from_bytes(bytes: &[u8]) -> Result<Self, AstroError> {
        let text = std::str::from_utf8(bytes).map_err(|_| bad("not UTF-8 text"))?;
        let mut lines = text
            .lines()
            .enumerate()
            .map(|(i, l)| (i + 1, l.trim().to_owned()))
            .filter(|(_, l)| !l.is_empty() && !l.starts_with('#'))
            .peekable();
        let (mut proton_tag, mut electron_tag) = (None, None);
        let mut arrays: HashMap<String, Array> = HashMap::new();
        let mut ended = false;
        while let Some((no, line)) = lines.next() {
            if ended {
                return Err(bad(format!("line {no}: data after `end`")));
            }
            let mut words = line.split_whitespace();
            match words.next() {
                Some("end") => ended = true,
                Some("tag") => {
                    let which = words.next().unwrap_or("");
                    let text: String = words.collect::<Vec<_>>().join(" ");
                    let slot = match which {
                        "proton" => &mut proton_tag,
                        "electron" => &mut electron_tag,
                        _ => return Err(bad(format!("line {no}: unknown tag `{which}`"))),
                    };
                    if slot.replace(text).is_some() {
                        return Err(bad(format!("line {no}: duplicate tag `{which}`")));
                    }
                }
                Some("array") => {
                    let (name, array) = read_array(words, &mut lines, no)?;
                    if arrays.insert(name.clone(), array).is_some() {
                        return Err(bad(format!("line {no}: duplicate array `{name}`")));
                    }
                }
                _ => return Err(bad(format!("line {no}: unrecognised line"))),
            }
        }
        if !ended {
            return Err(bad("the file has no `end` line"));
        }
        if let Some((name, _)) = REQUIRED.iter().find(|(n, _)| !arrays.contains_key(*n)) {
            return Err(bad(format!("array `{name}` is missing")));
        }
        Ok(Self {
            proton_tag: proton_tag.ok_or_else(|| bad("the proton tag is missing"))?,
            electron_tag: electron_tag.ok_or_else(|| bad("the electron tag is missing"))?,
            arrays,
        })
    }

    /// 陽子データベースの元の見出し。
    pub fn proton_tag(&self) -> &str {
        &self.proton_tag
    }

    /// 電子・制動放射データベースの元の見出し。
    pub fn electron_tag(&self) -> &str {
        &self.electron_tag
    }

    /// 1 次元の配列（名前は [`REQUIRED`] にあるもの）。
    pub(super) fn vector(&self, name: &str) -> &[f64] {
        self.arrays.get(name).map_or(&[], |a| a.values.as_slice())
    }

    /// 最後の添字を除く添字 `lead` で選んだ行（最後の次元の長さの列）。範囲外は空。
    pub(super) fn row(&self, name: &str, lead: &[usize]) -> &[f64] {
        let Some(a) = self.arrays.get(name) else {
            return &[];
        };
        let (last, heads) = match a.dims.split_last() {
            Some(x) if x.1.len() == lead.len() => x,
            _ => return &[],
        };
        let mut at = 0;
        for (&i, &d) in lead.iter().zip(heads) {
            if i >= d {
                return &[];
            }
            at = at * d + i;
        }
        a.values.get(at * last..(at + 1) * last).unwrap_or(&[])
    }
}
