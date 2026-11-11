//! 質量特性（`structure.mass_properties`）。総質量・重心・重心まわりの慣性テンソル（機体座標系）。
//!
//! 各部品は「重心の位置・箱の外形・姿勢」で表し、箱の慣性（一様な直方体）を姿勢で回して平行軸の定理で重心へ移す。
//! 外形が無ければ点質量、`inertia` を与えればそれを使う。主慣性モーメントと主軸は `Mat3::symmetric_eigen` で求める。
//! **質量特性はここだけ**: 姿勢の解析に渡す慣性も、projection が [`mass_properties`] を呼んで求める。

use crate::budget::{BudgetItem, limit_check};
use crate::spec::quantity_text;
use crate::spec::{Analysis, AnalysisError, AnalysisKind, Check, System, Verdict};
use sat_astro::math::{Mat3, Quat, Vec3};
use sat_units::typed::{Fraction, Length, Mass, Qty};
use serde::{Deserialize, Serialize};

/// 質量を持つ 1 部品。
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct MassPropertiesItem {
    pub id: String,
    /// 1 台あたりの質量
    pub mass: Qty<Mass>,
    /// 員数（同じ位置・外形・姿勢に置く台数。質量と慣性に掛ける。省略で 1、0 なら数えない）
    #[serde(default = "crate::spec::default_count")]
    pub count: u32,
    /// 部品の重心の位置（機体座標系）[m]
    pub position: Vec3,
    /// 箱の外形（部品座標系の辺の長さ）[m]。無ければ点質量
    #[serde(default)]
    pub size: Option<Vec3>,
    /// 部品座標系 → 機体座標系の回転（既定は回転なし）
    #[serde(default)]
    pub orientation: Option<Quat>,
    /// 部品の重心まわりの慣性テンソル（部品座標系）[kg·m²]。与えると箱の近似より優先する
    #[serde(default)]
    pub inertia: Option<Mat3>,
}

/// 重心の許容範囲（機体座標系の箱）。
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct MassPropertiesComBox {
    /// 箱の中心 [m]
    pub center: Vec3,
    /// 中心からの許容の半幅（軸ごと）[m]
    pub half_extent: Vec3,
}

/// 質量特性の入力。
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct MassPropertiesInput {
    pub items: Vec<MassPropertiesItem>,
    /// 質量の上限（与えると判定する）
    #[serde(default)]
    pub max_mass: Option<Qty<Mass>>,
    /// 重心の許容範囲（与えると判定する）
    #[serde(default)]
    pub com_box: Option<MassPropertiesComBox>,
}

/// 総質量・重心・慣性（[`mass_properties`] の結果）。
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct MassPropertiesSummary {
    pub total_mass: Qty<Mass>,
    /// 重心（機体座標系）[m]
    pub center_of_mass: Vec3,
    /// 重心まわりの慣性テンソル（機体座標系）[kg·m²]
    pub inertia: Mat3,
    /// 主慣性モーメント（昇順）[kg·m²]
    pub principal_moments: Vec3,
    /// 主軸（列ベクトルが `principal_moments` の順の単位ベクトル。機体座標系、右手系）
    pub principal_axes: Mat3,
}

/// 部品ごとの寄与。
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct MassPropertiesItemResult {
    pub id: String,
    /// 質量 × 員数
    pub mass: Qty<Mass>,
    /// 総質量に占める割合
    pub mass_fraction: Fraction,
    /// 系の重心まわりの慣性への寄与（機体座標系）[kg·m²]
    pub inertia_about_com: Mat3,
}

/// 質量特性の出力。
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct MassPropertiesOutput {
    pub summary: MassPropertiesSummary,
    pub items: Vec<MassPropertiesItemResult>,
    pub checks: Vec<Check>,
}

/// 質量特性。
pub struct MassProperties;

impl Analysis for MassProperties {
    const ID: &'static str = "structure.mass_properties";
    const TITLE: &'static str = "質量特性";
    const SYSTEM: System = System::Structure;
    const KIND: AnalysisKind = AnalysisKind::Steady;
    const DESCRIPTION: &'static str = "総質量・重心・慣性テンソル";
    const TITLE_EN: &'static str = "Mass properties";
    const DESCRIPTION_EN: &'static str = "Total mass, center of mass and inertia tensor";
    type Input = MassPropertiesInput;
    type Output = MassPropertiesOutput;

    fn run(input: &Self::Input) -> Result<Self::Output, AnalysisError> {
        let summary = mass_properties(&input.items)?;
        let m = summary.total_mass.value();
        let items = input
            .items
            .iter()
            .map(|it| MassPropertiesItemResult {
                id: it.id.clone(),
                mass: Qty::from_raw(it.total_mass()),
                mass_fraction: Fraction::new(it.total_mass() / m),
                inertia_about_com: item_inertia_about(it, summary.center_of_mass),
            })
            .collect();
        let checks = checks(input, &summary)?;
        Ok(MassPropertiesOutput {
            summary,
            items,
            checks,
        })
    }
}

/// 部品の一覧から総質量・重心・重心まわりの慣性を求める（projection も姿勢の解析の入力を作るのに使う）。
pub fn mass_properties(
    items: &[MassPropertiesItem],
) -> Result<MassPropertiesSummary, AnalysisError> {
    for it in items {
        let size_ok = it
            .size
            .is_none_or(|s| s.x >= 0.0 && s.y >= 0.0 && s.z >= 0.0);
        if !(it.total_mass() >= 0.0 && size_ok) {
            return Err(AnalysisError::InvalidInput(format!(
                "部品 {}: 質量と外形は 0 以上にしてください",
                it.id
            )));
        }
    }
    let total: f64 = items.iter().map(MassPropertiesItem::total_mass).sum();
    if total.is_nan() || total <= 0.0 {
        return Err(AnalysisError::InvalidInput("総質量が 0 です".to_owned()));
    }
    let com = items
        .iter()
        .fold(Vec3::ZERO, |acc, it| acc + it.position * it.total_mass())
        / total;
    let inertia = items
        .iter()
        .fold(Mat3::default(), |acc, it| acc + item_inertia_about(it, com));
    let (principal_moments, principal_axes) = inertia.symmetric_eigen();
    Ok(MassPropertiesSummary {
        total_mass: Qty::from_raw(total),
        center_of_mass: com,
        inertia,
        principal_moments,
        principal_axes,
    })
}

impl MassPropertiesItem {
    /// 質量 × 員数 [kg]。
    pub fn total_mass(&self) -> f64 {
        self.mass.value() * f64::from(self.count)
    }
}

/// 一様な直方体（辺 `size`）の重心まわりの慣性 [kg·m²]（箱の座標系）。
pub fn box_inertia(mass_kg: f64, size_m: Vec3) -> Mat3 {
    let (x2, y2, z2) = (size_m.x.powi(2), size_m.y.powi(2), size_m.z.powi(2));
    Mat3::diagonal(Vec3::new(y2 + z2, x2 + z2, x2 + y2)).scale(mass_kg / 12.0)
}

/// 部品の、点 `about`（機体座標系）まわりの慣性（機体座標系）。
fn item_inertia_about(it: &MassPropertiesItem, about: Vec3) -> Mat3 {
    let m = it.total_mass();
    let local = match (it.inertia, it.size) {
        (Some(i), _) => i.scale(f64::from(it.count)),
        (None, Some(s)) => box_inertia(m, s),
        (None, None) => Mat3::default(),
    };
    let r = it
        .orientation
        .map_or(Mat3::IDENTITY, |q| q.normalized().to_matrix());
    let own = r.mul(&local).mul(&r.transpose());
    own + parallel_axis(m, it.position - about)
}

/// 平行軸の定理の項 m (|d|² E − d dᵀ)。
fn parallel_axis(mass_kg: f64, d: Vec3) -> Mat3 {
    Mat3::IDENTITY.scale(mass_kg * d.norm_squared()) + Mat3::outer(d, d).scale(-mass_kg)
}

/// 総質量（`budget::limit_check` の判定。部品ごとに 1 行）と重心の判定。
fn checks(
    input: &MassPropertiesInput,
    s: &MassPropertiesSummary,
) -> Result<Vec<Check>, AnalysisError> {
    let mut out = Vec::new();
    if let Some(max) = input.max_mass {
        let rows: Vec<BudgetItem> = input
            .items
            .iter()
            .map(|it| BudgetItem::counted(it.id.clone(), it.mass.value(), it.count))
            .collect();
        out.extend(limit_check::<Mass>(&rows, max.value(), "mass", "総質量")?);
    }
    if let Some(b) = &input.com_box {
        let d = s.center_of_mass - b.center;
        let inside = d.x.abs() <= b.half_extent.x
            && d.y.abs() <= b.half_extent.y
            && d.z.abs() <= b.half_extent.z;
        out.push(Check::new(
            "center_of_mass",
            "重心の位置",
            Verdict::from_ok(inside),
            format!(
                "中心からのずれ ({}, {}, {})、許容 (±{}, ±{}, ±{})",
                quantity_text::<Length>(d.x),
                quantity_text::<Length>(d.y),
                quantity_text::<Length>(d.z),
                quantity_text::<Length>(b.half_extent.x),
                quantity_text::<Length>(b.half_extent.y),
                quantity_text::<Length>(b.half_extent.z)
            ),
        ));
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn item(id: &str, mass: f64, pos: Vec3, size: Option<Vec3>) -> MassPropertiesItem {
        MassPropertiesItem {
            id: id.to_owned(),
            mass: Qty::from_raw(mass),
            count: 1,
            position: pos,
            size,
            orientation: None,
            inertia: None,
        }
    }

    fn close(a: f64, b: f64) {
        assert!((a - b).abs() < 1e-12 * b.abs().max(1.0), "{a} vs {b}");
    }

    /// 6U（0.1×0.2×0.3 m、12 kg）の一様な箱: Ixx = m(b²+c²)/12 = 1·(0.04+0.09) = 0.13 kg·m²。
    #[test]
    fn single_box() {
        let s = mass_properties(&[item(
            "bus",
            12.0,
            Vec3::ZERO,
            Some(Vec3::new(0.1, 0.2, 0.3)),
        )])
        .unwrap();
        close(s.inertia.m[0][0], 0.13);
        close(s.inertia.m[1][1], 0.10);
        close(s.inertia.m[2][2], 0.05);
        close(s.inertia.m[0][1], 0.0);
    }

    /// 2 つの点質量 1 kg を x = ±0.1 m に置く: 重心 0、Iyy = Izz = 2·1·0.01 = 0.02、Ixx = 0。
    #[test]
    fn two_point_masses_parallel_axis() {
        let s = mass_properties(&[
            item("a", 1.0, Vec3::new(0.1, 0.0, 0.0), None),
            item("b", 1.0, Vec3::new(-0.1, 0.0, 0.0), None),
        ])
        .unwrap();
        close(s.center_of_mass.x, 0.0);
        close(s.inertia.m[0][0], 0.0);
        close(s.inertia.m[1][1], 0.02);
        close(s.inertia.m[2][2], 0.02);
    }

    /// 重心がずれた 2 点: 1 kg を (0.3, 0, 0)、3 kg を原点 → 重心 0.075、Izz = 1·0.225² + 3·0.075² = 0.0675。
    /// 積の慣性: 1 kg を (0.1, 0.1, 0) だけに置くと、その点まわりは 0、原点の 1 kg と組むと Ixy = −Σ m x y。
    #[test]
    fn offset_center_and_products() {
        let s = mass_properties(&[
            item("a", 1.0, Vec3::new(0.3, 0.0, 0.0), None),
            item("b", 3.0, Vec3::ZERO, None),
        ])
        .unwrap();
        close(s.center_of_mass.x, 0.075);
        close(s.inertia.m[2][2], 0.0675);
        let p = mass_properties(&[
            item("a", 1.0, Vec3::new(0.1, 0.1, 0.0), None),
            item("b", 1.0, Vec3::new(-0.1, -0.1, 0.0), None),
        ])
        .unwrap();
        close(p.inertia.m[0][1], -0.02);
        close(p.inertia.m[1][0], -0.02);
    }

    /// z 軸まわりに 90° 回した細長い箱は、x と y の慣性が入れ替わる。
    #[test]
    fn rotated_box_swaps_axes() {
        let mut it = item("rod", 2.0, Vec3::ZERO, Some(Vec3::new(1.0, 0.1, 0.1)));
        let plain = mass_properties(std::slice::from_ref(&it)).unwrap();
        it.orientation = Some(Quat::from_axis_angle(Vec3::Z, std::f64::consts::FRAC_PI_2));
        let rotated = mass_properties(&[it]).unwrap();
        assert!((rotated.inertia.m[0][0] - plain.inertia.m[1][1]).abs() < 1e-12);
        assert!((rotated.inertia.m[1][1] - plain.inertia.m[0][0]).abs() < 1e-12);
    }

    /// 細長い箱（1.0×0.1×0.1 m、2 kg）を z 軸まわりに 30° 回すと、慣性テンソルには積の項が出るが、
    /// 主慣性モーメントは回す前の対角成分（昇順）に一致し、最小の主軸は回した x 軸 (cos30°, sin30°, 0) になる。
    #[test]
    fn principal_axes_of_rotated_rod() {
        let mut it = item("rod", 2.0, Vec3::ZERO, Some(Vec3::new(1.0, 0.1, 0.1)));
        it.orientation = Some(Quat::from_axis_angle(Vec3::Z, 30.0_f64.to_radians()));
        let s = mass_properties(&[it]).unwrap();
        assert!(s.inertia.m[0][1].abs() > 1e-3);
        let small = 2.0 / 12.0 * 0.02;
        let large = 2.0 / 12.0 * 1.01;
        close(s.principal_moments.x, small);
        close(s.principal_moments.y, large);
        close(s.principal_moments.z, large);
        let axis = s.principal_axes.column(0);
        let expected = Vec3::new(
            30.0_f64.to_radians().cos(),
            30.0_f64.to_radians().sin(),
            0.0,
        );
        assert!((axis.dot(expected).abs() - 1.0).abs() < 1e-9, "{axis:?}");
    }

    /// 員数 2 は 2 倍の質量・慣性として数え、総質量の判定はバジェット（`budget::limit_check`）で行う。
    #[test]
    fn count_and_mass_budget() {
        let mut it = item(
            "rw",
            0.5,
            Vec3::new(0.1, 0.0, 0.0),
            Some(Vec3::new(0.05, 0.05, 0.05)),
        );
        let one = mass_properties(std::slice::from_ref(&it)).unwrap();
        it.count = 2;
        let input = MassPropertiesInput {
            items: vec![it],
            max_mass: Some(Qty::from_raw(0.9)),
            com_box: None,
        };
        let out = MassProperties::run(&input).unwrap();
        close(out.summary.total_mass.value(), 1.0);
        close(out.summary.inertia.m[0][0], 2.0 * one.inertia.m[0][0]);
        let c = out.checks.iter().find(|c| c.key == "mass").unwrap();
        assert_eq!(c.verdict, Verdict::Violation);
    }
}
