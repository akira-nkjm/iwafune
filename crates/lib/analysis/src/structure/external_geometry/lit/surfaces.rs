//! 面の一覧（部品の ID と外向き法線）の、向きごとの日照割合と、その報告。規則と前処理は親の `lit`（[`FaceOccluders`]）。

use super::super::cells::FaceCell;
use super::super::geometry::{ExternalGeometry, direction_key};
use super::{FaceOccluders, Footprint, ShadowingInput};
use sat_astro::math::Vec3;
use sat_units::typed::{Area, Fraction, Power, Qty};
use serde::{Deserialize, Serialize};
use std::cell::RefCell;
use std::collections::{BTreeMap, HashMap};
use std::rc::Rc;

/// 面 1 枚の、測った向きの中での日照割合の最小と平均。
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct PartLit {
    /// 面の部品の ID
    pub id: String,
    /// 光を受けた向きの中での日照割合の最小
    pub min: Fraction,
    /// 光を受けた向きの中での日照割合の平均（サンプルの単純平均）
    pub mean: Fraction,
    /// 面が光を受けた（法線と光源の向きの cos が正の）サンプルの数。0 なら `min`・`mean` は 1
    pub samples: u32,
    /// ほかの部品に覆われた面積（箱の面の面積のうち、覆いの範囲の和集合。熱の外面だけが数える。数えなければ省略）。
    /// 覆われた面積は環境とやり取りしない（直達日射・アルベド・惑星の赤外・宇宙への放射のどれも）
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub covered_area: Option<Qty<Area>>,
    /// `covered_area` / 箱の面の面積
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub covered: Option<Fraction>,
    /// 面を覆っている部品の ID（ID の昇順。覆われていなければ空。熱の外面だけが数える）
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub covered_by: Option<Vec<String>>,
    /// 面の前に平行に向き合う（接していない）箱への形態係数の和（1 で打ち切る。熱の外面だけが数える）。この割合の分、面は環境に開いていない
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub facing: Option<Fraction>,
    /// 向き合う箱の部品の ID（ID の昇順）
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub facing_by: Option<Vec<String>>,
    /// 面が環境（アルベド・惑星の赤外・宇宙）に開いている割合 `(1 − covered)(1 − facing)`
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub exposed: Option<Fraction>,
}

/// 影を数えた結果の報告（影を数えたときだけ出力に付く）。
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct ShadowingReport {
    /// 影を数えた面の日照割合（入力の面の順）
    pub parts: Vec<PartLit>,
    /// 影を数えなかった面の部品の ID（箱が無い、書かれた法線に合う箱の面が無い）
    pub unshadowed: Vec<String>,
    /// 光を受けたどれかの向きで、報告した面のどれかに影を落とした部品（ID の昇順）
    #[serde(default)]
    pub casters: Vec<String>,
    /// 報告した面のどれかを覆っている・平行に向き合っている部品（ID の昇順。熱の外面だけが数える）
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub coverers: Option<Vec<String>>,
    /// 影で遮られた直達日射のうち、影を落とす部品が外面を持たず、どこにも吸収されない分の報告（熱の解析だけ）
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub lost: Option<ShadowLoss>,
}

/// 影を落とす・面を覆う部品が熱の外面を持たないために、吸収されずに消える入熱の報告（熱の解析）。
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct ShadowLoss {
    /// 面に影を落とす・面を覆うが、自分の熱の外面を持たない部品（その部品が遮った入熱はどの節点にも吸収されない）
    pub parts: Vec<String>,
    /// 外面を持たない部品が遮って吸収されない入熱: 外面を持つ部品の影と覆いだけを数えた入熱の合計 − すべての影と覆いを数えた入熱の合計
    /// （外面の全部の合計。外面を持つ部品が遮った分は、その部品が吸収するので数えない）
    pub power: Qty<Power>,
    /// `power` / すべての影と覆いを数えた入熱の合計
    pub fraction: Fraction,
}

/// 箱ごとの、部品 `owner` の面の光学特性に**含まれる**か（取付の連鎖に `owner` があり、かつ自分の熱の外面を持たない）。
/// `geometry` の箱の並びと同じ順。取り付けられた部品が自分の外面（面積・法線・光学特性）を持つなら、面の下は二重に数えないよう
/// その部品に覆われたものとして扱う（含めない。覆い・向き合い・影の規則をそのまま使う）。
/// 外面を持つかは `has_surface`（面の一覧に入っているか。`ShadowLoss` の `surfaced` と同じ判定）で決める。
fn lumped_into(
    input: &ShadowingInput,
    geometry: &ExternalGeometry,
    owner: &str,
    has_surface: &dyn Fn(&str) -> bool,
) -> Vec<bool> {
    geometry
        .parts()
        .iter()
        .map(|p| {
            !has_surface(&p.id)
                && input
                    .boxes
                    .iter()
                    .find(|b| b.id == p.id)
                    .is_some_and(|b| b.mount_ancestors.iter().any(|a| a == owner))
        })
        .collect()
}

/// 面の一覧の、向きごとの日照割合。面ごとの前処理と、向きごとの結果の使い回しを持つ。
/// 固定姿勢の機体では光源の向きが一定なので、向きごとの結果は 1 度だけ解く（[`direction_key`]。浮動小数点の雑音の外で結果は変わらない）。
/// 向きごと（外面の添字と向きの key）の、升ごとの日照割合の使い回し。
type CellCache = HashMap<(usize, [i64; 3]), Rc<Vec<f64>>>;

/// 直前に使った升ごとの日照割合（外面の添字・向きの key・外面を持つ部品だけか・升ごとの値）。
type LastCells = (usize, [i64; 3], bool, Rc<Vec<f64>>);

#[derive(Debug, Clone)]
pub struct LitSurfaces {
    ids: Vec<String>,
    /// 箱の部品の ID（`faces` の箱の番号と同じ順）
    part_ids: Vec<String>,
    /// 面ごとの前処理。箱の面に合わなければ `None`
    faces: Vec<Option<FaceOccluders>>,
    /// 面ごとの、覆われた割合（向きによらない。覆いを調べないとき・箱の面に合わないときは 0）
    covered: Vec<f64>,
    /// 面ごとの、環境に開いている割合（覆いと向き合う箱を数えたもの。数えないときは 1）
    exposed: Vec<f64>,
    /// 覆いを調べたか（熱の外面）。報告に覆われた面積を載せるかを決める
    reports_covers: bool,
    /// 箱ごと（`part_ids` の順）の、熱の外面を持つか（面の一覧に入れた部品か）
    surfaced: Vec<bool>,
    cache: RefCell<HashMap<(usize, [i64; 3]), f64>>,
    /// 外面を持つ部品の影と覆いだけを数えた日照割合の使い回し（[`LitSurfaces::lit_surfaced_only`]）
    cache_surfaced: RefCell<HashMap<(usize, [i64; 3]), f64>>,
    /// 面ごとの升（板を升に分けた部品の、板の面。升に分けない面は `None`）。面の座標の長方形で、升の通し番号の順
    cells: Vec<Option<Vec<FaceCell>>>,
    /// 升ごとの、環境に開いている割合（`cells` と同じ並び。覆いを調べないときは 1）
    exposed_cells: Vec<Vec<f64>>,
    /// 升ごとの、環境に開いている割合を、熱の外面を持つ部品の覆いと向き合いだけで数えたもの（`cells` と同じ並び）
    exposed_surfaced_cells: Vec<Vec<f64>>,
    /// 升ごとの日照割合の使い回し（向きごと。[`LitSurfaces::lit_cells`]）
    cache_cells: RefCell<CellCache>,
    /// 外面を持つ部品の影と覆いだけを数えた、升ごとの日照割合の使い回し
    cache_cells_surfaced: RefCell<CellCache>,
    /// 直前に使った升ごとの日照割合（`(外面の添字, 向きの key, 外面を持つ部品だけか, 値)`）。同じ面の升は続けて呼ばれるので、表を引かずに済む
    last_cells: RefCell<Option<LastCells>>,
}

impl LitSurfaces {
    /// 箱と面の一覧から作る。影を切ってある（`enabled` が偽）か、使える箱が 1 つも無ければ `None`（影を数えない）。
    /// 面の部品は ID で箱に結びつける。`footprint` が [`Footprint::Credited`]（熱の外面）のときは、面を覆う部品も調べる
    /// （規則は `covers`。面の部品に取り付けられ、自分の外面を持たない部品は覆いに数えない。外面を持つ部品は数える。#472）。
    pub fn new<'a>(
        input: &ShadowingInput,
        footprint: Footprint,
        surfaces: impl IntoIterator<Item = (&'a str, Vec3)>,
    ) -> Option<Self> {
        Self::new_subdivided(input, footprint, surfaces, &BTreeMap::new())
    }

    /// [`LitSurfaces::new`] に、升に分けた板（部品の ID → 升の数 (u, v)）を加える。板の面（箱の最も薄い軸に垂直な面）は、
    /// 日照割合・覆われた割合・環境に開いている割合を升ごとに求める（[`LitSurfaces::cell_count`]）。板の縁の面と、板でない部品の面は面全体で 1 つ。
    pub fn new_subdivided<'a>(
        input: &ShadowingInput,
        footprint: Footprint,
        surfaces: impl IntoIterator<Item = (&'a str, Vec3)>,
        plates: &BTreeMap<String, [usize; 2]>,
    ) -> Option<Self> {
        if !input.is_enabled() {
            return None;
        }
        let geometry = ExternalGeometry::from_boxes(&input.boxes)?;
        let surfaces: Vec<(&str, Vec3)> = surfaces.into_iter().collect();
        let (ids, faces): (Vec<String>, Vec<_>) = surfaces
            .iter()
            .map(|&(id, normal)| {
                let face = geometry
                    .parts()
                    .iter()
                    .position(|p| p.id == id)
                    .and_then(|owner| {
                        let face = geometry.face_toward(owner, normal)?;
                        let mounted = lumped_into(input, &geometry, id, &|part| {
                            surfaces.iter().any(|(s, _)| *s == part)
                        });
                        if footprint == Footprint::Blocks {
                            return Some(FaceOccluders::new(&geometry, owner, &face, None));
                        }
                        // 面の部品に取り付けられ、自分の外面を持たない部品は、面の光学特性に含まれる（覆いに数えない。取付の連鎖で決め、接している距離に頼らない）。
                        // 自分の外面を持つ部品は含まれない: 面の下はその部品に覆われたものとして、日射と環境とのやり取りを二重に数えない（#472）
                        Some(
                            FaceOccluders::new(&geometry, owner, &face, Some(&mounted))
                                .with_covers(&geometry, owner, &face, &mounted),
                        )
                    });
                (id.to_owned(), face)
            })
            .unzip();
        let part_ids: Vec<String> = geometry.parts().iter().map(|p| p.id.clone()).collect();
        let surfaced: Vec<bool> = part_ids
            .iter()
            .map(|id| surfaces.iter().any(|(s, _)| s == id))
            .collect();
        let exposed = faces
            .iter()
            .map(|f| {
                f.as_ref()
                    .map_or(1.0, |f| f.exposed_fraction_among(&|_| true))
            })
            .collect();
        let covered = faces
            .iter()
            .map(|f| f.as_ref().map_or(0.0, FaceOccluders::covered_fraction))
            .collect();
        let cells = split_cells(&geometry, &ids, &faces, plates);
        let exposed_cells = exposed_per_cell(&cells, &faces, &|_| true);
        let exposed_surfaced_cells = exposed_per_cell(&cells, &faces, &|j| {
            surfaced.get(j).copied().unwrap_or(false)
        });
        Some(Self {
            ids,
            part_ids,
            faces,
            covered,
            exposed,
            reports_covers: footprint == Footprint::Credited,
            surfaced,
            cache: RefCell::new(HashMap::new()),
            cache_surfaced: RefCell::new(HashMap::new()),
            cells,
            exposed_cells,
            exposed_surfaced_cells,
            cache_cells: RefCell::new(HashMap::new()),
            cache_cells_surfaced: RefCell::new(HashMap::new()),
            last_cells: RefCell::new(None),
        })
    }

    /// `id` の面を持つか（面の一覧に入れた部品か）。
    pub fn has_surface(&self, id: &str) -> bool {
        self.ids.iter().any(|x| x == id)
    }

    /// 面 `i` の覆われた割合（0〜1。向きによらない。箱の面に合わない面と、覆いを調べないときは 0）。
    pub fn covered(&self, i: usize) -> f64 {
        self.covered.get(i).copied().unwrap_or(0.0)
    }

    /// 面 `i` の、光源の向き `direction`（機体座標系）での日照割合。箱の面に合わない面は影を数えないので 1。
    pub fn lit(&self, i: usize, direction: Vec3) -> f64 {
        let Some(face) = self.faces.get(i).and_then(|f| f.as_ref()) else {
            return 1.0;
        };
        let Some(d) = direction.normalized().filter(|d| d.x.is_finite()) else {
            return 0.0;
        };
        let key = (i, direction_key(d));
        if let Some(hit) = self.cache.borrow().get(&key) {
            return *hit;
        }
        let lit = face.lit_fraction(d);
        self.cache.borrow_mut().insert(key, lit);
        lit
    }

    /// 面 `i` が環境に開いている割合（0〜1。接して覆う部品と、平行に向き合う部品を引く）。箱の面に合わない面と、覆いを調べないときは 1。
    pub fn exposed(&self, i: usize) -> f64 {
        self.exposed.get(i).copied().unwrap_or(1.0)
    }

    /// 面 `i` が環境に開いている割合を、熱の外面を持つ部品の覆いと向き合いだけで数えたもの（外面を持たない部品は無いものとする）。
    pub fn exposed_surfaced_only(&self, i: usize) -> f64 {
        self.faces.get(i).and_then(|f| f.as_ref()).map_or(1.0, |f| {
            f.exposed_fraction_among(&|j| self.surfaced.get(j).copied().unwrap_or(false))
        })
    }

    /// 面 `i` の日照割合を、熱の外面を持つ部品の影と覆いだけで数えたもの（外面を持たない部品は無いものとする）。
    /// 外面を持たない部品が遮って吸収されない入熱は、これと [`LitSurfaces::lit`] の差で求める
    /// （外面を持つ部品が遮った分は、その部品が吸収するので失われない）。
    pub fn lit_surfaced_only(&self, i: usize, direction: Vec3) -> f64 {
        let Some(face) = self.faces.get(i).and_then(|f| f.as_ref()) else {
            return 1.0;
        };
        let Some(d) = direction.normalized().filter(|d| d.x.is_finite()) else {
            return 0.0;
        };
        let key = (i, direction_key(d));
        if let Some(hit) = self.cache_surfaced.borrow().get(&key) {
            return *hit;
        }
        let lit = face.lit_fraction_among(d, &|j| self.surfaced.get(j).copied().unwrap_or(false));
        self.cache_surfaced.borrow_mut().insert(key, lit);
        lit
    }

    /// 光を受けた向き（サンプルの光源の向き。日陰のサンプルは呼び出し側が除く）ごとの日照割合の最小と平均。
    pub fn summary(&self, directions: &[Vec3]) -> ShadowingReport {
        let mut parts = Vec::new();
        let mut unshadowed = Vec::new();
        let mut casters = std::collections::BTreeSet::new();
        let mut coverers = std::collections::BTreeSet::new();
        for (i, face) in self.faces.iter().enumerate() {
            let Some(face) = face else {
                unshadowed.push(self.ids[i].clone());
                continue;
            };
            let lit: Vec<f64> = directions
                .iter()
                .filter(|d| face.receives_light(**d))
                .map(|d| self.lit(i, *d))
                .collect();
            for d in directions {
                casters.extend(
                    face.casters(*d)
                        .into_iter()
                        .map(|j| self.part_ids[j].as_str()),
                );
            }
            let n = lit.len();
            let covered_by: std::collections::BTreeSet<&str> = face
                .coverers()
                .into_iter()
                .map(|j| self.part_ids[j].as_str())
                .collect();
            coverers.extend(covered_by.iter().map(|id| (*id).to_owned()));
            let facing_by: std::collections::BTreeSet<&str> = face
                .facers()
                .into_iter()
                .map(|j| self.part_ids[j].as_str())
                .collect();
            coverers.extend(facing_by.iter().map(|id| (*id).to_owned()));
            parts.push(PartLit {
                id: self.ids[i].clone(),
                min: Fraction::new(lit.iter().copied().fold(1.0, f64::min)),
                mean: Fraction::new(if n == 0 {
                    1.0
                } else {
                    lit.iter().sum::<f64>() / n as f64
                }),
                samples: u32::try_from(n).unwrap_or(u32::MAX),
                covered_area: self
                    .reports_covers
                    .then(|| Qty::from_raw(face.area() * self.covered[i])),
                covered: self.reports_covers.then(|| Fraction::new(self.covered[i])),
                facing: self
                    .reports_covers
                    .then(|| Fraction::new(face.facing_fraction_among(&|_| true))),
                facing_by: self
                    .reports_covers
                    .then(|| facing_by.into_iter().map(str::to_owned).collect()),
                exposed: self.reports_covers.then(|| Fraction::new(self.exposed[i])),
                covered_by: self
                    .reports_covers
                    .then(|| covered_by.into_iter().map(str::to_owned).collect()),
            });
        }
        ShadowingReport {
            parts,
            unshadowed,
            casters: casters.into_iter().map(str::to_owned).collect(),
            coverers: self.reports_covers.then(|| coverers.into_iter().collect()),
            lost: None,
        }
    }
}

mod cells;
use cells::{exposed_per_cell, split_cells};
