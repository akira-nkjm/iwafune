//! 升に分けた板の温度の分布（`ThermalPlateTemperatures`）。頼まれたときだけ（`Analysis::run_detailed`）、板ごとの升のメッシュ
//! （機体の座標系）と、升ごとの温度を返す。**升の四隅の位置と、升の並び（番号）はここだけ**（格子は `plate_grid::PlateGrid`、
//! 板の位置・姿勢は解析の入力が持つ `ThermalPlate::frame`。姿勢をここで計算し直さない）。
//!
//! - 定常（`thermal.steady`）: ケースごとに 1 つの場（`quantity = steady`、ケースの ID とラベル付き）。
//! - 非定常（`thermal.transient`）: 升ごとの最低・最高・走行の終わりの温度の 3 つの場（`min`・`max`・`end`）。時刻ごとの全履歴は載せない
//!   （全履歴は出力の `series` にある）。
//! - メッシュは板ごとに 1 つの `SurfaceGroup`（`id` は部品の ID）で、四角形は升の通し番号の順。場の値の並びは、全部の板の
//!   升をこの順に並べたもの（`cells` と同じ並び）。
//!
//! 板が 1 つも無ければ詳細は無い（`None`）。解析（steady・transient）を知らない葉のモジュールで、解析が節点の温度を渡す。

use super::cell::ThermalCell;
use super::network::ThermalPlate;
use crate::mesh_view::{SurfaceGroup, SurfaceMesh};
use crate::plate_grid::PlateGrid;
use crate::spec::AnalysisError;
use sat_astro::math::Vec3;
use sat_units::typed::{Qty, Temperature};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

/// `Analysis::DETAIL_TYPE` の値。`ThermalPlateTemperatures` の TS の型名と一致する。
pub const DETAIL_TYPE: &str = "ThermalPlateTemperatures";

/// 場が表す量。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(rename_all = "snake_case")]
pub enum ThermalPlateQuantity {
    /// 定常解（`case` のケース）
    Steady,
    /// 走行の間の最低温度（非定常）
    Min,
    /// 走行の間の最高温度（非定常）
    Max,
    /// 走行の終わりの温度（非定常）
    End,
}

/// 場の並びの 1 つの升。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct ThermalPlateCell {
    /// 升の節点の ID（結果の `node` と同じ）
    pub node: String,
    pub cell: ThermalCell,
}

/// 升ごとの温度の 1 つの場（選んで見る 1 枚）。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct ThermalPlateField {
    pub quantity: ThermalPlateQuantity,
    /// 定常のときのケースの ID（非定常は無い）
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub case: Option<String>,
    /// 定常のときのケースのラベル
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub label: Option<String>,
    /// 升ごとの温度（`ThermalPlateTemperatures::cells` と同じ並び）
    pub temperature: Vec<Qty<Temperature>>,
}

/// 升に分けた板の温度の分布と、それを描くためのメッシュ（`thermal.steady` と `thermal.transient` の重い任意の出力）。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct ThermalPlateTemperatures {
    /// 板ごとの升のメッシュ（升の四隅。隣り合う升は頂点を共有する。四角形は升の通し番号の順）
    pub mesh: SurfaceMesh,
    /// 升の並び（板の順、板の中は升の通し番号の順。メッシュの四角形と同じ並び）
    pub cells: Vec<ThermalPlateCell>,
    /// 選んで見る場（定常はケースの順、非定常は最低・最高・終わり）
    pub fields: Vec<ThermalPlateField>,
}

/// 場 1 枚の元（節点の ID → 温度 [K]）。
pub struct PlateFieldSource<'a> {
    pub quantity: ThermalPlateQuantity,
    /// ケースの ID とラベル（定常）
    pub case: Option<(&'a str, &'a str)>,
    pub temperatures: BTreeMap<&'a str, f64>,
}

/// 板のメッシュと升の並び。
fn geometry(
    plates: &[ThermalPlate],
) -> Result<(SurfaceMesh, Vec<ThermalPlateCell>), AnalysisError> {
    let mut nodes = Vec::new();
    let mut groups = Vec::new();
    let mut cells = Vec::new();
    for p in plates {
        let size = [p.frame.size.x, p.frame.size.y, p.frame.size.z];
        let grid = PlateGrid::new(size, p.cells.map(|c| c as usize)).ok_or_else(|| {
            AnalysisError::InvalidInput(format!(
                "Plate {}: the part has no usable box size",
                p.part
            ))
        })?;
        let [nu, nv] = grid.cells;
        let first = nodes.len() as u32;
        // 格子の頂点（機体の座標系の位置 = 箱の位置 + 姿勢 × 頂点のずれ）
        for a in 0..=nu {
            for b in 0..=nv {
                let o = grid.vertex_offset(a, b);
                let at = p.frame.position + p.frame.orientation.rotate(Vec3::new(o[0], o[1], o[2]));
                nodes.push([at.x as f32, at.y as f32, at.z as f32]);
            }
        }
        let vertex = |a: usize, b: usize| first + (a * (nv + 1) + b) as u32;
        let mut quads = Vec::with_capacity(grid.count());
        for k in 0..grid.count() {
            let (i, j) = grid.position(k);
            quads.push([
                vertex(i, j),
                vertex(i + 1, j),
                vertex(i + 1, j + 1),
                vertex(i, j + 1),
            ]);
            let cell = ThermalCell {
                part: p.part.clone(),
                i: i as u32,
                j: j as u32,
            };
            cells.push(ThermalPlateCell {
                node: cell.node_id(),
                cell,
            });
        }
        groups.push(SurfaceGroup {
            id: p.part.clone(),
            quads,
        });
    }
    Ok((
        SurfaceMesh {
            nodes,
            groups,
            lines: Vec::new(),
        },
        cells,
    ))
}

/// 板の温度の分布。板が無ければ `None`。`sources` の節点の温度に、板の升の節点が無ければ入力の誤り。
pub fn plate_temperatures(
    plates: &[ThermalPlate],
    sources: Vec<PlateFieldSource<'_>>,
) -> Result<Option<ThermalPlateTemperatures>, AnalysisError> {
    if plates.is_empty() {
        return Ok(None);
    }
    let (mesh, cells) = geometry(plates)?;
    let fields = sources
        .into_iter()
        .map(|s| {
            let temperature = cells
                .iter()
                .map(|c| {
                    s.temperatures
                        .get(c.node.as_str())
                        .map(|t| Qty::from_raw(*t))
                        .ok_or_else(|| {
                            AnalysisError::InvalidInput(format!(
                                "No temperature for plate cell {}",
                                c.node
                            ))
                        })
                })
                .collect::<Result<Vec<_>, _>>()?;
            Ok(ThermalPlateField {
                quantity: s.quantity,
                case: s.case.map(|(id, _)| id.to_owned()),
                label: s.case.map(|(_, label)| label.to_owned()),
                temperature,
            })
        })
        .collect::<Result<Vec<_>, AnalysisError>>()?;
    Ok(Some(ThermalPlateTemperatures {
        mesh,
        cells,
        fields,
    }))
}

/// 詳細を JSON にする（`Analysis::run_detailed` の `detail`）。
pub fn to_detail(
    detail: Option<ThermalPlateTemperatures>,
) -> Result<Option<serde_json::Value>, AnalysisError> {
    detail
        .map(|d| serde_json::to_value(d).map_err(|e| AnalysisError::Json(e.to_string())))
        .transpose()
}

#[cfg(test)]
mod tests;
