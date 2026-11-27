//! McIlwain の L と B/B₀（磁力線に沿う第 2 不変量 I から求める）。
//!
//! 手順（点 P、`Bm = |B(P)|`）:
//! 1. P から `|B|` が減る向きに磁力線を追い（[`crate::ode::Dop853`]。弧長が独立変数）、`|B|` の極小（赤道面の値 `B₀`）を通り過ぎて
//!    `|B|` が再び `Bm` に戻る点（共役のミラー点）まで進む。P と共役点が 1 組のミラー点。
//! 2. 2 点の間で `I = ∫ √(1 − B/Bm) ds` を求める。両端で被積分関数が √ の形に 0 になるので、
//!    Gauss–Chebyshev（第 2 種）の求積で扱う（節点での B は同じ磁力線の密出力から取る）。
//! 3. `X = I³B/M` から McIlwain の関数 `F`（[`super::dipole_f`]、定義どおり双極子の積分から数値で求める）を引き、
//!    `L³ = F·M/B`。L は基準半径 `a` を単位にする。
//!
//! **双極子モーメント M の取り方**（[`MomentConvention`]）。既定は **モデル自身の次数 1 の項** `M = √(g10² + g11² + h11²)·a³`
//! で、中心双極子なら L が赤道面の距離 / a にちょうど一致する。係数の元期が変わると M が（年に 0.05 % ほど）動くので、
//! 同じ点の L も L ∝ M^(1/3) だけ動く。AP-8・AE-8 のような**ある場のために作られた束縛粒子のマップ**は、そのマップが作られた
//! 固定の M で L を引く前提なので、その場合は [`MomentConvention::Fixed`] で `B₀`（赤道面強度）を固定値で与える。

use super::dipole_f::dipole_f;
use super::gauss::FieldSnapshot;
use crate::math::Vec3;
use crate::ode::Dop853;
use core::f64::consts::PI;

/// 求積の節点数（中心双極子で L が `r_eq/a` と 1e-7 以内で合う。テストで 2 倍の節点との差も確認している）。
const NODES: usize = 32;

/// L の評価で M をどう決めるか。
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum MomentConvention {
    /// モデルの次数 1 の項から決まる M（既定）。
    ModelDipole,
    /// 固定の赤道面強度 `b0` [T]（基準半径の球面上。`M = b0·a³`）。
    Fixed { b0_t: f64 },
}

/// L を求める条件。
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct LOptions {
    /// M の取り方。
    pub moment: MomentConvention,
    /// 磁力線が地心から基準半径のこの倍数を超えたら閉じていないとみなす（L の上限の目安）。
    pub max_radius_ref: f64,
}

impl Default for LOptions {
    fn default() -> Self {
        Self {
            moment: MomentConvention::ModelDipole,
            max_radius_ref: 12.0,
        }
    }
}

/// L が定義できない理由（型で返す。パニックも無限ループもしない）。
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum NoL {
    /// 点で磁場が 0（地心など）か有限でない。
    #[error("The field is zero or not finite at the point")]
    NoField,
    /// 磁力線が `max_radius_ref` までに閉じない（開いた磁力線、または L が上限より大きい）。
    #[error("The field line does not close within the distance limit")]
    OpenFieldLine,
    /// 積分が収束しない・第 2 不変量が F の表せる範囲の外。
    #[error("The invariant is outside the supported range or the integration failed")]
    OutOfRange,
    /// M が 0 以下（固定値の指定が不正、または次数 1 の項が無い）。
    #[error("The dipole moment is not positive")]
    NoMoment,
}

/// L の結果。
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct LShell {
    /// McIlwain の L（基準半径が単位）。
    pub l: f64,
    /// B/B₀（B₀ は同じ磁力線に沿う |B| の最小値）。
    pub b_over_b0: f64,
    /// 点の |B| [T]。
    pub b_t: f64,
    /// 磁力線に沿う |B| の最小値 B₀ [T]。
    pub b0_t: f64,
    /// 第 2 不変量 I [m]。
    pub i_m: f64,
    /// 共役のミラー点（ECEF）[m]。
    pub conjugate_ecef_m: Vec3,
    /// |B| が最小の点（ECEF）[m]。
    pub minimum_ecef_m: Vec3,
}

/// 磁力線を追う積分器（弧長 [m] が独立変数）。
fn tracer(ref_radius_m: f64) -> Dop853 {
    Dop853 {
        rtol: 1e-10,
        atol: 1e-4,
        max_step: 0.5 * ref_radius_m,
        max_steps: 100_000,
    }
}

impl FieldSnapshot {
    fn b_vec(&self, p: [f64; 3]) -> Vec3 {
        self.field_ecef_t(Vec3::new(p[0], p[1], p[2]))
    }

    fn b_mag(&self, p: [f64; 3]) -> f64 {
        self.b_vec(p).norm()
    }

    /// 磁力線の向きの単位ベクトル（`sigma` 倍）。場が 0 なら 0。
    fn unit(&self, p: [f64; 3], sigma: f64) -> [f64; 3] {
        let b = self.b_vec(p);
        let n = b.norm();
        if n > 0.0 && n.is_finite() {
            [sigma * b.x / n, sigma * b.y / n, sigma * b.z / n]
        } else {
            [0.0; 3]
        }
    }

    /// 磁力線に沿う |B| の微分 d|B|/ds（向き `sigma`）を、刻み `h` [m] の中心差分で求める。
    fn dbds(&self, p: [f64; 3], sigma: f64, h: f64) -> f64 {
        let u = self.unit(p, sigma);
        let at = |k: f64| [p[0] + k * u[0], p[1] + k * u[1], p[2] + k * u[2]];
        (self.b_mag(at(h)) - self.b_mag(at(-h))) / (2.0 * h)
    }

    /// 位置 `r_ecef_m` [m]（ECEF）での McIlwain の L と B/B₀。
    pub fn l_shell(&self, r_ecef_m: Vec3, opts: &LOptions) -> Result<LShell, NoL> {
        let a = self.ref_radius_m();
        let cap = opts.max_radius_ref * a;
        let p0 = [r_ecef_m.x, r_ecef_m.y, r_ecef_m.z];
        let b_m = self.b_mag(p0);
        if !(b_m > 0.0 && b_m.is_finite()) || !r_ecef_m.norm().is_finite() {
            return Err(NoL::NoField);
        }
        let b0_dipole = match opts.moment {
            MomentConvention::ModelDipole => self.dipole_b0_t(),
            MomentConvention::Fixed { b0_t } => b0_t,
        };
        if !(b0_dipole > 0.0 && b0_dipole.is_finite()) {
            return Err(NoL::NoMoment);
        }
        if r_ecef_m.norm() >= cap {
            return Err(NoL::OpenFieldLine);
        }
        let sigma = self.direction(p0, a);
        let (min_pt, s_min, b_min) = self.trace_to_minimum(p0, b_m, cap, sigma)?;
        let (conj, s_conj) = self.trace_to_conjugate(min_pt, b_m, cap, sigma)?;
        let i_m = self.second_invariant(p0, b_m, s_min + s_conj, sigma)?;
        let x = i_m.powi(3) * b_m / (b0_dipole * a.powi(3));
        let f = dipole_f(x.cbrt()).ok_or(NoL::OutOfRange)?;
        let l = (f * b0_dipole * a.powi(3) / b_m).cbrt() / a;
        let b0_t = b_min.min(b_m);
        Ok(LShell {
            l,
            b_over_b0: b_m / b0_t,
            b_t: b_m,
            b0_t,
            i_m,
            conjugate_ecef_m: Vec3::new(conj[0], conj[1], conj[2]),
            minimum_ecef_m: Vec3::new(min_pt[0], min_pt[1], min_pt[2]),
        })
    }

    /// 点から見て |B| が減る向き（+1 か −1）。
    fn direction(&self, p: [f64; 3], a: f64) -> f64 {
        if self.dbds(p, 1.0, 1e-3 * a) <= 0.0 {
            1.0
        } else {
            -1.0
        }
    }

    /// 点から |B| が減る向きに追い、|B| の極小点を返す（点、弧長、極小値）。
    fn trace_to_minimum(
        &self,
        p0: [f64; 3],
        b_m: f64,
        cap: f64,
        sigma: f64,
    ) -> Result<([f64; 3], f64, f64), NoL> {
        let a = self.ref_radius_m();
        let norm = |p: &[f64; 3]| (p[0] * p[0] + p[1] * p[1] + p[2] * p[2]).sqrt();
        // 正のあいだは「まだ減っている・まだ閉じる範囲の中」。
        let event =
            |_t: f64, p: &[f64; 3]| (-sigma * self.dbds(*p, 1.0, 1e-3 * a)).min(cap - norm(p));
        let seg = tracer(a)
            .integrate_segment(
                |_t, p| self.unit(*p, sigma),
                0.0,
                p0,
                6.0 * cap,
                None,
                event,
            )
            .map_err(|_| NoL::OutOfRange)?;
        if !seg.event || norm(&seg.y) >= cap * (1.0 - 1e-9) {
            return Err(NoL::OpenFieldLine);
        }
        Ok((seg.y, seg.t, self.b_mag(seg.y).min(b_m)))
    }

    /// 極小点から同じ向きに追い、|B| が `b_m` に戻る共役のミラー点を返す（点、極小点からの弧長）。
    fn trace_to_conjugate(
        &self,
        min_pt: [f64; 3],
        b_m: f64,
        cap: f64,
        sigma: f64,
    ) -> Result<([f64; 3], f64), NoL> {
        let a = self.ref_radius_m();
        let norm = |p: &[f64; 3]| (p[0] * p[0] + p[1] * p[1] + p[2] * p[2]).sqrt();
        let event = |_t: f64, p: &[f64; 3]| (b_m - self.b_mag(*p)).min(cap - norm(p));
        let seg = tracer(a)
            .integrate_segment(
                |_t, p| self.unit(*p, sigma),
                0.0,
                min_pt,
                6.0 * cap,
                None,
                event,
            )
            .map_err(|_| NoL::OutOfRange)?;
        if !seg.event || norm(&seg.y) >= cap * (1.0 - 1e-9) {
            return Err(NoL::OpenFieldLine);
        }
        Ok((seg.y, seg.t))
    }

    /// P から弧長 `total` までのミラー点の間の `I = ∫ √(1 − B/Bm) ds`（Gauss–Chebyshev 第 2 種の求積）。
    fn second_invariant(&self, p0: [f64; 3], b_m: f64, total: f64, sigma: f64) -> Result<f64, NoL> {
        if total <= 0.0 {
            return Ok(0.0);
        }
        let thetas: Vec<f64> = (1..=NODES)
            .map(|i| i as f64 * PI / (NODES + 1) as f64)
            .collect();
        let times: Vec<f64> = thetas
            .iter()
            .map(|t| 0.5 * total * (1.0 - t.cos()))
            .collect();
        let points = tracer(self.ref_radius_m())
            .integrate_at(|_t, p: &[f64; 3]| self.unit(*p, sigma), 0.0, p0, &times)
            .map_err(|_| NoL::OutOfRange)?;
        let sum: f64 = thetas
            .iter()
            .zip(&points)
            .map(|(th, p)| th.sin() * (1.0 - self.b_mag(*p) / b_m).max(0.0).sqrt())
            .sum();
        Ok(0.5 * total * PI / (NODES + 1) as f64 * sum)
    }
}
