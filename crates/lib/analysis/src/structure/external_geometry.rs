//! 外形の面積の報告（`structure.external_geometry`）。寸法（`size`）を持つ部品の箱から、機体の外形を出す。
//! 投影面積（向きごと・全方向の平均。箱どうしが隠し合う分を引いた厳密な和集合）と、外に出ている表面積（ほかの箱に覆われない面）。
//! 高度低下・空力と太陽輻射圧のトルク・打ち上げの外寸・のちの影の計算が、同じ幾何を使う。**報告だけで、判定は付けない。**
//!
//! 規則（モデル化の既定はここに持つ。入力は箱だけ）:
//! - **箱 1 つ = 部品 1 つ**。員数（`count`）が 2 以上の部品は姿勢が 1 つなので、1 つぶんだけ数え、一覧にする。
//! - **投影面積**: 向きに沿った各箱の輪郭の和集合（`sat_astro::math::planar`）。内部の部品は和集合が隠すので、分類しない。
//!   面積は ±向きで等しい（同じ平面への正射影）ので、軸ごとに X・Y・Z の 3 つを報告する。図心も同じ点になる。
//! - **全方向の平均・最小・最大**: 向きの組（[`directions::DirectionSet::standard`]）の各向きと、機体の 3 軸で測る。
//!   平均は求積（誤差は `directions` に）。最小・最大は、測った向きの中の値で、真の極値の近似（最大は下限、最小は上限）。
//! - **外に出ている表面積**: 箱の面のうち、ほかの箱に覆われない部分（`exposed`）。**内部の部品**（8 つの頂点がほかの 1 つの箱の中
//!   にある部品）は、面も数えず、一覧にする。向きの内訳は面の法線の最大成分の軸で分ける。
//! - 寸法が無い（または正でない）部品は計算に入れず、一覧に残す（箱の選び方は干渉の確認と共通）。箱が 1 つも無ければ、入力の不足。
//!
//! - **面の日照割合**（`lit`）: 遠い光源（太陽）に対して、ほかの箱が落とす影を引いた面の照らされる割合。太陽電池の発電と熱の外面の直達日射が使う
//!   （規則は `lit` の説明）。この報告は日照割合を出さない。
//!
//! 箱は干渉の確認と同じ `OrientedBox`・`InterferenceBox`。

pub mod directions;
pub mod exposed;
pub mod geometry;
pub mod lit;

use super::interference::{
    InterferenceBox, InterferenceRepeated, InterferenceSkipped, count, usable_boxes,
};
use crate::spec::quantity_text;
use crate::spec::{Analysis, AnalysisArea, AnalysisError, AnalysisKind, Check, System, Verdict};
use directions::DirectionSet;
use exposed::Exposure;
pub use geometry::{DirectionArea, ExternalGeometry, PartBox, Projection, direction_key, mean_of};
pub use lit::{
    DEFAULT_SHADOWING, Footprint, LitSurfaces, PartLit, ShadowLoss, ShadowingInput, ShadowingReport,
};
use sat_astro::math::Vec3;
use sat_units::typed::{Area, Qty};
use serde::{Deserialize, Serialize};

/// 機体座標系の軸。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(rename_all = "snake_case")]
pub enum BodyAxis {
    X,
    Y,
    Z,
}

impl BodyAxis {
    const ALL: [Self; 3] = [Self::X, Self::Y, Self::Z];

    fn direction(self) -> Vec3 {
        match self {
            Self::X => Vec3::X,
            Self::Y => Vec3::Y,
            Self::Z => Vec3::Z,
        }
    }
}

/// 面の向き（外向き法線が最も近い機体座標系の軸）。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(rename_all = "snake_case")]
pub enum FaceDirection {
    NegX,
    PosX,
    NegY,
    PosY,
    NegZ,
    PosZ,
}

impl FaceDirection {
    /// `Exposure::by_direction` の添字の順。
    const ALL: [Self; 6] = [
        Self::NegX,
        Self::PosX,
        Self::NegY,
        Self::PosY,
        Self::NegZ,
        Self::PosZ,
    ];
}

/// 外形の面積の報告の入力。
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct ExternalGeometryInput {
    /// 寸法を持つ部品の箱（干渉の確認と同じ）
    pub boxes: Vec<InterferenceBox>,
    /// 寸法を持たないので計算に入れない部品の ID
    #[serde(default)]
    pub unsized_parts: Vec<String>,
}

/// 機体の軸に沿って見た投影。
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct AxisProjection {
    /// 見る向きの軸（+向きと −向きで同じ投影なので、向きの符号は区別しない）
    pub along: BodyAxis,
    /// 投影面積（+向きと −向きで同じ）
    pub area: Qty<Area>,
    /// 輪郭の和集合の図心（機体座標系。機体の原点を通り軸に垂直な平面の上の点なので、軸の成分は 0）
    pub centre: Vec3,
}

/// 測った向きの中での最小・最大の投影面積。
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct ProjectionExtreme {
    pub area: Qty<Area>,
    /// その向き（機体座標系の単位ベクトル。逆向きでも同じ面積）
    pub direction: Vec3,
}

/// 面の向きごとの、外に出ている表面積。
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct ExposedByDirection {
    pub face: FaceDirection,
    pub area: Qty<Area>,
}

/// 部品ごとの、外に出ている表面積。
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct ExposedByPart {
    pub id: String,
    pub area: Qty<Area>,
}

/// 内部と判定した部品（ほかの箱に丸ごと含まれる）。
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct InternalPart {
    pub id: String,
    /// 含む箱の部品（複数あれば ID の昇順で先のもの）
    pub inside: String,
}

/// 外形の面積の報告の出力。
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct ExternalGeometryOutput {
    /// 計算に入れた箱の数（員数が 2 以上の部品も 1 つ）
    pub parts_with_box: u32,
    /// 機体の軸に沿った投影（X・Y・Z）
    pub projected: Vec<AxisProjection>,
    /// 全方向の平均の投影面積
    pub mean_projected_area: Qty<Area>,
    /// 最小の投影面積（測った向きの中）
    pub min_projected: ProjectionExtreme,
    /// 最大の投影面積（測った向きの中）
    pub max_projected: ProjectionExtreme,
    /// 測った向きの数（求積の向き + 機体の 3 軸）
    pub directions_measured: u32,
    /// 外に出ている表面積の合計
    pub exposed_area: Qty<Area>,
    /// 面の向き別（−X, +X, −Y, +Y, −Z, +Z）
    pub exposed_by_direction: Vec<ExposedByDirection>,
    /// 部品別（内部の部品を除く。ID の昇順）
    pub exposed_by_part: Vec<ExposedByPart>,
    /// 内部と判定した部品（表面積に入れない）
    pub internal_parts: Vec<InternalPart>,
    /// 計算に入れなかった部品（寸法が無い）
    pub skipped: Vec<InterferenceSkipped>,
    /// 員数が 2 以上で、1 つの箱として数えた部品
    pub repeated: Vec<InterferenceRepeated>,
    /// 判定（報告だけなので、ok の 1 つ）
    pub checks: Vec<Check>,
}

/// 外形の面積の報告。
pub struct ExternalGeometryReport;

impl Analysis for ExternalGeometryReport {
    const ID: &'static str = "structure.external_geometry";
    const TITLE: &'static str = "外形の面積";
    const AREA: AnalysisArea = AnalysisArea::System(System::Structure);
    const KIND: AnalysisKind = AnalysisKind::Steady;
    const DESCRIPTION: &'static str = "部品の箱から出した投影面積（向きごと・全方向の平均）と外に出ている表面積（箱での近似。報告だけ）";
    const TITLE_EN: &'static str = "External geometry";
    const DESCRIPTION_EN: &'static str = "Projected areas (per axis and averaged over all directions) and exposed surface area from the boxes of the parts (a box approximation; report only)";
    type Input = ExternalGeometryInput;
    type Output = ExternalGeometryOutput;

    fn run(input: &Self::Input) -> Result<Self::Output, AnalysisError> {
        let (judged, mut skipped) = usable_boxes(&input.boxes, &input.unsized_parts);
        if judged.is_empty() {
            return Err(AnalysisError::InvalidInput(
                "No part has a size, so there is no external geometry".to_owned(),
            ));
        }
        skipped.sort_by(|a, b| a.id.cmp(&b.id));
        let repeated = judged
            .iter()
            .filter(|b| b.count > 1)
            .map(|b| InterferenceRepeated {
                id: b.id.clone(),
                count: b.count,
            })
            .collect();
        let geometry = ExternalGeometry::from_usable(&judged);
        Ok(report(&geometry, skipped, repeated))
    }
}

/// 幾何から出力を組み立てる。
fn report(
    geometry: &ExternalGeometry,
    skipped: Vec<InterferenceSkipped>,
    repeated: Vec<InterferenceRepeated>,
) -> ExternalGeometryOutput {
    let axes: Vec<DirectionArea> = BodyAxis::ALL
        .iter()
        .map(|a| DirectionArea {
            direction: a.direction(),
            weight: 0.0,
            area: geometry.projected(a.direction()).map_or(0.0, |p| p.area),
        })
        .collect();
    let sampled = geometry.projected_areas(&DirectionSet::standard());
    let mean = mean_of(&sampled);
    // 機体の軸を先に並べる。同じ値なら先のほうを残す（向きの組の順は固定）
    let all: Vec<&DirectionArea> = axes.iter().chain(sampled.iter()).collect();
    let (min_projected, max_projected) = (extreme(&all, |a, b| a < b), extreme(&all, |a, b| a > b));
    let exposure = geometry.exposure();
    let internal_parts: Vec<InternalPart> = (0..geometry.parts().len())
        .filter_map(|i| {
            Some(InternalPart {
                id: geometry.parts()[i].id.clone(),
                inside: geometry.parts()[geometry.container_of(i)?].id.clone(),
            })
        })
        .collect();
    let checks = vec![Check::new(
        "external_geometry",
        "External geometry",
        Verdict::Ok,
        format!(
            "Projected area {} to {} (mean {} over all directions), exposed surface {} (boxes: {}, judged internal: {}; boxes are an approximation of the real shapes)",
            quantity_text::<Area>(min_projected.area.value()),
            quantity_text::<Area>(max_projected.area.value()),
            quantity_text::<Area>(mean),
            quantity_text::<Area>(exposure.total),
            geometry.parts().len(),
            internal_parts.len(),
        ),
    )];
    ExternalGeometryOutput {
        parts_with_box: count(geometry.parts().len()),
        projected: axis_projections(geometry),
        mean_projected_area: Qty::from_raw(mean),
        min_projected,
        max_projected,
        directions_measured: count(all.len()),
        exposed_area: Qty::from_raw(exposure.total),
        exposed_by_direction: exposed_by_direction(&exposure),
        exposed_by_part: exposed_by_part(geometry, &exposure),
        internal_parts,
        skipped,
        repeated,
        checks,
    }
}

/// 測った向きのうち、`better` の意味で最も良い値（最初に見つけたもの）。
fn extreme(all: &[&DirectionArea], better: fn(f64, f64) -> bool) -> ProjectionExtreme {
    let mut best = all[0];
    for a in all {
        if better(a.area, best.area) {
            best = a;
        }
    }
    ProjectionExtreme {
        area: Qty::from_raw(best.area),
        direction: best.direction,
    }
}

/// 機体の軸に沿った投影（面積と図心）。
fn axis_projections(geometry: &ExternalGeometry) -> Vec<AxisProjection> {
    BodyAxis::ALL
        .iter()
        .map(|axis| {
            let p = geometry.projected(axis.direction());
            AxisProjection {
                along: *axis,
                area: Qty::from_raw(p.map_or(0.0, |p| p.area)),
                centre: p.and_then(|p| p.centre).unwrap_or(Vec3::ZERO),
            }
        })
        .collect()
}

/// 面の向き別の表面積（−X, +X, −Y, +Y, −Z, +Z）。
fn exposed_by_direction(exposure: &Exposure) -> Vec<ExposedByDirection> {
    FaceDirection::ALL
        .iter()
        .zip(exposure.by_direction)
        .map(|(face, area)| ExposedByDirection {
            face: *face,
            area: Qty::from_raw(area),
        })
        .collect()
}

/// 部品別の表面積（内部の部品を除く）。
fn exposed_by_part(geometry: &ExternalGeometry, exposure: &Exposure) -> Vec<ExposedByPart> {
    exposure
        .by_part
        .iter()
        .map(|(i, area)| ExposedByPart {
            id: geometry.parts()[*i].id.clone(),
            area: Qty::from_raw(*area),
        })
        .collect()
}

#[cfg(test)]
mod tests;
