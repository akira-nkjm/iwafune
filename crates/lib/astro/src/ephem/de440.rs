//! JPL DE440 の数値暦（Chebyshev 係数）の読み取り。
//!
//! sat-astro はファイル I/O をしないので、データ（`de440.bin`、20.7 MB。JPL の DE440 を下の形式に変換したもの）は
//! 呼び出し側が読み込んで [`De440::from_bytes`] に渡し、[`super::install_de440`] で登録する。
//! 先頭 4 個の係数は f64、残りは f32（復号誤差は最大 5 cm）。独立変数は TDB で、入力の秒（TT とみなす）に
//! TDB − TT の主要項（最大 1.7 ms）を足して評価する。
//!
//! 形式: `"DE440BIN"`・版 u32（= 1）・区間数 u32、続いて区間ごとに 40 バイトの記述子
//! （中心 i32・対象 i32・区間数 u32・係数の数 u32・f64 の係数の数 u32・成分数 u32・開始 [s] f64・区間長 [s] f64）、
//! その後に係数の本体。すべてリトルエンディアン。

use crate::AstroError;
use crate::constants::SECONDS_PER_DAY;
use crate::math::Vec3;
use crate::time::Epoch;
use std::collections::HashMap;

const MAGIC: &[u8; 8] = b"DE440BIN";
const FORMAT_VERSION: u32 = 1;
const HEADER_BYTES: usize = 16;
const DESCRIPTOR_BYTES: usize = 40;

/// NAIF の天体番号（惑星は重心）。
pub mod body_id {
    pub const SSB: i32 = 0;
    pub const MERCURY_BARYCENTER: i32 = 1;
    pub const VENUS_BARYCENTER: i32 = 2;
    pub const EARTH_MOON_BARYCENTER: i32 = 3;
    pub const MARS_BARYCENTER: i32 = 4;
    pub const JUPITER_BARYCENTER: i32 = 5;
    pub const SATURN_BARYCENTER: i32 = 6;
    pub const URANUS_BARYCENTER: i32 = 7;
    pub const NEPTUNE_BARYCENTER: i32 = 8;
    pub const SUN: i32 = 10;
    pub const MOON: i32 = 301;
    pub const EARTH: i32 = 399;
}

#[derive(Debug, Clone)]
struct Segment {
    n_intervals: usize,
    n_coeffs: usize,
    n_f64: usize,
    components: usize,
    init_s: f64,
    interval_s: f64,
    offset: usize,
}

impl Segment {
    fn end_s(&self) -> f64 {
        self.init_s + self.interval_s * self.n_intervals as f64
    }
    fn row_bytes(&self) -> usize {
        self.n_f64 * 8 + (self.n_coeffs - self.n_f64) * 4
    }
}

/// 読み込んだ DE440 の表。
#[derive(Clone)]
pub struct De440 {
    bytes: Vec<u8>,
    segments: HashMap<(i32, i32), Segment>,
    /// 全区間が覆う TDB 秒の範囲
    span_s: (f64, f64),
}

impl core::fmt::Debug for De440 {
    /// 係数の本体（20 MB）は出さず、区間の数と期間だけを出す。
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("De440")
            .field("segments", &self.segments.len())
            .field("bytes", &self.bytes.len())
            .field("span_s", &self.span_s)
            .finish()
    }
}

fn invalid(msg: &str) -> AstroError {
    AstroError::InvalidInput(format!("Invalid DE440 data: {msg}"))
}

fn read<const N: usize>(bytes: &[u8], at: usize) -> Result<[u8; N], AstroError> {
    bytes
        .get(at..at + N)
        .and_then(|s| s.try_into().ok())
        .ok_or_else(|| invalid("truncated"))
}

impl De440 {
    /// `de440.bin` の中身から作る。形式・版・長さが合わなければ `InvalidInput`。
    pub fn from_bytes(bytes: Vec<u8>) -> Result<De440, AstroError> {
        if bytes.get(..8) != Some(MAGIC.as_slice()) {
            return Err(invalid("does not start with DE440BIN"));
        }
        let u32_at = |at: usize| read::<4>(&bytes, at).map(u32::from_le_bytes);
        if u32_at(8)? != FORMAT_VERSION {
            return Err(invalid("unsupported format version"));
        }
        let count = u32_at(12)? as usize;
        let mut offset = HEADER_BYTES + count * DESCRIPTOR_BYTES;
        let mut segments = HashMap::with_capacity(count);
        let mut span = (f64::NEG_INFINITY, f64::INFINITY);
        for i in 0..count {
            let at = HEADER_BYTES + i * DESCRIPTOR_BYTES;
            let f64_at = |k: usize| read::<8>(&bytes, k).map(f64::from_le_bytes);
            let seg = Segment {
                n_intervals: u32_at(at + 8)? as usize,
                n_coeffs: u32_at(at + 12)? as usize,
                n_f64: u32_at(at + 16)? as usize,
                components: u32_at(at + 20)? as usize,
                init_s: f64_at(at + 24)?,
                interval_s: f64_at(at + 32)?,
                offset,
            };
            let ok = seg.n_intervals > 0
                && seg.n_coeffs > 0
                && seg.n_f64 <= seg.n_coeffs
                && seg.components >= 3
                && seg.interval_s > 0.0
                && seg.init_s.is_finite();
            if !ok {
                return Err(invalid("invalid interval descriptor"));
            }
            offset += seg.components * seg.n_intervals * seg.row_bytes();
            span = (span.0.max(seg.init_s), span.1.min(seg.end_s()));
            let key = (
                read::<4>(&bytes, at).map(i32::from_le_bytes)?,
                read::<4>(&bytes, at + 4).map(i32::from_le_bytes)?,
            );
            segments.insert(key, seg);
        }
        if offset != bytes.len() {
            return Err(invalid("length does not match the descriptor"));
        }
        Ok(De440 {
            bytes,
            segments,
            span_s: span,
        })
    }

    /// 表が覆う期間（J2000 からの TDB 秒）。
    pub fn span_s(&self) -> (f64, f64) {
        self.span_s
    }

    /// `epoch` が表の期間に入るか。
    pub fn covers(&self, epoch: Epoch) -> bool {
        let tdb = tt_to_tdb_s(epoch.j2000_s);
        (self.span_s.0..=self.span_s.1).contains(&tdb)
    }

    fn coefficient(&self, seg: &Segment, component: usize, interval: usize, k: usize) -> f64 {
        let row = component * seg.n_intervals + interval;
        let base = seg.offset + row * seg.row_bytes();
        if k < seg.n_f64 {
            read::<8>(&self.bytes, base + k * 8).map_or(f64::NAN, f64::from_le_bytes)
        } else {
            let at = base + seg.n_f64 * 8 + (k - seg.n_f64) * 4;
            read::<4>(&self.bytes, at).map_or(f64::NAN, |b| f64::from(f32::from_le_bytes(b)))
        }
    }

    /// 区間 `center → target` の位置 [km]（TDB 秒）。
    fn segment_km(&self, center: i32, target: i32, tdb_s: f64) -> Option<Vec3> {
        let seg = self.segments.get(&(center, target))?;
        if !(seg.init_s..=seg.end_s()).contains(&tdb_s) {
            return None;
        }
        let raw = ((tdb_s - seg.init_s) / seg.interval_s).floor() as usize;
        let interval = raw.min(seg.n_intervals - 1);
        let t0 = seg.init_s + seg.interval_s * interval as f64;
        let x = 2.0 * (tdb_s - t0) / seg.interval_s - 1.0;
        let eval = |component: usize| {
            // Clenshaw の漸化式
            let (mut b1, mut b2) = (0.0, 0.0);
            for k in (1..seg.n_coeffs).rev() {
                let next = 2.0 * x * b1 - b2 + self.coefficient(seg, component, interval, k);
                b2 = b1;
                b1 = next;
            }
            x * b1 - b2 + self.coefficient(seg, component, interval, 0)
        };
        Some(Vec3::new(eval(0), eval(1), eval(2)))
    }

    /// 地心から天体 `target`（[`body_id`]）への位置（ECI = ICRF ≈ J2000 平均赤道）[m]。期間外は `None`。
    pub fn geocentric_position_m(&self, target: i32, epoch: Epoch) -> Option<Vec3> {
        use body_id::{EARTH, EARTH_MOON_BARYCENTER as EMB, MOON, SSB};
        let tdb = tt_to_tdb_s(epoch.j2000_s);
        let earth_from_emb = self.segment_km(EMB, EARTH, tdb)?;
        let km = if target == MOON {
            self.segment_km(EMB, MOON, tdb)? - earth_from_emb
        } else {
            let emb = self.segment_km(SSB, EMB, tdb)?;
            self.segment_km(SSB, target, tdb)? - emb - earth_from_emb
        };
        Some(km * 1000.0)
    }
}

/// TT 秒 → TDB 秒（主要 2 項。最大 1.7 ms）。
fn tt_to_tdb_s(tt_s: f64) -> f64 {
    let days = tt_s / SECONDS_PER_DAY;
    let g = (357.53 + 0.985_600_3 * days).to_radians();
    let l = (246.11 + 0.902_517_92 * days).to_radians();
    tt_s + 0.001_657 * g.sin() + 0.000_022 * l.sin()
}

#[cfg(test)]
mod tests;
