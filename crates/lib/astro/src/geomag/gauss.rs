//! 球面調和関数による内部起源の地磁気（Gauss 係数 g・h、Schmidt 準正規化、基準半径つき）。
//!
//! ポテンシャルは `V = a Σₙ Σₘ (a/r)^(n+1) (gₙᵐ cos mφ + hₙᵐ sin mφ) Pₙᵐ(cos θ)`、磁束密度は `B = −∇V`。
//! 係数は元期の値と年変化 `ġ`・`ḣ`（nT/年、線形）で持つ。Legendre 関数の漸化式は重力場
//! （[`crate::force`] の V/W 漸化式）と同じものを使う（`force::gravity::GravityTable::from_schmidt`。
//! Schmidt 準正規化 → 非正規化の換算はそこに 1 か所だけある）。
//!
//! 配布ファイルの文字列形式（[`GaussModel::from_text_bytes`]・[`GaussModel::to_text`]）は docs/architecture.md の
//! 「Geomagnetic field coefficients」にある。

use crate::AstroError;
use crate::constants::{B0_EARTH, R_EARTH, SECONDS_PER_JULIAN_CENTURY};
use crate::earth::{ecef_to_eci, eci_to_ecef};
use crate::force::gravity::GravityTable;
use crate::math::Vec3;
use crate::time::Epoch;

/// 係数の最大次数（これを超えるファイルは受け付けない）。
pub const MAX_DEGREE: usize = 50;

/// 1 年の長さ [s]（ユリウス年。年変化の換算に使う）。
const SECONDS_PER_YEAR: f64 = SECONDS_PER_JULIAN_CENTURY / 100.0;

/// 1 つの次数・位数の係数。単位は nT と nT/年。
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct GaussTerm {
    /// 次数 n（1 以上）
    pub n: u8,
    /// 位数 m（0 以上 n 以下）
    pub m: u8,
    /// g（元期の値）[nT]
    pub g: f64,
    /// h（元期の値。m = 0 では 0）[nT]
    pub h: f64,
    /// ġ [nT/年]
    pub g_dot: f64,
    /// ḣ [nT/年]
    pub h_dot: f64,
}

/// 内部起源の地磁気のモデル（Gauss 係数の集合）。
#[derive(Debug, Clone, PartialEq)]
pub struct GaussModel {
    name: String,
    epoch: Epoch,
    ref_radius_m: f64,
    degree: usize,
    terms: Vec<GaussTerm>,
}

/// ある時刻に固定した地磁気（係数に年変化を足し終えたもの）。軌道上の多数の点で評価するときは、
/// 時刻ごとに 1 度だけ [`GaussModel::at`] で作って使い回す。
pub struct FieldSnapshot {
    table: GravityTable,
    ref_radius_m: f64,
    degree: usize,
    /// 次数 1 の項から決まる双極子の赤道強度 `√(g10² + g11² + h11²)` [T]（基準半径の球面上）
    dipole_b0_t: f64,
}

impl GaussModel {
    /// 係数から作る。係数は重複なし、`1 ≤ n ≤ MAX_DEGREE`、`m ≤ n`、`m = 0` では `h = 0`、基準半径は正。
    pub fn new(
        name: &str,
        epoch: Epoch,
        ref_radius_m: f64,
        terms: Vec<GaussTerm>,
    ) -> Result<Self, AstroError> {
        let bad = |msg: String| Err(AstroError::InvalidInput(msg));
        if !(ref_radius_m.is_finite() && ref_radius_m > 0.0) {
            return bad(format!("Reference radius must be positive: {ref_radius_m}"));
        }
        let mut degree = 0usize;
        let mut seen = std::collections::HashSet::new();
        for t in &terms {
            let (n, m) = (usize::from(t.n), usize::from(t.m));
            if n == 0 || n > MAX_DEGREE || m > n {
                return bad(format!("Invalid degree/order ({}, {})", t.n, t.m));
            }
            if m == 0 && (t.h != 0.0 || t.h_dot != 0.0) {
                return bad(format!("h must be 0 for order 0 (degree {n})"));
            }
            let finite = [t.g, t.h, t.g_dot, t.h_dot].iter().all(|v| v.is_finite());
            if !finite {
                return bad(format!("Non-finite coefficient at ({n}, {m})"));
            }
            if !seen.insert((n, m)) {
                return bad(format!("Duplicate coefficient at ({n}, {m})"));
            }
            degree = degree.max(n);
        }
        if degree == 0 {
            return bad("The model has no coefficients".to_string());
        }
        Ok(Self {
            name: name.to_string(),
            epoch,
            ref_radius_m,
            degree,
            terms,
        })
    }

    /// 既存の傾斜双極子（[`super::dipole_axis_ecef`]・[`B0_EARTH`]）を次数 1 の Gauss 係数で表したもの。
    ///
    /// 双極子モーメント m̂（南半球側を指す単位ベクトル。ECEF）の向きに `G = (g11, h11, g10) = B₀ₐ m̂`、
    /// `B₀ₐ = B₀ (R_E/a)³` を置く（ポテンシャル `a³ G·r̂/r²` が `B₀ R_E³ m̂·r̂/r²` に等しい）。
    /// `ref_radius_m` は基準半径 `a`。年変化は 0、元期は `epoch`。
    pub fn tilted_dipole(epoch: Epoch, ref_radius_m: f64) -> Result<Self, AstroError> {
        let m_hat = super::dipole_axis_ecef();
        let b0a_nt = B0_EARTH * (R_EARTH / ref_radius_m).powi(3) * 1e9;
        let term = |n, m, g, h| GaussTerm {
            n,
            m,
            g,
            h,
            g_dot: 0.0,
            h_dot: 0.0,
        };
        Self::new(
            "tilted dipole",
            epoch,
            ref_radius_m,
            vec![
                term(1, 0, b0a_nt * m_hat.z, 0.0),
                term(1, 1, b0a_nt * m_hat.x, b0a_nt * m_hat.y),
            ],
        )
    }

    /// モデルの名前。
    pub fn name(&self) -> &str {
        &self.name
    }

    /// 係数の元期。
    pub fn epoch(&self) -> Epoch {
        self.epoch
    }

    /// 基準半径 [m]。
    pub fn ref_radius_m(&self) -> f64 {
        self.ref_radius_m
    }

    /// 最大次数。
    pub fn degree(&self) -> usize {
        self.degree
    }

    /// 係数の一覧。
    pub fn terms(&self) -> &[GaussTerm] {
        &self.terms
    }

    /// 時刻 `epoch` に固定した地磁気。年変化は元期からの経過年（ユリウス年）に線形に掛ける（外挿も同じ）。
    pub fn at(&self, epoch: Epoch) -> FieldSnapshot {
        let years = epoch.seconds_since(self.epoch) / SECONDS_PER_YEAR;
        let value = |t: &GaussTerm| {
            (
                usize::from(t.n),
                usize::from(t.m),
                t.g + t.g_dot * years,
                t.h + t.h_dot * years,
            )
        };
        let coeffs: Vec<_> = self.terms.iter().map(value).collect();
        let dip = |m: usize| coeffs.iter().find(|c| c.0 == 1 && c.1 == m);
        let (g10, g11, h11) = (
            dip(0).map_or(0.0, |c| c.2),
            dip(1).map_or(0.0, |c| c.2),
            dip(1).map_or(0.0, |c| c.3),
        );
        FieldSnapshot {
            table: GravityTable::from_schmidt(self.ref_radius_m, self.degree, &coeffs),
            ref_radius_m: self.ref_radius_m,
            degree: self.degree,
            dipole_b0_t: (g10 * g10 + g11 * g11 + h11 * h11).sqrt() * 1e-9,
        }
    }

    /// 時刻 `epoch`・ECEF の位置 `r_ecef_m` [m] での磁束密度（ECEF）[T]。地心では 0。
    pub fn field_ecef_t(&self, r_ecef_m: Vec3, epoch: Epoch) -> Vec3 {
        self.at(epoch).field_ecef_t(r_ecef_m)
    }

    /// 時刻 `epoch`・ECI の位置 `r_eci_m` [m] での磁束密度（ECI）[T]（[`super::dipole_field_eci_t`] と同じ座標系・単位）。
    pub fn field_eci_t(&self, r_eci_m: Vec3, epoch: Epoch) -> Vec3 {
        let b_ecef = self.field_ecef_t(eci_to_ecef(r_eci_m, epoch), epoch);
        ecef_to_eci(b_ecef, epoch)
    }
}

impl FieldSnapshot {
    /// ECEF の位置 `r_ecef_m` [m] での磁束密度（ECEF）[T]。地心では 0。
    pub fn field_ecef_t(&self, r_ecef_m: Vec3) -> Vec3 {
        // 係数の単位 nT。`GravityTable` の加速度は ∇V なので B = −∇V。
        -self
            .table
            .acceleration_ecef(r_ecef_m, self.degree, self.degree, 1)
            * 1e-9
    }

    /// 基準半径 [m]。
    pub fn ref_radius_m(&self) -> f64 {
        self.ref_radius_m
    }

    /// 次数 1 の項だけから決まる双極子の赤道強度 `√(g10² + g11² + h11²)` [T]（基準半径の球面上）。
    /// 双極子モーメントは `M = b0 · a³`。
    pub fn dipole_b0_t(&self) -> f64 {
        self.dipole_b0_t
    }
}
