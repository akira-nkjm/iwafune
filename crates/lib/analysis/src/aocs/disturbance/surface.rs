//! 力を受ける面（空力・太陽輻射圧）の、係数・受圧面積・圧力中心。
//!
//! 面積と圧力中心は、**書かれていればその値**（向きによらない一定の値）、**無ければ部品の箱から向きごとに出す**:
//! 面積は流れ（太陽）の向きに沿って見た箱の和集合の正射影の面積、圧力中心は和集合の輪郭の図心
//! （`ExternalGeometry::projected`。高度低下の平均の面積・報告 `structure.external_geometry` と同じ関数。幾何はここに持たない）。
//! 圧力中心 − 重心（腕）は、図心と重心の差のうち向きに垂直な成分（向きに沿う成分は力と平行でトルクに効かない）。
//! 物理のモデル（係数 × 面積、力は向きに沿う）は [`super`] にあり、ここは面積と腕を決めるだけ。

use super::DEFAULT_CP_OFFSET;
use crate::structure::external_geometry::{ExternalGeometry, Projection};
use crate::structure::interference::InterferenceBox;
use sat_astro::math::Vec3;
use sat_units::typed::{Area, Qty};
use serde::{Deserialize, Serialize};
use std::cell::RefCell;
use std::collections::HashMap;

/// 力を受ける面（キャノンボール近似: 向きによらない一定の係数。面積と圧力中心は書くか、箱から出す）。
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct DisturbanceSurface {
    /// 係数（空力は Cd、太陽輻射圧は Cr）
    pub coefficient: f64,
    /// 受圧面積。書かれていればこの値（向きによらない）。省略時は部品の箱から向きごとに出す（箱が要る）
    #[serde(default)]
    pub area: Option<Qty<Area>>,
    /// 圧力中心 − 重心 [m]（機体座標系）。書かれていればこの値。省略時は箱から出した圧力中心と重心の差
    /// （箱が無ければ [`DEFAULT_CP_OFFSET`] = 0、重心に一致しトルクが出ない）
    #[serde(default)]
    pub cp_offset: Option<Vec3>,
}

/// 面積と圧力中心を出すための、部品の箱と重心。
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct DisturbanceGeometry {
    /// 寸法を持つ部品の箱（機体座標系。`structure.external_geometry` と同じ入力）
    pub boxes: Vec<InterferenceBox>,
    /// 重心 [m]（機体座標系。腕の基準）
    pub center_of_mass: Vec3,
}

/// 面積・圧力中心の出どころ。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(rename_all = "snake_case")]
pub enum SurfaceValueSource {
    /// 設定に書かれた値
    Written,
    /// 部品の箱の和集合の投影から、向きごとに出した値
    Geometry,
    /// 圧力中心だけ: 書かれておらず箱も無いので、重心に置いた（トルクは 0）
    CentreOfMass,
}

/// ある向きでの面の評価。
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SurfaceSample {
    /// 受圧面積 [m²]
    pub area: f64,
    /// 圧力中心 − 重心 [m]（機体座標系）
    pub cp_offset: Vec3,
    pub area_source: SurfaceValueSource,
    pub cp_source: SurfaceValueSource,
}

/// 向きの量子化の細かさ（成分を 1e-12 の刻みに丸めて、同じ向きの繰り返しを使い回す。差は浮動小数点の雑音の大きさ）。
const DIRECTION_KEY_SCALE: f64 = 1e12;

/// 面の評価器。箱の幾何（内部の判定込み）を 1 度だけ作り、向きごとの投影を使い回す。
pub(super) struct SurfaceModel {
    geometry: Option<(ExternalGeometry, Vec3)>,
    /// 向き（量子化）→ 投影。固定姿勢の機体の一定の向き（太陽指向の太陽の向きなど）を何度も解かない
    cache: RefCell<HashMap<[i64; 3], Option<Projection>>>,
}

impl SurfaceModel {
    pub(super) fn new(geometry: Option<&DisturbanceGeometry>) -> Self {
        Self {
            geometry: geometry.and_then(|g| {
                ExternalGeometry::from_boxes(&g.boxes).map(|e| (e, g.center_of_mass))
            }),
            cache: RefCell::new(HashMap::new()),
        }
    }

    /// 使える箱があるか。
    pub(super) fn has_geometry(&self) -> bool {
        self.geometry.is_some()
    }

    /// 向き `direction`（機体座標系。流れ・太陽の向き）での投影。
    fn project(&self, direction: Vec3) -> Option<Projection> {
        let (geometry, _) = self.geometry.as_ref()?;
        let d = direction.normalized().filter(|d| d.x.is_finite())?;
        let key = [d.x, d.y, d.z].map(|c| (c * DIRECTION_KEY_SCALE).round() as i64);
        if let Some(hit) = self.cache.borrow().get(&key) {
            return *hit;
        }
        let projection = geometry.projected(d);
        self.cache.borrow_mut().insert(key, projection);
        projection
    }

    /// 向き `direction` に沿って力を受けるときの面積と腕。書かれた値が優先で、無いほうだけ箱から出す。
    pub(super) fn evaluate(&self, surface: &DisturbanceSurface, direction: Vec3) -> SurfaceSample {
        let projection = (surface.area.is_none() || surface.cp_offset.is_none())
            .then(|| self.project(direction))
            .flatten();
        let (area, area_source) = match surface.area {
            Some(a) => (a.value(), SurfaceValueSource::Written),
            None => (
                projection.map_or(0.0, |p| p.area),
                SurfaceValueSource::Geometry,
            ),
        };
        let centre = projection.and_then(|p| p.centre);
        let (cp_offset, cp_source) = match (surface.cp_offset, centre, &self.geometry) {
            (Some(v), _, _) => (v, SurfaceValueSource::Written),
            (None, Some(c), Some((_, com))) => (
                perpendicular(c - *com, direction),
                SurfaceValueSource::Geometry,
            ),
            _ => (DEFAULT_CP_OFFSET, SurfaceValueSource::CentreOfMass),
        };
        SurfaceSample {
            area,
            cp_offset,
            area_source,
            cp_source,
        }
    }
}

/// `v` の、向き `direction` に垂直な成分（向きに沿う成分は力と平行なのでトルクに効かない）。
fn perpendicular(v: Vec3, direction: Vec3) -> Vec3 {
    match direction.normalized() {
        Some(d) => v - d * v.dot(d),
        None => v,
    }
}

#[cfg(test)]
mod tests;
