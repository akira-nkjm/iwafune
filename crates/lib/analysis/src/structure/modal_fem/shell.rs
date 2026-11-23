//! 4 節点の長方形の平らなシェル要素。節点あたり 6 自由度（局所軸 (u, v, n) の並進 3 と回転 3）。
//!
//! - 曲げ: Reissner–Mindlin 板。横せん断は MITC4（辺の中点で測ったせん断ひずみを内挿してロッキングを避ける）。
//! - 面内: 双線形（Q4）の平面応力。
//! - 面内回転（ドリリング）: 回転 θₙ と面内の回転 ω = ½(∂v/∂x − ∂u/∂y) の差への罰金項 γ·G·t·(θₙ − ω)²。
//!   2×2 の積分で、要素内で一様でない θₙ の分布（ω では作れない）にも剛性が付く。
//! - 質量: 対角の集中質量（要素の質量を 4 節点に等分）。回転は Mindlin 板の回転慣性 m t²/12 だけを置く。
//!
//! 局所軸 (u, v, n) は全体座標軸の巡回の並べ替え（n, u = n+1, v = n+2）なので、座標変換は要らない
//! （局所の自由度の並びを全体の自由度の添字に写すだけ）。

/// 要素の自由度の数。
pub const ELEMENT_DOF: usize = 24;
/// せん断補正係数（等方の一様な板の既定値 5/6）。
pub const SHEAR_CORRECTION: f64 = 5.0 / 6.0;

/// 要素の剛性行列（局所の自由度は節点ごとに u, v, w, θu, θv, θn）。
pub type ElementMatrix = [[f64; ELEMENT_DOF]; ELEMENT_DOF];

/// 要素の材料と板厚。
#[derive(Debug, Clone, Copy)]
pub struct ShellProps {
    pub youngs_modulus: f64,
    pub poisson_ratio: f64,
    pub thickness: f64,
    /// ドリリングの罰金の倍率（G·t に対する比）
    pub drilling_factor: f64,
}

const NODE_XI: [(f64, f64); 4] = [(-1.0, -1.0), (1.0, -1.0), (1.0, 1.0), (-1.0, 1.0)];

/// 形状関数 N_i(ξ, η)。
fn shape(i: usize, xi: f64, eta: f64) -> f64 {
    let (a, b) = NODE_XI[i];
    (1.0 + a * xi) * (1.0 + b * eta) / 4.0
}

/// ∂N_i/∂ξ と ∂N_i/∂η。
fn shape_d(i: usize, xi: f64, eta: f64) -> (f64, f64) {
    let (a, b) = NODE_XI[i];
    (a * (1.0 + b * eta) / 4.0, b * (1.0 + a * xi) / 4.0)
}

/// K += scale · Bᵀ D B（B は R 行の 24 列のひずみ行列）。
fn add_btdb<const R: usize>(
    k: &mut ElementMatrix,
    b: &[[f64; ELEMENT_DOF]; R],
    d: &[[f64; R]; R],
    scale: f64,
) {
    for r in 0..R {
        for s in 0..R {
            let w = scale * d[r][s];
            if w == 0.0 {
                continue;
            }
            for i in 0..ELEMENT_DOF {
                if b[r][i] == 0.0 {
                    continue;
                }
                for j in 0..ELEMENT_DOF {
                    k[i][j] += w * b[r][i] * b[s][j];
                }
            }
        }
    }
}

/// 面内（膜）のひずみ行列 B_m（行: εx, εy, γxy）。
fn membrane_b(a: f64, b: f64, xi: f64, eta: f64) -> [[f64; ELEMENT_DOF]; 3] {
    let mut m = [[0.0; ELEMENT_DOF]; 3];
    for i in 0..4 {
        let (dxi, deta) = shape_d(i, xi, eta);
        let (dx, dy) = (2.0 / a * dxi, 2.0 / b * deta);
        m[0][6 * i] = dx;
        m[1][6 * i + 1] = dy;
        m[2][6 * i] = dy;
        m[2][6 * i + 1] = dx;
    }
    m
}

/// 曲げのひずみ行列 B_b（行: κx = ∂θv/∂x, κy = −∂θu/∂y, κxy = ∂θv/∂y − ∂θu/∂x）。
fn bending_b(a: f64, b: f64, xi: f64, eta: f64) -> [[f64; ELEMENT_DOF]; 3] {
    let mut m = [[0.0; ELEMENT_DOF]; 3];
    for i in 0..4 {
        let (dxi, deta) = shape_d(i, xi, eta);
        let (dx, dy) = (2.0 / a * dxi, 2.0 / b * deta);
        m[0][6 * i + 4] = dx;
        m[1][6 * i + 3] = -dy;
        m[2][6 * i + 3] = -dx;
        m[2][6 * i + 4] = dy;
    }
    m
}

/// 点 (ξ, η) での ∂w/∂ξ + (a/2)θv（ξ 方向の共変せん断ひずみ）の行。
fn gamma_xi_row(a: f64, xi: f64, eta: f64) -> [f64; ELEMENT_DOF] {
    let mut r = [0.0; ELEMENT_DOF];
    for i in 0..4 {
        r[6 * i + 2] = shape_d(i, xi, eta).0;
        r[6 * i + 4] = a / 2.0 * shape(i, xi, eta);
    }
    r
}

/// 点 (ξ, η) での ∂w/∂η − (b/2)θu（η 方向の共変せん断ひずみ）の行。
fn gamma_eta_row(b: f64, xi: f64, eta: f64) -> [f64; ELEMENT_DOF] {
    let mut r = [0.0; ELEMENT_DOF];
    for i in 0..4 {
        r[6 * i + 2] = shape_d(i, xi, eta).1;
        r[6 * i + 3] = -b / 2.0 * shape(i, xi, eta);
    }
    r
}

/// MITC4 の横せん断ひずみ行列（行: γxz, γyz）。
/// γξ は η = ±1 の辺の中点 (0, ∓1) の値を η 方向に、γη は ξ = ±1 の辺の中点 (∓1, 0) の値を ξ 方向に線形に内挿する。
fn shear_b(a: f64, b: f64, xi: f64, eta: f64) -> [[f64; ELEMENT_DOF]; 2] {
    let (ga, gc) = (gamma_xi_row(a, 0.0, -1.0), gamma_xi_row(a, 0.0, 1.0));
    let (gd, gb) = (gamma_eta_row(b, -1.0, 0.0), gamma_eta_row(b, 1.0, 0.0));
    let mut m = [[0.0; ELEMENT_DOF]; 2];
    for c in 0..ELEMENT_DOF {
        m[0][c] = 2.0 / a * (0.5 * (1.0 - eta) * ga[c] + 0.5 * (1.0 + eta) * gc[c]);
        m[1][c] = 2.0 / b * (0.5 * (1.0 - xi) * gd[c] + 0.5 * (1.0 + xi) * gb[c]);
    }
    m
}

/// ドリリングの行 θₙ − ω（ω = ½(∂v/∂x − ∂u/∂y)）。
fn drilling_row(a: f64, b: f64, xi: f64, eta: f64) -> [f64; ELEMENT_DOF] {
    let mut r = [0.0; ELEMENT_DOF];
    for i in 0..4 {
        let (dxi, deta) = shape_d(i, xi, eta);
        r[6 * i] = 0.5 * (2.0 / b * deta);
        r[6 * i + 1] = -0.5 * (2.0 / a * dxi);
        r[6 * i + 5] = shape(i, xi, eta);
    }
    r
}

/// 長方形（辺 a × b、局所の u 方向 a、v 方向 b）の要素剛性行列。
pub fn stiffness(a: f64, b: f64, p: &ShellProps) -> ElementMatrix {
    let (e, nu, t) = (p.youngs_modulus, p.poisson_ratio, p.thickness);
    let g = e / (2.0 * (1.0 + nu));
    let plane = |c: f64| {
        [
            [c, c * nu, 0.0],
            [c * nu, c, 0.0],
            [0.0, 0.0, c * (1.0 - nu) / 2.0],
        ]
    };
    let d_m = plane(e * t / (1.0 - nu * nu));
    let d_b = plane(e * t.powi(3) / (12.0 * (1.0 - nu * nu)));
    let ks = SHEAR_CORRECTION * g * t;
    let d_s = [[ks, 0.0], [0.0, ks]];
    let gp = 1.0 / 3.0_f64.sqrt();
    let w = a * b / 4.0;
    let mut k = [[0.0; ELEMENT_DOF]; ELEMENT_DOF];
    for (xi, eta) in [(-gp, -gp), (gp, -gp), (gp, gp), (-gp, gp)] {
        add_btdb(&mut k, &membrane_b(a, b, xi, eta), &d_m, w);
        add_btdb(&mut k, &bending_b(a, b, xi, eta), &d_b, w);
        add_btdb(&mut k, &shear_b(a, b, xi, eta), &d_s, w);
        let dr = [drilling_row(a, b, xi, eta)];
        add_btdb(&mut k, &dr, &[[p.drilling_factor * g * t]], w);
    }
    k
}

/// 要素の質量 `m_e` を 4 節点に等分したときの、1 節点あたりの (並進の質量, 回転慣性)。回転慣性は Mindlin 板の m t²/12。
pub fn node_mass(m_e: f64, thickness: f64) -> (f64, f64) {
    let m = m_e / 4.0;
    (m, m * thickness * thickness / 12.0)
}
