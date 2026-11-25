//! 箱型の外殻の内側の放射交換（灰色・拡散面の閉空間）。**閉空間の放射の交換面積（Gebhart 係数）の計算はここだけ。**
//!
//! 6 面の形態係数は `sat_astro::view_factor`、連立方程式は `sat_astro::math::solve_dense`。節点は知らない
//! （面と節点の対応は `network::couplings`）。内部の部品による視線の遮蔽は扱わない。
//!
//! Gebhart 係数 `B_ij` は「面 i が出した放射のうち、反射を重ねて最終的に面 j が吸収する割合」で、
//! `B_ij = F_ij ε_j + Σ_k F_ik (1 − ε_k) B_kj`。面 i, j の放射交換面積は `ε_i A_i B_ij`（i ⇄ j で対称）で、
//! 面 i から面 j への正味の熱流は `σ ε_i A_i B_ij (T_i⁴ − T_j⁴)`。

use crate::structure::interference::InterferenceBox;
use sat_astro::math::{Vec3, solve_dense};
use sat_astro::view_factor::box_faces;
use sat_units::typed::{Length, Qty};
use serde::{Deserialize, Serialize};

/// 箱の寸法（機体座標の X・Y・Z 方向の長さ）。
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct ThermalBoxSize {
    pub x: Qty<Length>,
    pub y: Qty<Length>,
    pub z: Qty<Length>,
}

/// 箱型の外殻の内面の放射モデルの入力。
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct ThermalEnclosure {
    /// 箱の寸法（機体の根の部品の外形。決まらなければ `None` で、外殻の放射は使わない）
    #[serde(default)]
    pub size: Option<ThermalBoxSize>,
    /// 内面の赤外の放射率（6 面で共通。0 を超え 1 以下）
    pub emissivity: f64,
    /// 箱の中心（機体座標系。機体の根の部品の箱の中心）。無ければ原点。外面が箱の面の上にあるかの判断に使う
    #[serde(default)]
    pub center: Option<Vec3>,
    /// 部品の箱（機体座標系。いま有効な形態の配置）。外面の部品の箱の位置を調べるのに使う。
    /// 空なら位置は調べない（法線と面積だけで 6 面の節点を決める）
    #[serde(default)]
    pub boxes: Vec<InterferenceBox>,
}

/// 箱の 6 面（`BOX_FACES` の順）の面積・形態係数・放射の交換面積。
#[derive(Debug, Clone, PartialEq)]
pub struct BoxExchange {
    /// 面積 [m²]
    pub area_m2: Vec<f64>,
    /// 形態係数 `F_ij`
    pub view_factors: Vec<Vec<f64>>,
    /// 放射の交換面積 `ε_i A_i B_ij` [m²]（対称。対角は 0）
    pub exchange_m2: Vec<Vec<f64>>,
}

/// Gebhart 係数 `B_ij`。`view` は形態係数（n×n）、`emissivity` は面ごとの放射率。
/// 大きさが合わない・放射率が 0 を超え 1 以下でない・特異なときは `None`。
pub fn gebhart(view: &[Vec<f64>], emissivity: &[f64]) -> Option<Vec<Vec<f64>>> {
    let n = emissivity.len();
    if view.len() != n || view.iter().any(|r| r.len() != n) {
        return None;
    }
    if !emissivity.iter().all(|e| *e > 0.0 && *e <= 1.0) {
        return None;
    }
    // (I − F·diag(ρ)) B = F·diag(ε)  を列ごとに解く
    let mut m = vec![0.0; n * n];
    for i in 0..n {
        for k in 0..n {
            m[i * n + k] = f64::from(u8::from(i == k)) - view[i][k] * (1.0 - emissivity[k]);
        }
    }
    let mut b = vec![vec![0.0; n]; n];
    for j in 0..n {
        let rhs: Vec<f64> = (0..n).map(|i| view[i][j] * emissivity[j]).collect();
        let col = solve_dense(m.clone(), rhs)?;
        for i in 0..n {
            b[i][j] = col[i];
        }
    }
    Some(b)
}

/// 放射の交換面積 `ε_i A_i B_ij` [m²]。`area_m2` は面ごとの面積。入力が不正なときは `None`。
pub fn exchange_areas(
    view: &[Vec<f64>],
    area_m2: &[f64],
    emissivity: &[f64],
) -> Option<Vec<Vec<f64>>> {
    let b = gebhart(view, emissivity)?;
    if area_m2.len() != emissivity.len() {
        return None;
    }
    Some(
        b.iter()
            .enumerate()
            .map(|(i, row)| row.iter().map(|v| emissivity[i] * area_m2[i] * v).collect())
            .collect(),
    )
}

/// 直方体の 6 面の交換（寸法 [m]・内面の放射率）。寸法が正でない・放射率が範囲外なら `None`。
pub fn box_exchange(size_m: [f64; 3], emissivity: f64) -> Option<BoxExchange> {
    if !size_m.iter().all(|d| d.is_finite() && *d > 0.0) {
        return None;
    }
    let view: Vec<Vec<f64>> = box_faces(size_m).iter().map(|r| r.to_vec()).collect();
    let area_m2: Vec<f64> = (0..6)
        .map(|i| {
            let (p, q) = match i / 2 {
                0 => (1, 2),
                1 => (0, 2),
                _ => (0, 1),
            };
            size_m[p] * size_m[q]
        })
        .collect();
    let exchange_m2 = exchange_areas(&view, &area_m2, &[emissivity; 6])?;
    Some(BoxExchange {
        area_m2,
        view_factors: view,
        exchange_m2,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn box_view(size: [f64; 3]) -> Vec<Vec<f64>> {
        box_faces(size).iter().map(|r| r.to_vec()).collect()
    }

    /// 黒体（ε = 1）なら B_ij = F_ij。
    #[test]
    fn black_enclosure_is_the_view_factor() {
        let view = box_view([0.1, 0.226, 0.366]);
        let b = gebhart(&view, &[1.0; 6]).unwrap();
        for i in 0..6 {
            for j in 0..6 {
                assert!((b[i][j] - view[i][j]).abs() < 1e-12);
            }
        }
    }

    /// 放射した分はどこかの面に吸収される: Σ_j B_ij = 1。交換面積は対称。
    #[test]
    fn gebhart_rows_sum_to_one_and_exchange_is_symmetric() {
        let size = [0.1, 0.226, 0.366];
        let e = box_exchange(size, 0.6).unwrap();
        let view = box_view(size);
        let b = gebhart(&view, &[0.6; 6]).unwrap();
        for i in 0..6 {
            assert!((b[i].iter().sum::<f64>() - 1.0).abs() < 1e-9, "row {i}");
            for j in 0..6 {
                assert!((e.exchange_m2[i][j] - e.exchange_m2[j][i]).abs() < 1e-12);
            }
        }
        // 面ごとに異なる放射率でも対称
        let eps = [0.9, 0.3, 0.5, 0.8, 0.2, 0.7];
        let areas: Vec<f64> = e.area_m2.clone();
        let x = exchange_areas(&view, &areas, &eps).unwrap();
        for i in 0..6 {
            for j in 0..6 {
                assert!((x[i][j] - x[j][i]).abs() < 1e-12, "{i} {j}");
            }
        }
    }

    /// 2 面だけの閉空間（F12 = F21 = 1、自己の形態係数 0）: 交換面積 = A / (1/ε1 + 1/ε2 − 1)（手計算）。
    /// ε1 = 0.8、ε2 = 0.4、A = 2 m² → 2 / (1.25 + 2.5 − 1) = 0.727272…
    #[test]
    fn two_surface_enclosure_by_hand() {
        let view = vec![vec![0.0, 1.0], vec![1.0, 0.0]];
        let x = exchange_areas(&view, &[2.0, 2.0], &[0.8, 0.4]).unwrap();
        let expected = 2.0 / (1.0 / 0.8 + 1.0 / 0.4 - 1.0);
        assert!((x[0][1] - expected).abs() < 1e-12);
        assert!((x[1][0] - expected).abs() < 1e-12);
    }

    /// 非常に平たい箱の ±Z 面は平行平板の極限 A / (1/ε1 + 1/ε2 − 1)（ε1 = ε2 = ε）に近づく。
    #[test]
    fn flat_box_approaches_parallel_plates() {
        let eps = 0.8;
        let e = box_exchange([1.0, 1.0, 1e-3], eps).unwrap();
        let plates = 1.0 / (2.0 / eps - 1.0);
        assert!((e.exchange_m2[4][5] / plates - 1.0).abs() < 0.01);
    }

    #[test]
    fn invalid_inputs_give_none() {
        assert!(box_exchange([1.0, 1.0, 0.0], 0.8).is_none());
        assert!(box_exchange([1.0, 1.0, 1.0], 0.0).is_none());
        assert!(box_exchange([1.0, 1.0, 1.0], 1.2).is_none());
    }
}
