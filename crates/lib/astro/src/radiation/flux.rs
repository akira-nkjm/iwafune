//! 束縛粒子のマップから積分フラックス `> E` を引く。**NASA の NSSDC RADBELT（`TRMFUN.FOR` 1987 の `TRARA1`・`TRARA2`）と
//! 同じ手順**を、語の並びを直接たどる形で書き直したもの（コードの写しではなく、手順を自分の言葉で書いている）。
//! 数値が変わる細部まで合わせている:
//!
//! - 計算は単精度（`f32`）。Fortran の実数と同じ丸めになる。
//! - L は 15.6 で頭打ちにし、倍率をかけて**整数に切り捨てる**。B/B₀ も `(B/B₀ − 1)` に倍率をかけて切り捨てる（B/B₀ < 1 は 1）。
//! - 対数フラックスの刻み `FISTEP` は倍率の比 `IHEAD(7)/IHEAD(2)` の**整数除算**。
//! - エネルギー方向（[`TrappedMap::integral_flux`] = `TRARA1`。最小のエネルギーより下・最大より上は、隣り合う 2 つのサブマップからの直線の外挿）: log₁₀F を E について直線補間する。上のサブマップのフラックスが 0 で、
//!   1 つ下のサブマップがあれば、その下 2 つからの外挿との**小さい方**を取る。log₁₀F は 0 以上に保つ（F ≤ 0 はフラックス 0）。
//! - (L, B/B₀) 方向（`trara2`）: L を挟む 2 つのサブサブマップを、赤道のフラックスが大きい方が後ろ（2 番）になるように並べ、
//!   それぞれの折れ線（B の増分の累積 × 対数フラックスが刻みずつ下がる）を、原点を通る直線の傾きで比べながら両方に沿って進み、
//!   目的の B の手前の 2 点（`FKBM, FLOGM` と `FKB, FLOG`）の間を直線で結ぶ。
//!   手順の途中でフラックスが見つからない道はすべて 0。
//! - 最初のサブサブマップがすでに目的の L より大きい（`TRARA2` で下側が未定義になる）ときは、下側を空のサブサブマップ（長さ 0）とみなす。
//!   NASA のマップは先頭に `L = 0` の見かけの節点を持つので、実際のマップでは起きない。

use super::map::TrappedMap;

/// 4 語に満たないサブサブマップは空（`[長さ, L, log F₀]` だけ）。
const EMPTY_LEN: usize = 4;

/// マップの語（`MAP(k)`。1 始まり）。範囲の外は 0。
struct Words<'a>(&'a [i64]);

impl Words<'_> {
    fn at(&self, k: usize) -> i64 {
        k.checked_sub(1)
            .and_then(|i| self.0.get(i))
            .copied()
            .unwrap_or(0)
    }
    fn f(&self, k: usize) -> f32 {
        self.at(k) as f32
    }
    /// `MAP(k)` から始まる部分（Fortran で配列の途中の要素を渡すのに当たる）。
    fn from(&self, k: usize) -> Words<'_> {
        Words(self.0.get(k.saturating_sub(1)..).unwrap_or(&[]))
    }
}

/// `trara2` の手順の段（Fortran のラベルに当たる）。
#[derive(Clone, Copy)]
enum Go {
    Swap,
    Init,
    Beyond,
    Restart,
    Go32,
    Go28,
    Go29,
    Compare,
    FromLower,
    EmptyLower,
    Finish,
}

/// `TRARA2`: サブマップ `m`（最初のサブサブマップが `MAP(1)`）での、倍率つきの L `il` と B/B₀ − 1 `ib` の、倍率つきの対数フラックス。
/// フラックスが無ければ 0。
fn trara2(m: &Words<'_>, il: i64, ib: i64, fistep: f32) -> f32 {
    let (fnl, fnb) = (il as f32, ib as f32);
    // L が il 以下の最後のサブサブマップ（1 番）と、il より大きい最初のサブサブマップ（2 番）。
    let (mut i1, mut l1, mut i2) = (0usize, 0usize, 0usize);
    let mut l2;
    loop {
        l2 = usize::try_from(m.at(i2 + 1)).unwrap_or(0);
        if l2 == 0 {
            return 0.0; // 壊れたマップ（最後の見かけの節点が無い）
        }
        if m.at(i2 + 2) > il {
            break;
        }
        i1 = i2;
        l1 = l2;
        i2 += l2;
    }
    if l1 < EMPTY_LEN && l2 < EMPTY_LEN {
        return 0.0;
    }
    let mut itime = 0;
    let mut go = if m.at(i2 + 3) > m.at(i1 + 3) {
        Go::Init
    } else {
        Go::Swap
    };
    let (mut flog1, mut flog2, mut fkb1, mut fkb2) = (0.0f32, 0.0f32, 0.0f32, 0.0f32);
    let (mut fincr1, mut fincr2, mut sl1, mut sl2) = (0.0f32, 0.0f32, 0.0f32, 0.0f32);
    let (mut fkbj1, mut fkb, mut flog, mut fkbm, mut flogm, mut dfl) =
        (0.0f32, 0.0f32, 0.0f32, 0.0f32, 0.0f32, 0.0f32);
    let (mut j1, mut j2) = (4usize, 4usize);
    loop {
        match go {
            Go::Swap => {
                std::mem::swap(&mut i1, &mut i2);
                std::mem::swap(&mut l1, &mut l2);
                go = Go::Init;
            }
            Go::Init => {
                // 2 つのサブサブマップの L から内挿の重みを決め、2 番の折れ線を目的の B を越える増分まで進める。
                dfl = (fnl - m.f(i1 + 2)) / (m.f(i2 + 2) - m.f(i1 + 2));
                flog1 = m.f(i1 + 3);
                flog2 = m.f(i2 + 3);
                fkb1 = 0.0;
                fkb2 = 0.0;
                if l1 < EMPTY_LEN {
                    go = Go::Go32;
                    continue;
                }
                let mut hit = false;
                j2 = 4;
                while j2 <= l2 {
                    fincr2 = m.f(i2 + j2);
                    if fkb2 + fincr2 > fnb {
                        hit = true;
                        break;
                    }
                    fkb2 += fincr2;
                    flog2 -= fistep;
                    j2 += 1;
                }
                if hit {
                    go = Go::Beyond;
                } else {
                    itime += 1;
                    if itime != 1 {
                        return 0.0;
                    }
                    go = Go::Swap;
                }
            }
            Go::Beyond => {
                if itime == 1 {
                    go = Go::Restart;
                    continue;
                }
                if j2 == 4 {
                    go = Go::Go28;
                    continue;
                }
                // 1 番の折れ線を、2 番の傾きとの交点が 1 番の側に入るまで進める。
                sl2 = flog2 / fkb2;
                let mut broke = false;
                j1 = 4;
                while j1 <= l1 {
                    fincr1 = m.f(i1 + j1);
                    fkb1 += fincr1;
                    flog1 -= fistep;
                    fkbj1 = ((flog1 / fistep) * fincr1 + fkb1) / ((fincr1 / fistep) * sl2 + 1.0);
                    if fkbj1 <= fkb1 {
                        broke = true;
                        break;
                    }
                    j1 += 1;
                }
                if !broke && fkbj1 <= fkb2 {
                    return 0.0;
                }
                if fkbj1 <= fkb2 {
                    go = Go::Go29;
                } else {
                    fkb1 = 0.0;
                    go = Go::Restart;
                }
            }
            Go::Restart => {
                fkb2 = 0.0;
                go = Go::Go32;
            }
            Go::Go32 => {
                // 両方の折れ線を先頭からやり直す。
                j2 = 4;
                fincr2 = m.f(i2 + 4);
                flog2 = m.f(i2 + 3);
                flog1 = m.f(i1 + 3);
                go = Go::Go28;
            }
            Go::Go28 => {
                flogm = flog1 + (flog2 - flog1) * dfl;
                fkbm = 0.0;
                fkb2 += fincr2;
                flog2 -= fistep;
                sl2 = flog2 / fkb2;
                if l1 < EMPTY_LEN {
                    go = Go::EmptyLower;
                    continue;
                }
                j1 = 4;
                fincr1 = m.f(i1 + 4);
                fkb1 += fincr1;
                flog1 -= fistep;
                sl1 = flog1 / fkb1;
                go = Go::Compare;
            }
            Go::Go29 => {
                fkbm = fkbj1 + (fkb2 - fkbj1) * dfl;
                flogm = fkbm * sl2;
                flog2 -= fistep;
                fkb2 += fincr2;
                sl1 = flog1 / fkb1;
                sl2 = flog2 / fkb2;
                go = Go::Compare;
            }
            Go::Compare => {
                if sl1 < sl2 {
                    go = Go::FromLower;
                    continue;
                }
                // 1 番の傾きが 2 番以上: 2 番の折れ線の側を 1 番の傾きの直線と交わらせる。
                let fkbj2 = ((flog2 / fistep) * fincr2 + fkb2) / ((fincr2 / fistep) * sl1 + 1.0);
                fkb = fkb1 + (fkbj2 - fkb1) * dfl;
                flog = fkb * sl1;
                if fkb >= fnb {
                    go = Go::Finish;
                    continue;
                }
                fkbm = fkb;
                flogm = flog;
                if j1 >= l1 {
                    return 0.0;
                }
                j1 += 1;
                fincr1 = m.f(i1 + j1);
                flog1 -= fistep;
                fkb1 += fincr1;
                sl1 = flog1 / fkb1;
            }
            Go::FromLower => {
                fkbj1 = ((flog1 / fistep) * fincr1 + fkb1) / ((fincr1 / fistep) * sl2 + 1.0);
                fkb = fkbj1 + (fkb2 - fkbj1) * dfl;
                flog = fkb * sl2;
                if fkb >= fnb {
                    go = Go::Finish;
                    continue;
                }
                fkbm = fkb;
                flogm = flog;
                if j2 >= l2 {
                    return 0.0;
                }
                j2 += 1;
                fincr2 = m.f(i2 + j2);
                flog2 -= fistep;
                fkb2 += fincr2;
                sl2 = flog2 / fkb2;
                go = Go::Compare;
            }
            Go::EmptyLower => {
                // 1 番が空: 増分 0・傾き −900000 の仮の 1 番との比べ合いにする。
                fincr1 = 0.0;
                sl1 = -900_000.0;
                go = Go::FromLower;
            }
            Go::Finish => {
                if fkb < fkbm + 1.0e-10 {
                    return 0.0;
                }
                let r = flogm + (flog - flogm) * ((fnb - fkbm) / (fkb - fkbm));
                return r.max(0.0);
            }
        }
    }
}

impl TrappedMap {
    /// 積分フラックス `> E`（全方向）[cm⁻² s⁻¹]（`TRARA1` と、その後の `10^log₁₀F`。log₁₀F ≤ 0 はフラックス 0）。
    /// `l` は McIlwain の L、`b_over_b0` は B/B₀、`energy_mev` は [MeV]。手順の細部はモジュールの説明。
    pub fn integral_flux(&self, l: f64, b_over_b0: f64, energy_mev: f64) -> f64 {
        // 引数が有限でない・L かエネルギーが正でないときは 0（Fortran は L の絶対値を取るが、負の L は意味を持たない）
        if !(l.is_finite() && b_over_b0.is_finite() && energy_mev.is_finite())
            || l <= 0.0
            || energy_mev <= 0.0
        {
            return 0.0;
        }
        let log10_f = self.log10_flux(l as f32, b_over_b0 as f32, energy_mev as f32);
        if log10_f > 0.0 {
            10f64.powf(f64::from(log10_f))
        } else {
            0.0
        }
    }

    /// `TRARA1`（エネルギー 1 つ）: 積分フラックスの常用対数（0 以上）。
    fn log10_flux(&self, l: f32, b_over_b0: f32, e: f32) -> f32 {
        let d = &self.descr;
        let m = Words(&self.body);
        let fistep = (d[6] / d[1]) as f32; // 整数除算
        let (escale, fscale) = (d[3] as f32, d[6] as f32);
        let xnl = 15.6f32.min(l.abs());
        let nl = (xnl * d[4] as f32) as i64;
        let bb0 = if b_over_b0 < 1.0 { 1.0 } else { b_over_b0 };
        let nb = ((bb0 - 1.0) * d[5] as f32) as i64;
        // サブマップの先頭の位置（0 始まりの語の位置 = Fortran の添字 − 1）: 0 番目（E0）・1 番目（E1）・2 番目（E2）・3 番目
        let (mut i0, mut i1, mut i2) = (0usize, 0usize, m.at(1) as usize);
        let mut i3 = i2 + m.at(i2 + 1) as usize;
        let mut l3 = m.at(i3 + 1);
        let (mut e0, mut e1) = (0.0f32, m.at(2) as f32 / escale);
        let mut e2 = m.at(i2 + 2) as f32 / escale;
        while !(e <= e2 || l3 == 0) {
            (i0, i1, i2) = (i1, i2, i3);
            i3 += l3 as usize;
            l3 = m.at(i3 + 1);
            (e0, e1) = (e1, e2);
            e2 = m.at(i2 + 2) as f32 / escale;
        }
        let at = |i: usize| trara2(&m.from(i + 3), nl, nb, fistep) / fscale;
        let (f1, f2) = (at(i1), at(i2));
        let mut f = f1 + (f2 - f1) * (e - e1) / (e2 - e1);
        if (f2 <= 0.0 || f2.is_nan()) && i1 != 0 {
            // 上のサブマップにフラックスが無く、1 つ下のサブマップがあれば、下 2 つからの外挿との小さい方を取る。
            let f0 = at(i0);
            f = f.min(f0 + (f1 - f0) * (e - e0) / (e1 - e0));
        }
        f.max(0.0)
    }
}
