//! AP-8・AE-8 の束縛粒子の地図（NSSDC データセット PT-11B の ASCII 版）の読み取り。
//!
//! sat-astro はファイル I/O をしないので、ファイルは呼び出し側が読み込んで [`TrappedMap::from_ascii_bytes`] に渡す。
//!
//! # ファイルの形式
//!
//! Fortran の `(1X,12I6)`（先頭 1 文字は送り制御、残りは幅 6 の整数 12 個）で書いた整数の列。
//! 先頭の 8 語が見出し `IHEAD(1..8)`:
//!
//! | 語 | 意味 | 例 |
//! |---|---|---|
//! | 1 | 地図の種類 | 2（AP8MIN）・8（AE8MIN） |
//! | 2 | 磁場強度の刻みごとに対数フラックスが下がる量の逆数（1/4 桁） | 4 |
//! | 3 | 元期（年） | 1964・1970 |
//! | 4 | エネルギーの倍率 [MeV⁻¹]（AP-8 は 100、AE-8 は 6400） | 100 |
//! | 5 | L の倍率 | 2048（AP-8）・2100（AE-8） |
//! | 6 | B/B₀ の倍率 | 2048（AP-8）・1024（AE-8） |
//! | 7 | 対数フラックスの倍率 | 1024 |
//! | 8 | 地図の語数 | 16584（ap8min） |
//!
//! その後ろが**エネルギーごとの副地図**の並び（積分フラックス `> E`）。副地図は `[語数, E×倍率, L ごとの副副地図…]`、
//! 副副地図は `[語数, L×倍率, log₁₀F₀×倍率, B/B₀ の増分×倍率 …]`（語数は自分を含む）。`F₀` は磁力線の最小磁場の点
//! （B/B₀ = 1）の積分フラックスで、増分 `Δ₁, Δ₂, …` は「B/B₀ がそこまで増えるたびに log₁₀F が `1/IHEAD(2)` だけ下がる」幅
//! （`B/B₀ = 1 + Δ₁ + … + Δₖ` で log₁₀F = log₁₀F₀ − k/IHEAD(2)。間は直線）。増分が尽きた先はフラックス 0。
//! 各副地図は先頭に `L = 0` の見かけの副副地図（`[3, 0, 0]`）と、末尾に `L = 32767/倍率` の見かけの副副地図
//! （`[3, 32767, 0]`）を持ち、フラックスが無い L の副副地図は増分が空（`[3, L, 0]`）。最後に語数 0 で終わる。
//! 形式の定義は NASA の NSSDC RADBELT（`RADBELT.FOR` の冒頭の説明）に従う。引き方は [`super::flux`]（`TRMFUN.FOR` 1987 の手順）。
//! 語の並びはそのまま保持する（引き方が語の位置を直接たどるため）。

use crate::AstroError;

/// 地図の見出し。
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct MapHeader {
    /// 地図の種類（NASA の `MTYPE`: 2 = AP8MIN、7 = AE8MAX、8 = AE8MIN など）
    pub map_type: i32,
    /// 元期（年）。太陽活動極大の地図は 1970、極小は 1964
    pub epoch_year: i32,
    /// log₁₀F が 1 だけ下がる B/B₀ の刻みの数（`IHEAD(2)`）
    pub steps_per_decade: u32,
}

/// 読み込んだ地図。積分フラックス `> E` [cm⁻² s⁻¹] を (L, B/B₀, E) の関数として持つ（引き方は [`TrappedMap::integral_flux`]）。
#[derive(Debug, Clone, PartialEq)]
pub struct TrappedMap {
    pub(super) header: MapHeader,
    /// 見出しの 8 語（`IHEAD(1..8)`）
    pub(super) descr: [i64; 8],
    /// 見出しの後ろの語（地図の本体。`MAP(1)` が `body[0]`）
    pub(super) body: Vec<i64>,
    /// エネルギー [MeV]（昇順）
    energies: Vec<f64>,
}

fn bad(msg: impl Into<String>) -> AstroError {
    AstroError::InvalidInput(format!("Trapped-particle map: {}", msg.into()))
}

/// `(1X,12I6)` の整数の列を取り出す。
fn read_words(text: &str) -> Result<Vec<i64>, AstroError> {
    let mut words = Vec::new();
    for (i, raw) in text.lines().enumerate() {
        let line = raw.trim_end_matches('\r');
        let body = line.get(1..).unwrap_or("");
        let mut at = 0;
        while at < body.len() {
            let field = body
                .get(at..(at + 6).min(body.len()))
                .ok_or_else(|| bad(format!("line {}: not ASCII", i + 1)))?;
            at += 6;
            let t = field.trim();
            if t.is_empty() {
                continue;
            }
            words.push(
                t.parse::<i64>()
                    .map_err(|_| bad(format!("line {}: `{t}` is not an integer", i + 1)))?,
            );
        }
    }
    Ok(words)
}

impl TrappedMap {
    /// ASCII のファイル（`ap8min.asc` など）の中身から作る。形式・語数・並びが合わなければ `InvalidInput`。
    pub fn from_ascii_bytes(bytes: &[u8]) -> Result<Self, AstroError> {
        let text = std::str::from_utf8(bytes).map_err(|_| bad("not ASCII text"))?;
        let words = read_words(text)?;
        let head = words
            .get(..8)
            .ok_or_else(|| bad("the header is incomplete"))?;
        let body = words[8..].to_vec();
        let declared = usize::try_from(head[7]).map_err(|_| bad("negative word count"))?;
        if body.len() != declared {
            return Err(bad(format!(
                "the header declares {declared} words but the file has {}",
                body.len()
            )));
        }
        if head[1] <= 0 || head[3..7].iter().any(|&s| s <= 0) {
            return Err(bad("a scale in the header is not positive"));
        }
        let energies = check_structure(&body, head[3] as f64)?;
        let mut descr = [0i64; 8];
        descr.copy_from_slice(head);
        Ok(Self {
            header: MapHeader {
                map_type: i32::try_from(head[0]).map_err(|_| bad("bad map type"))?,
                epoch_year: i32::try_from(head[2]).map_err(|_| bad("bad epoch"))?,
                steps_per_decade: u32::try_from(head[1]).map_err(|_| bad("bad step count"))?,
            },
            descr,
            body,
            energies,
        })
    }

    /// 見出し。
    pub fn header(&self) -> MapHeader {
        self.header
    }

    /// 地図のエネルギー [MeV]（昇順）。
    pub fn energies_mev(&self) -> Vec<f64> {
        self.energies.clone()
    }
}

/// 副地図・副副地図の語数が全体に収まることと、エネルギーが昇順なことを確かめ、エネルギーを返す。
/// 副副地図の L の並びは確かめない（`ae8min.asc` の 5 MeV の副地図には順序の外れたフラックス無しの節点があり、
/// 引き方は NASA と同じく語の並びを先頭からたどるだけなので、そのまま持つ）。
fn check_structure(body: &[i64], energy_scale: f64) -> Result<Vec<f64>, AstroError> {
    let mut energies: Vec<f64> = Vec::new();
    let mut at = 0;
    while at < body.len() && body[at] != 0 {
        let len = usize::try_from(body[at]).map_err(|_| bad("negative sub-map length"))?;
        let sub = body
            .get(at..at + len)
            .filter(|_| len >= 2)
            .ok_or_else(|| bad("a sub-map runs past the end of the file"))?;
        let e = sub[1] as f64 / energy_scale;
        if energies.last().is_some_and(|&p| e <= p) {
            return Err(bad("the energies are not increasing"));
        }
        energies.push(e);
        let mut j = 2;
        while j < len {
            let n = usize::try_from(sub[j])
                .ok()
                .filter(|&n| n >= 3 && j + n <= len)
                .ok_or_else(|| bad("a sub-sub-map has a bad length"))?;
            j += n;
        }
        at += len;
    }
    if energies.is_empty() {
        return Err(bad("no sub-maps"));
    }
    Ok(energies)
}
