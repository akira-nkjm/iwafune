//! ホイールの組（任意の台数・配置）への角運動量とトルクの配分。
//!
//! 軸の行列 `W = [a₁ … a_N]`（3×N、列は機体座標系の単位ベクトル）の擬似逆行列
//! `W⁺ = Wᵀ (W Wᵀ)⁻¹` で、機体の角運動量 `h` を最小ノルムで各ホイールに配る（`h_i = a_i · (W Wᵀ)⁻¹ h`）。
//! 直交 3 台でも 4 台ピラミッドでも同じ式で、軸が 3 次元を張らなければエラー。

use crate::spec::{AnalysisError, ensure_fraction, ensure_positive};
use sat_astro::math::{Mat3, Vec3};
use sat_units::typed::{AngularMomentum, Fraction, Qty, Torque};
use serde::{Deserialize, Serialize};

/// 最大角運動量のうち使ってよい割合の既定値（RW アンロード頻度・アンロード時間で共有）。**既定値はここだけ。**
pub const DEFAULT_USABLE_MOMENTUM_FRACTION: Fraction = Fraction::new(0.8);

/// 配分がこれより小さいホイールは、その向きの容量を決めない（ほぼ直交する軸の 0 割りを避ける）。
const NEGLIGIBLE_SHARE: f64 = 1e-15;

/// 最大角運動量のうち使ってよい割合（入力 `given`、省略時 [`DEFAULT_USABLE_MOMENTUM_FRACTION`]）。0 より大きく 1 以下か調べる。
pub fn usable_momentum_fraction(given: Option<Fraction>) -> Result<Fraction, AnalysisError> {
    let f = given.unwrap_or(DEFAULT_USABLE_MOMENTUM_FRACTION);
    ensure_fraction("使える割合", f.value())?;
    ensure_positive("使える割合", f.value())?;
    Ok(f)
}

/// リアクションホイール 1 台。
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct RwWheel {
    pub id: String,
    /// 回転軸（機体座標系。長さは問わず、正規化して使う）
    pub axis: Vec3,
    /// 最大角運動量
    pub max_momentum: Qty<AngularMomentum>,
    /// 最大トルク
    pub max_torque: Qty<Torque>,
}

/// 配分の準備ができたホイールの組。
#[derive(Debug, Clone)]
pub struct WheelSet {
    axes: Vec<Vec3>,
    /// `(W Wᵀ)⁻¹`
    gram_inv: Mat3,
}

impl WheelSet {
    /// 軸を正規化して擬似逆行列を用意する。
    pub fn new(wheels: &[RwWheel]) -> Result<Self, AnalysisError> {
        let mut axes = Vec::with_capacity(wheels.len());
        for w in wheels {
            ensure_positive(
                &format!("ホイール {} の最大角運動量", w.id),
                w.max_momentum.value(),
            )?;
            ensure_positive(
                &format!("ホイール {} の最大トルク", w.id),
                w.max_torque.value(),
            )?;
            let axis = w.axis.normalized().ok_or_else(|| {
                AnalysisError::InvalidInput(format!("ホイール {} の軸がゼロベクトルです", w.id))
            })?;
            axes.push(axis);
        }
        let gram = axes
            .iter()
            .fold(Mat3::default(), |m, &a| m + Mat3::outer(a, a));
        let gram_inv = gram.inverse().ok_or_else(|| {
            AnalysisError::InvalidInput(
                "ホイールの軸が 3 軸を張りません（3 台以上で、同じ平面に並ばない配置にしてください）"
                    .to_owned(),
            )
        })?;
        Ok(Self { axes, gram_inv })
    }

    /// 機体座標系のベクトル（角運動量やトルク）を各ホイールの軸まわりの成分に配る（最小ノルム）。
    pub fn distribute(&self, v_body: Vec3) -> Vec<f64> {
        let g = self.gram_inv.mul_vec(v_body);
        self.axes.iter().map(|a| a.dot(g)).collect()
    }

    /// 方向 `dir_body` に蓄えられる角運動量の上限 [N·m·s]（各ホイールの上限 `limits` [N·m·s] のどれかに当たるまで）。
    pub fn capacity_along(&self, dir_body: Vec3, limits: &[f64]) -> f64 {
        let Some(d) = dir_body.normalized() else {
            return 0.0;
        };
        self.distribute(d)
            .iter()
            .zip(limits)
            .filter(|(share, _)| share.abs() > NEGLIGIBLE_SHARE)
            .map(|(share, lim)| lim / share.abs())
            .fold(f64::INFINITY, f64::min)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn wheel(id: &str, axis: Vec3) -> RwWheel {
        RwWheel {
            id: id.to_owned(),
            axis,
            max_momentum: Qty::from_raw(0.02),
            max_torque: Qty::from_raw(0.002),
        }
    }

    #[test]
    fn orthogonal_triad_is_identity() {
        let set = WheelSet::new(&[
            wheel("X", Vec3::X),
            wheel("Y", Vec3::Y),
            wheel("Z", Vec3::Z),
        ])
        .unwrap();
        let h = set.distribute(Vec3::new(0.001, -0.002, 0.003));
        assert!(
            (h[0] - 0.001).abs() < 1e-15
                && (h[1] + 0.002).abs() < 1e-15
                && (h[2] - 0.003).abs() < 1e-15
        );
        let lim = [0.016; 3];
        // 対角方向は各軸が 1/√3 ずつ持つので √3 倍まで入る
        let c = set.capacity_along(Vec3::new(1.0, 1.0, 1.0), &lim);
        assert!((c - 0.016 * 3f64.sqrt()).abs() < 1e-12);
    }

    #[test]
    fn pyramid_reconstructs_and_shares() {
        // 傾き β のピラミッド 4 台: 軸 = (±cos β, 0, sin β), (0, ±cos β, sin β)
        let b = 54.74f64.to_radians();
        let (c, s) = (b.cos(), b.sin());
        let wheels = [
            wheel("A", Vec3::new(c, 0.0, s)),
            wheel("B", Vec3::new(-c, 0.0, s)),
            wheel("C", Vec3::new(0.0, c, s)),
            wheel("D", Vec3::new(0.0, -c, s)),
        ];
        let set = WheelSet::new(&wheels).unwrap();
        // z 方向の h は 4 台で等分: h_i = h / (4 sin β)
        let h = set.distribute(Vec3::new(0.0, 0.0, 0.01));
        for hi in &h {
            assert!((hi - 0.01 / (4.0 * s)).abs() < 1e-12);
        }
        // W h_w で元に戻る
        let mut back = Vec3::ZERO;
        let axes: Vec<Vec3> = wheels
            .iter()
            .map(|w| w.axis.normalized().unwrap())
            .collect();
        for (a, hi) in axes
            .iter()
            .zip(set.distribute(Vec3::new(0.003, -0.001, 0.002)))
        {
            back += *a * hi;
        }
        assert!((back - Vec3::new(0.003, -0.001, 0.002)).norm() < 1e-14);
    }

    #[test]
    fn coplanar_wheels_are_rejected() {
        assert!(WheelSet::new(&[wheel("X", Vec3::X), wheel("Y", Vec3::Y)]).is_err());
    }
}
