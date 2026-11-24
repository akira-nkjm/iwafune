//! フレームの部材（梁要素）。2 節点の Euler–Bernoulli 梁（節点あたり 6 自由度）と、正方形の断面の性質。
//!
//! 局所の x が部材の軸、y・z が断面の軸（全体の軸の巡回の並べ替え。x = a, y = a+1, z = a+2 なので右手系）。
//! 断面は正方形（中実か、肉厚一定の角パイプ）なので、y・z の曲げは同じ剛性で、局所の y・z の向きは結果に効かない。
//! 曲げはせん断変形を無視する（部材の長さに対して断面が小さい、レールと枠の部材を想定する）。
//! 質量は節点の集中質量（回転慣性なし）で、[`super::masses`] が部材に沿って配る。

use super::input::ModalFemMemberSection;

/// 要素の自由度の数（2 節点 × 6）。
pub const BEAM_DOF: usize = 12;
/// 中実の正方形のねじり定数 J = 0.1406 a⁴ の係数（正方形の断面のねじり: J = 0.1406 a⁴。文献の値。丸めた近似）。
const SQUARE_TORSION: f64 = 0.1406;

/// 梁要素の剛性行列（局所の自由度は節点ごとに u, v, w, θx, θy, θz）。
pub type BeamMatrix = [[f64; BEAM_DOF]; BEAM_DOF];

/// 断面の性質。
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SectionProps {
    /// 断面積 A [m²]
    pub area: f64,
    /// 断面二次モーメント I（y・z 共通）[m⁴]
    pub second_moment: f64,
    /// ねじり定数 J [m⁴]
    pub torsion: f64,
}

/// 梁要素の材料と断面。
#[derive(Debug, Clone, Copy)]
pub struct BeamProps {
    pub youngs_modulus: f64,
    pub poisson_ratio: f64,
    pub section: SectionProps,
}

impl ModalFemMemberSection {
    /// 断面の性質。**正方形の断面から A・I・J を出すのはここだけ**。
    /// 外側の辺 a、肉厚 t（無ければ中実）として、
    /// 中実: A = a²、I = a⁴/12、J = 0.1406 a⁴。
    /// 角パイプ: A = a² − (a − 2t)²、I = (a⁴ − (a − 2t)⁴)/12、J = t (a − t)³（薄肉の閉断面のねじり 4 Aₘ² t / s、Aₘ = (a − t)²、s = 4 (a − t)）。
    /// 辺が正でない、肉厚が正でない、または肉厚が辺の半分以上なら `None`。
    pub fn properties(&self) -> Option<SectionProps> {
        let a = self.width.value();
        if !(a.is_finite() && a > 0.0) {
            return None;
        }
        let Some(wall) = self.wall else {
            return Some(SectionProps {
                area: a * a,
                second_moment: a.powi(4) / 12.0,
                torsion: SQUARE_TORSION * a.powi(4),
            });
        };
        let t = wall.value();
        if !(t.is_finite() && t > 0.0 && 2.0 * t < a) {
            return None;
        }
        let inner = a - 2.0 * t;
        Some(SectionProps {
            area: a * a - inner * inner,
            second_moment: (a.powi(4) - inner.powi(4)) / 12.0,
            torsion: t * (a - t).powi(3),
        })
    }
}

/// 長さ `len` の梁要素の剛性行列。曲げは三次のエルミート補間（Euler–Bernoulli）、軸とねじりは線形。
/// y 方向の曲げ（v, θz）と z 方向の曲げ（w, θy）は、回転の符号（θy = −∂w/∂x）だけが違う。
pub fn stiffness(len: f64, p: &BeamProps) -> BeamMatrix {
    let (e, a, i, j) = (
        p.youngs_modulus,
        p.section.area,
        p.section.second_moment,
        p.section.torsion,
    );
    let g = e / (2.0 * (1.0 + p.poisson_ratio));
    let mut k = [[0.0; BEAM_DOF]; BEAM_DOF];
    let mut set = |r: usize, c: usize, x: f64| {
        k[r][c] = x;
        k[c][r] = x;
    };
    // 軸（u: 0, 6）とねじり（θx: 3, 9）
    let (ea, gj) = (e * a / len, g * j / len);
    set(0, 0, ea);
    set(6, 6, ea);
    set(0, 6, -ea);
    set(3, 3, gj);
    set(9, 9, gj);
    set(3, 9, -gj);
    // 曲げ: (変位, 回転) の自由度の組と回転の符号 s（xy 面は +1、xz 面は −1）
    let c = e * i / len.powi(3);
    for (v1, r1, v2, r2, s) in [(1, 5, 7, 11, 1.0), (2, 4, 8, 10, -1.0)] {
        set(v1, v1, 12.0 * c);
        set(v2, v2, 12.0 * c);
        set(v1, v2, -12.0 * c);
        set(v1, r1, s * 6.0 * len * c);
        set(v1, r2, s * 6.0 * len * c);
        set(v2, r1, -s * 6.0 * len * c);
        set(v2, r2, -s * 6.0 * len * c);
        set(r1, r1, 4.0 * len * len * c);
        set(r2, r2, 4.0 * len * len * c);
        set(r1, r2, 2.0 * len * len * c);
    }
    k
}
