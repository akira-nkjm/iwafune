//! 板（薄い箱）を面内の格子に分ける幾何。**板の法線の軸・面内の 2 軸の順・均等な格子の升の寸法と位置はここだけ**
//! （粗い有限要素 `structure::modal_fem` と、板の中の温度分布 `thermal::network::subdivide` が共有する）。
//!
//! 箱の最も薄い軸が法線（板厚の方向）。面内の軸 (u, v) は法線から巡回順（u = n+1, v = n+2）なので (u, v, n) は右手系:
//! 法線が x なら u = y・v = z、y なら u = z・v = x、z なら u = x・v = y。升の番号は (i, j) = (u 方向, v 方向)で、
//! 升の通し番号は `i × (v 方向の数) + j`。寸法はすべて箱の座標系（部品の座標系）。

/// 板とみなす最大の厚みの比（最も薄い辺 ÷ 次に薄い辺）。これより厚い箱は板として分けない。
pub const PLATE_MAX_THICKNESS_RATIO: f64 = 0.25;

/// 最も薄い軸（板の法線）。同じ長さなら添字の小さいほう。
pub fn thinnest_axis(extents: [f64; 3]) -> usize {
    (0..3)
        .min_by(|a, b| extents[*a].total_cmp(&extents[*b]))
        .unwrap_or(0)
}

/// 板の面内の軸 (u, v)（法線から巡回順）。
pub fn in_plane_axes(normal: usize) -> [usize; 2] {
    [(normal + 1) % 3, (normal + 2) % 3]
}

/// 板を面内の `cells = [u 方向, v 方向]` の升に均等に分けた格子。
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PlateGrid {
    /// 箱の辺の長さ [m]（箱の座標系の x, y, z）
    pub size: [f64; 3],
    /// 法線の軸（最も薄い軸）
    pub normal: usize,
    /// 面内の軸 (u, v)
    pub axes: [usize; 2],
    /// 升の数 (u, v)
    pub cells: [usize; 2],
}

impl PlateGrid {
    /// 箱 `size` を `cells` の升に分ける。辺は正の有限値、升の数は 1 以上でなければならない（そうでなければ `None`）。
    /// 板かどうか（[`PLATE_MAX_THICKNESS_RATIO`]）は [`Self::is_plate`] が見る（格子そのものは厚い箱にも作れる）。
    pub fn new(size: [f64; 3], cells: [usize; 2]) -> Option<Self> {
        let valid = size.iter().all(|s| s.is_finite() && *s > 0.0) && cells.iter().all(|n| *n >= 1);
        valid.then(|| {
            let normal = thinnest_axis(size);
            Self {
                size,
                normal,
                axes: in_plane_axes(normal),
                cells,
            }
        })
    }

    /// 板の厚み（最も薄い辺）[m]。
    pub fn thickness(&self) -> f64 {
        self.size[self.normal]
    }

    /// 板の形か（最も薄い辺が、次に薄い辺の [`PLATE_MAX_THICKNESS_RATIO`] 以下）。
    pub fn is_plate(&self) -> bool {
        let [u, v] = self.axes;
        self.thickness() <= PLATE_MAX_THICKNESS_RATIO * self.size[u].min(self.size[v])
    }

    /// 升の数の合計。
    pub fn count(&self) -> usize {
        self.cells[0] * self.cells[1]
    }

    /// 升 1 つの面内の辺の長さ (u, v) [m]。
    pub fn cell_size(&self) -> [f64; 2] {
        let [u, v] = self.axes;
        [
            self.size[u] / self.cells[0] as f64,
            self.size[v] / self.cells[1] as f64,
        ]
    }

    /// 升 1 つの箱の辺の長さ（箱の座標系の x, y, z。厚みは板のまま）。
    pub fn cell_box_size(&self) -> [f64; 3] {
        let mut size = self.size;
        let [du, dv] = self.cell_size();
        let [u, v] = self.axes;
        size[u] = du;
        size[v] = dv;
        size
    }

    /// 升 1 つの面積 [m²]（升は同じ大きさ）。
    pub fn cell_area(&self) -> f64 {
        let [du, dv] = self.cell_size();
        du * dv
    }

    /// 升 (i, j) の通し番号。
    pub fn index(&self, i: usize, j: usize) -> usize {
        i * self.cells[1] + j
    }

    /// 通し番号 → 升 (i, j)。
    pub fn position(&self, k: usize) -> (usize, usize) {
        (k / self.cells[1], k % self.cells[1])
    }

    /// 升 (i, j) の中心の、箱の中心からのずれ（箱の座標系の x, y, z。法線方向は 0）[m]。
    pub fn centre_offset(&self, i: usize, j: usize) -> [f64; 3] {
        let [du, dv] = self.cell_size();
        let [u, v] = self.axes;
        let mut at = [0.0; 3];
        at[u] = (i as f64 + 0.5) * du - 0.5 * self.size[u];
        at[v] = (j as f64 + 0.5) * dv - 0.5 * self.size[v];
        at
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn axes_run_cyclically_from_the_normal() {
        assert_eq!(in_plane_axes(0), [1, 2]);
        assert_eq!(in_plane_axes(1), [2, 0]);
        assert_eq!(in_plane_axes(2), [0, 1]);
        assert_eq!(thinnest_axis([0.3, 0.003, 0.2]), 1);
        // 同じ長さなら添字の小さいほう
        assert_eq!(thinnest_axis([0.1, 0.1, 0.5]), 0);
    }

    /// 手計算: 0.3 × 0.2 × 0.003 m の板（法線は z、u = x、v = y）を 3 × 2 に分けると、升は 0.1 × 0.1 m で面積 0.01 m²。
    /// 升 (0,0) の中心は (−0.1, −0.05)、升 (2,1) の中心は (0.1, 0.05)。通し番号は 0..6 で (2,1) が 5。
    #[test]
    fn cells_by_hand() {
        let g = PlateGrid::new([0.3, 0.2, 0.003], [3, 2]).unwrap();
        assert_eq!((g.normal, g.axes, g.count()), (2, [0, 1], 6));
        assert_eq!(g.cell_size(), [0.3 / 3.0, 0.1]);
        assert!((g.cell_area() - 0.01).abs() < 1e-15);
        assert_eq!(g.cell_box_size()[2], 0.003);
        let (a, b) = (g.centre_offset(0, 0), g.centre_offset(2, 1));
        assert!((a[0] + 0.1).abs() < 1e-15 && (a[1] + 0.05).abs() < 1e-15 && a[2] == 0.0);
        assert!((b[0] - 0.1).abs() < 1e-15 && (b[1] - 0.05).abs() < 1e-15);
        assert_eq!((g.index(2, 1), g.position(5)), (5, (2, 1)));
        assert!(g.is_plate());
        assert!(!PlateGrid::new([0.3, 0.2, 0.1], [2, 2]).unwrap().is_plate());
    }

    #[test]
    fn a_normal_along_y_uses_z_then_x() {
        let g = PlateGrid::new([0.2, 0.002, 0.4], [2, 4]).unwrap();
        assert_eq!((g.normal, g.axes), (1, [2, 0]));
        assert_eq!(g.cell_size(), [0.2, 0.05]); // u = z（0.4 / 2）、v = x（0.2 / 4）
    }

    #[test]
    fn invalid_sizes_and_counts_are_refused() {
        assert!(PlateGrid::new([0.3, 0.2, 0.0], [1, 1]).is_none());
        assert!(PlateGrid::new([0.3, 0.2, f64::NAN], [1, 1]).is_none());
        assert!(PlateGrid::new([0.3, 0.2, 0.003], [0, 1]).is_none());
    }
}
