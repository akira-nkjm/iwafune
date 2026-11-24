//! 面の一覧（部品の ID と外向き法線）の、向きごとの日照割合と、その報告。規則と前処理は親の `lit`（[`FaceOccluders`]）。

use super::super::geometry::{ExternalGeometry, direction_key};
use super::{FaceOccluders, Footprint, ShadowingInput};
use sat_astro::math::Vec3;
use sat_units::typed::{Fraction, Power, Qty};
use serde::{Deserialize, Serialize};
use std::cell::RefCell;
use std::collections::HashMap;

/// 面 1 枚の、測った向きの中での日照割合の最小と平均。
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct PartLit {
    /// 面の部品の ID
    pub id: String,
    /// 光を受けた向きの中での日照割合の最小
    pub min: Fraction,
    /// 光を受けた向きの中での日照割合の平均（標本の単純平均）
    pub mean: Fraction,
    /// 面が光を受けた（法線と光源の向きの cos が正の）標本の数。0 なら `min`・`mean` は 1
    pub samples: u32,
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
    /// 影で遮られた直達日射のうち、影を落とす部品が外面を持たず、どこにも吸収されない分の報告（熱の解析だけ）
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub lost: Option<ShadowLoss>,
}

/// 影を落とす部品が熱の外面を持たないために、吸収されずに消える日射の報告（熱の解析）。
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct ShadowLoss {
    /// 面に影を落とすが、自分の熱の外面を持たない部品（その部品が遮った日射はどの節点にも吸収されない）
    pub parts: Vec<String>,
    /// 影を数えない入熱の合計 − 影を数えた入熱の合計（外面の全部の合計）
    pub power: Qty<Power>,
    /// `power` / 影を数えた入熱の合計
    pub fraction: Fraction,
}

/// 箱ごとの、部品 `owner` に（取付の連鎖で）取り付けられているか。`geometry` の箱の並びと同じ順。
fn mounted_on(input: &ShadowingInput, geometry: &ExternalGeometry, owner: &str) -> Vec<bool> {
    geometry
        .parts()
        .iter()
        .map(|p| {
            input
                .boxes
                .iter()
                .find(|b| b.id == p.id)
                .is_some_and(|b| b.mount_ancestors.iter().any(|a| a == owner))
        })
        .collect()
}

/// 面の一覧の、向きごとの日照割合。面ごとの前処理と、向きごとの結果の使い回しを持つ。
/// 固定姿勢の機体では光源の向きが一定なので、向きごとの結果は 1 度だけ解く（[`direction_key`]。浮動小数点の雑音の外で結果は変わらない）。
#[derive(Debug, Clone)]
pub struct LitSurfaces {
    ids: Vec<String>,
    /// 箱の部品の ID（`faces` の箱の番号と同じ順）
    part_ids: Vec<String>,
    /// 面ごとの前処理。箱の面に合わなければ `None`
    faces: Vec<Option<FaceOccluders>>,
    cache: RefCell<HashMap<(usize, [i64; 3]), f64>>,
}

impl LitSurfaces {
    /// 箱と面の一覧から作る。影を切ってある（`enabled` が偽）か、使える箱が 1 つも無ければ `None`（影を数えない）。
    /// 面の部品は ID で箱に結びつける。
    pub fn new<'a>(
        input: &ShadowingInput,
        footprint: Footprint,
        surfaces: impl IntoIterator<Item = (&'a str, Vec3)>,
    ) -> Option<Self> {
        if !input.is_enabled() {
            return None;
        }
        let geometry = ExternalGeometry::from_boxes(&input.boxes)?;
        let (ids, faces): (Vec<String>, Vec<_>) = surfaces
            .into_iter()
            .map(|(id, normal)| {
                let face = geometry
                    .parts()
                    .iter()
                    .position(|p| p.id == id)
                    .and_then(|owner| {
                        let face = geometry.face_toward(owner, normal)?;
                        let mounted = mounted_on(input, &geometry, id);
                        let credited = (footprint == Footprint::Credited).then_some(&mounted[..]);
                        Some(FaceOccluders::new(&geometry, owner, &face, credited))
                    });
                (id.to_owned(), face)
            })
            .unzip();
        let part_ids = geometry.parts().iter().map(|p| p.id.clone()).collect();
        Some(Self {
            ids,
            part_ids,
            faces,
            cache: RefCell::new(HashMap::new()),
        })
    }

    /// `id` の面を持つか（面の一覧に入れた部品か）。
    pub fn has_surface(&self, id: &str) -> bool {
        self.ids.iter().any(|x| x == id)
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

    /// 光を受けた向き（標本の光源の向き。日陰の標本は呼び出し側が除く）ごとの日照割合の最小と平均。
    pub fn summary(&self, directions: &[Vec3]) -> ShadowingReport {
        let mut parts = Vec::new();
        let mut unshadowed = Vec::new();
        let mut casters = std::collections::BTreeSet::new();
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
            parts.push(PartLit {
                id: self.ids[i].clone(),
                min: Fraction::new(lit.iter().copied().fold(1.0, f64::min)),
                mean: Fraction::new(if n == 0 {
                    1.0
                } else {
                    lit.iter().sum::<f64>() / n as f64
                }),
                samples: u32::try_from(n).unwrap_or(u32::MAX),
            });
        }
        ShadowingReport {
            parts,
            unshadowed,
            casters: casters.into_iter().map(str::to_owned).collect(),
            lost: None,
        }
    }
}
