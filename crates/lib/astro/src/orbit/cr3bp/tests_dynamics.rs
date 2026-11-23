//! 運動方程式・ヤコビ定数・状態遷移行列のテスト。

use super::*;
use crate::ode::Dop853;

fn earth_moon() -> Cr3bp {
    Cr3bp::from_bodies(CentralBody::Earth, Secondary::Body(CentralBody::Moon)).expect("mu")
}

fn tight() -> Dop853 {
    Dop853 {
        rtol: 1e-13,
        atol: 1e-13,
        ..Dop853::default()
    }
}

/// 6×6 の行列式（部分ピボット付きの Gauss 消去。テスト用）。
pub(super) fn det6(a: &[[f64; 6]; 6]) -> f64 {
    let mut m = *a;
    let mut det = 1.0;
    for c in 0..6 {
        let p = (c..6).fold(c, |b, r| if m[r][c].abs() > m[b][c].abs() { r } else { b });
        if m[p][c] == 0.0 {
            return 0.0;
        }
        if p != c {
            m.swap(p, c);
            det = -det;
        }
        det *= m[c][c];
        for r in (c + 1)..6 {
            let f = m[r][c] / m[c][c];
            for k in c..6 {
                m[r][k] -= f * m[c][k];
            }
        }
    }
    det
}

pub(super) fn mat_mul(x: &[[f64; 6]; 6], y: &[[f64; 6]; 6]) -> [[f64; 6]; 6] {
    let mut o = [[0.0; 6]; 6];
    for i in 0..6 {
        for j in 0..6 {
            o[i][j] = (0..6).map(|k| x[i][k] * y[k][j]).sum();
        }
    }
    o
}

/// 正準な座標 (q, p)（p = v + ẑ × q = (vx − y, vy + x, vz)）に直した行列 `T Φ T⁻¹`。
fn canonical(phi: &[[f64; 6]; 6]) -> [[f64; 6]; 6] {
    let mut tm = [[0.0; 6]; 6];
    let mut tinv = [[0.0; 6]; 6];
    for i in 0..6 {
        tm[i][i] = 1.0;
        tinv[i][i] = 1.0;
    }
    tm[3][1] = -1.0;
    tm[4][0] = 1.0;
    tinv[3][1] = 1.0;
    tinv[4][0] = -1.0;
    mat_mul(&mat_mul(&tm, phi), &tinv)
}

/// シンプレクティック性の誤差 `max |ΦcᵀJΦc − J|`（Φc は正準な座標の行列）。
pub(super) fn symplectic_error(phi: &[[f64; 6]; 6]) -> f64 {
    let pc = canonical(phi);
    let mut jm = [[0.0; 6]; 6];
    for i in 0..3 {
        jm[i][3 + i] = 1.0;
        jm[3 + i][i] = -1.0;
    }
    let mut pt = [[0.0; 6]; 6];
    for i in 0..6 {
        for j in 0..6 {
            pt[i][j] = pc[j][i];
        }
    }
    let r = mat_mul(&mat_mul(&pt, &jm), &pc);
    (0..6)
        .flat_map(|i| (0..6).map(move |j| (i, j)))
        .map(|(i, j)| (r[i][j] - jm[i][j]).abs())
        .fold(0.0, f64::max)
}

#[test]
fn gradient_and_hessian_match_finite_differences() {
    let cr = earth_moon();
    let p = [0.3, -0.4, 0.2];
    let h = 1e-6;
    let g = cr.potential_gradient(p);
    let hess = cr.potential_hessian(p);
    for i in 0..3 {
        let (mut a, mut b) = (p, p);
        a[i] += h;
        b[i] -= h;
        // 中心差分の打ち切り誤差 ~ h² · Ω''' ≲ 1e-11、丸め ~ 1e-16/h。許容 1e-8
        let fd = (cr.effective_potential(a) - cr.effective_potential(b)) / (2.0 * h);
        assert!((fd - g[i]).abs() < 1e-8, "grad {i}");
        let (ga, gb) = (cr.potential_gradient(a), cr.potential_gradient(b));
        for j in 0..3 {
            let fd = (ga[j] - gb[j]) / (2.0 * h);
            assert!((fd - hess[j][i]).abs() < 1e-8, "hess {j}{i}");
        }
    }
}

/// μ → 0: 主天体まわりの円軌道（半径 r）は、回転系では角速度 r^(−3/2) − 1 で回る。
/// 副天体の摂動は ≲ 2μ/(1−r)² · ½t² ≲ 1e-9（μ = 1e-10、t = 2、r = 0.1）。許容は 1e-8。
#[test]
fn two_body_circular_orbit_in_the_rotating_frame_for_tiny_mu() {
    let mu = 1e-10;
    let cr = Cr3bp::new(mu).expect("mu");
    let r = 0.1_f64;
    let w = r.powf(-1.5) - 1.0;
    let x0 = [r - mu, 0.0, 0.0, 0.0, r * w, 0.0];
    let t = 2.0;
    let out = cr.flow(x0, &[t], &tight()).expect("flow");
    let (px, py) = (out[0][0] + mu, out[0][1]);
    assert!((px - r * (w * t).cos()).abs() < 1e-8 && (py - r * (w * t).sin()).abs() < 1e-8);
}

/// ヤコビ定数の保存。許容 1e-9（rtol = atol = 1e-13 で積分、有界な軌道、t = 10）。
#[test]
fn jacobi_constant_is_conserved() {
    let cr = earth_moon();
    let x0 = [0.9, 0.0, 0.05, 0.0, 0.3, 0.1];
    let c0 = cr.jacobi(&x0);
    let times: Vec<f64> = (1..=20).map(|i| 0.5 * i as f64).collect();
    for s in cr.flow(x0, &times, &tight()).expect("flow") {
        assert!((cr.jacobi(&s) - c0).abs() < 1e-9, "{}", cr.jacobi(&s) - c0);
    }
}

fn stm_case() -> (Cr3bp, State6, f64) {
    (earth_moon(), [0.85, 0.05, 0.04, 0.02, -0.2, 0.05], 3.0)
}

/// 状態遷移行列 = 流れの有限差分。刻み h = 1e-5: 打ち切り ~ h²·|Φ''| ≲ 1e-9、積分誤差 / h ~ 1e-8。許容 1e-6。
#[test]
fn stm_equals_finite_differences_of_the_flow() {
    let (cr, x0, t) = stm_case();
    let (_, phi) = cr.flow_with_stm(x0, t, &tight()).expect("stm");
    let h = 1e-5;
    for j in 0..6 {
        let (mut a, mut b) = (x0, x0);
        a[j] += h;
        b[j] -= h;
        let fa = cr.flow(a, &[t], &tight()).expect("flow")[0];
        let fb = cr.flow(b, &[t], &tight()).expect("flow")[0];
        for i in 0..6 {
            let fd = (fa[i] - fb[i]) / (2.0 * h);
            assert!(
                (fd - phi[i][j]).abs() < 1e-6 * phi[i][j].abs().max(1.0),
                "{i}{j}"
            );
        }
    }
}

/// Liouville: det Φ = 1。許容 1e-8（‖Φ‖ ~ 10 の有界な軌道）。
#[test]
fn stm_determinant_is_one() {
    let (cr, x0, t) = stm_case();
    let (_, phi) = cr.flow_with_stm(x0, t, &tight()).expect("stm");
    assert!((det6(&phi) - 1.0).abs() < 1e-8);
}

/// シンプレクティック性 ΦᵀJΦ = J（正準な座標で）。許容 1e-8。
#[test]
fn stm_is_symplectic_in_canonical_coordinates() {
    let (cr, x0, t) = stm_case();
    let (_, phi) = cr.flow_with_stm(x0, t, &tight()).expect("stm");
    assert!(symplectic_error(&phi) < 1e-8);
}

/// 周期軌道のモノドロミー行列の検査（共通）: det = 1、シンプレクティック（⇒ 固有値は逆数の対）、
/// 単位の固有値の対（`M f = f`（f は軌道の接線 = 流れの右辺）と `∇Cᵀ M = ∇Cᵀ`（ヤコビ定数の勾配））。
/// 周期性の誤差 δ の分（20δ）を固有ベクトルの一致の許容に足す。許容は積分の相対誤差 ε = 1e-13 に行列の大きさ ‖M‖ を掛けたもの:
/// det は ε‖M‖‖M⁻¹‖ ≲ ε‖M‖²、シンプレクティックの誤差は ε‖M‖²、固有ベクトルの条件は ε‖M‖。行列式とシンプレクティックの係数は 10。
/// 固有ベクトルの一致の係数は 100（最初に 10 とおいたら実測が 4 倍超えた。適応刻みの大域誤差は ~10² 刻みの局所誤差の積み上げなので、係数を 100 に改めた）。
pub(super) fn check_monodromy(cr: &Cr3bp, orbit: &PeriodicOrbit) {
    let (x0, integrator) = (&orbit.state, &CorrectorConfig::default().integrator);
    let (end, m) = cr
        .flow_with_stm(*x0, orbit.period, integrator)
        .expect("stm");
    let m = &m;
    // 周期性の誤差 δ（軌道が閉じる精度）。一致を調べる量は δ に対して高々 20δ（ヘッセ行列の大きさ）ずれる。
    let delta = (0..6).map(|i| (end[i] - x0[i]).abs()).fold(0.0, f64::max);
    let norm = m.iter().flatten().fold(1.0_f64, |a, v| a.max(v.abs()));
    let eps = 1e-13;
    assert!(
        (det6(m) - 1.0).abs() < 10.0 * eps * norm * norm,
        "det {}",
        det6(m)
    );
    assert!(
        symplectic_error(m) < 10.0 * eps * norm * norm,
        "symplectic {}",
        symplectic_error(m)
    );
    let f = cr.derivative(x0);
    for i in 0..6 {
        let mf: f64 = (0..6).map(|j| m[i][j] * f[j]).sum();
        assert!(
            (mf - f[i]).abs() < 100.0 * eps * norm + 20.0 * delta,
            "M f = f, row {i}: {}",
            mf - f[i]
        );
    }
    // ∇C = 2∇Ω − 2v の成分（位置: 2Ωx 等、速度: −2v）
    let g = cr.potential_gradient([x0[0], x0[1], x0[2]]);
    let gc = [
        2.0 * g[0],
        2.0 * g[1],
        2.0 * g[2],
        -2.0 * x0[3],
        -2.0 * x0[4],
        -2.0 * x0[5],
    ];
    let gnorm = gc.iter().fold(1.0_f64, |a, v| a.max(v.abs()));
    for j in 0..6 {
        let gm: f64 = (0..6).map(|i| gc[i] * m[i][j]).sum();
        assert!(
            (gm - gc[j]).abs() < 100.0 * eps * norm * gnorm + 20.0 * delta,
            "gradC M = gradC, col {j}: {} (norm {norm:e}, gnorm {gnorm:e}, delta {delta:e})",
            gm - gc[j]
        );
    }
}
