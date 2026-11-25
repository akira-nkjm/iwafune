//! 面の日照割合（遠い光源 = 太陽に対して、ほかの部品の箱が落とす影を引いた、面の照らされる割合）。
//! **箱どうしの影の計算はここだけ**: 太陽電池の発電（`power::solar`）・熱の外面の直達日射（`thermal::orbit_heat`）・
//! 外形の面積の報告が、同じ関数を使う。幾何は `exposed`（面の被覆）と同じ材料（箱の面・平面の凸多角形の和集合 `sat_astro::math::planar`）。
//!
//! 規則（光源は無限遠の点光源。太陽の視直径による半影は扱わない）。面 `F`（箱の面の長方形、外向き法線 `n`、部品 `P` の箱）と、
//! 光源の向き `d`（単位ベクトル。面から光源へ向かう）について:
//! - **裏面**: `n·d ≤ GRAZING_COS`（約 0。面にほぼ平行な光を含む）なら面は光を受けない（日照割合 0）。箱は自分の面を隠さない（箱は凸なので、自分の面の手前に自分は無い）。
//! - **遮る箱**: `P` 以外の全部の箱。面の平面より手前（`n` の側）にある部分だけが影を作る。面の平面より [`SURFACE_TOLERANCE`]
//!   以上手前に出ない箱は遮らない（面と同じ平面に並ぶ隣の箱、面の裏に付く構体のパネル。取付の面合わせの許容で、
//!   太陽電池が取り付けられた先のパネルに「影にされる」ことを避ける）。手前に出る箱は、平面より手前の部分（箱の頂点と、平面を横切る辺の交点の凸包）を、
//!   向き `d` に沿って面の平面へ射影した凸多角形が影になる（平面から高さ `h` の点は、面の中で `h·(d − n(n·d)) / (n·d)` だけ横にずれる）。
//! - 日照割合 = 1 − （影の凸多角形を面の長方形で切り取った和集合の面積）/ 面の面積。重なる影は 1 度だけ数える。
//!   日照割合は 1e-12（相対）以内で 0 か 1 に丸める（引き算の丸め誤差の除去）。
//!
//! - **面に載る部品**（[`Footprint`]）: 面に取り付けた部品（面の平面に接する・横切る箱と、取付の連鎖で面の部品の上にある部品。取付の隙間は許容に頼らない）は、
//!   面の一部を覆う。**太陽電池は覆う**（[`Footprint::Blocks`]。その下のセルは光を受けない）。**熱の外面は、自分の外面を持たない部品を覆わない**（[`Footprint::Credited`]。
//!   面の光学特性は、その部品を含めた表面を表す慣例なので、部品の真下（面への正射影）は日照のまま数え、その外へ伸びる影だけを引く）。
//!   取り付けられた部品が自分の熱の外面を持つなら（パネルの上のセル）接地面の特別扱いはなく、覆い・向き合い・影の通常の規則で持ち主の面を暗くする（#472。同じ面積を二度数えない）。
//!
//! - **接地面の意味**（`Credited`）: 載る部品が持ち主を遮らないことだけ。第三の部品の影は接地面の下にも落ちる（暗い範囲は、載る部品の影から接地面を除いたものと、そのほかの影・覆いの和集合）。
//!
//! 面が箱の面のどれにも合わない（部品の書かれた法線が箱の軸から外れる、部品に箱が無い）ときは、影を数えない（日照割合 1）。
//! 一覧は [`LitSurfaces::summary`] が `unshadowed` に残す。
//!
//! 構成: 規則の実装は [`FaceOccluders`]（面ごとに、向きによらない前処理 = 遮りうる箱の平面より手前の頂点と接地面を 1 度だけ作り、
//! 向きごとに頂点をずらして影を作る）。面の一覧と向きごとの使い回し・報告は子の `surfaces`（[`LitSurfaces`]）。
//!
//! **升に分けた板の面**（#456）: 面の一部（升の長方形 [`FaceCell`]。長方形は子の兄弟 `cells`）についても、同じ関数が日照割合・覆われた割合・向き合う面への形態係数を出す
//! （影・接地面・覆いの多角形を升の長方形で切り、升の面積で割る。標本点は使わない）。面全体は「升が無い」場合で、同じ演算をたどる（升に分けない設計の数値は変わらない）。
//! 升の面積で重みづけした日照割合・覆われた割合の和は面全体の値に一致する（和集合の面積は升の和に分けられる）。

use super::super::interference::InterferenceBox;
use super::super::interference::obb::{BoxFace, OrientedBox};
use super::geometry::{ExternalGeometry, SURFACE_TOLERANCE};
use sat_astro::math::Vec3;
use sat_astro::math::planar::{ConvexPolygon, union_measure};
use serde::{Deserialize, Serialize};
use std::cell::OnceCell;

mod covers;
mod dark;
mod regions;
mod shadows;
mod surfaces;
use super::cells::FaceCell;
use covers::{Cover, Facing, covers_of, facing_of};
use dark::{DarkShapes, FixedDark};
pub use surfaces::{LitSurfaces, PartLit, ShadowLoss, ShadowingReport};

/// 面に載る部品（面の平面に接する・横切る箱と、面の部品に取り付けられた部品）の接地面を、面の日照にどう数えるか。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Footprint {
    /// 接地面は光を遮る（接地面の下は日陰）。太陽電池の発電
    Blocks,
    /// 接地面は面の一部として日照のまま数え、接地面の外へ伸びる影だけを引く。熱の外面
    Credited,
}

/// 部品の書かれた法線が箱の面の法線に合うとみなす cos（約 2.6°）。
pub const FACE_MATCH_COS: f64 = 0.999;
/// 面が光を受けるとみなす `n·d` の下限。これ以下（面にほぼ平行な光）は影の長さが `h / (n·d)` で発散して幾何が数値的に不安定になり、
/// 受ける光も `n·d` に比例して 0 に近いので、受けない（日照割合 0）ものとして扱う。
pub const GRAZING_COS: f64 = 1e-9;
/// 影が面の長方形から離れているとみなす幅 [m]（`shadow` が凸包を省く。丸め誤差よりずっと大きい）。
const SHADOW_MISS_MARGIN_M: f64 = 1e-9;
/// 日照割合を 0 か 1 に丸める相対の幅。
const LIT_SNAP: f64 = 1e-12;

impl ExternalGeometry {
    /// 部品 `owner` の箱の面のうち、外向き法線が `normal`（機体座標系。長さは問わない）に最も近い面。
    /// 法線との cos が [`FACE_MATCH_COS`] に届かなければ `None`。
    pub fn face_toward(&self, owner: usize, normal: Vec3) -> Option<BoxFace> {
        let n = normal.normalized()?;
        let part = self.parts().get(owner)?;
        part.shape
            .faces()
            .into_iter()
            .map(|f| (f.normal.dot(n), f))
            .filter(|(c, _)| *c >= FACE_MATCH_COS)
            .max_by(|a, b| a.0.total_cmp(&b.0))
            .map(|(_, f)| f)
    }

    /// 部品 `owner` の箱の面 `face` の日照割合（0〜1。規則はモジュールの説明）。`direction` は光源の向き（機体座標系。長さは問わない）。
    /// 向きが零か非有限なら 0。`credited` は面に載る部品を日照に数える（熱の外面）ときの、箱ごとの「面の部品に取り付けられている」印
    /// （箱の並びと同じ順）で、`None` なら面に載る部品も遮る（太陽電池）。向きを替えて何度も解くなら [`FaceOccluders`] を使う。
    pub fn lit_fraction(
        &self,
        owner: usize,
        face: &BoxFace,
        direction: Vec3,
        credited: Option<&[bool]>,
    ) -> f64 {
        FaceOccluders::new(self, owner, face, credited).lit_fraction(direction)
    }
}

/// 遮りうる箱 1 つの、向きによらない前処理。
#[derive(Debug, Clone)]
struct Occluder {
    /// 箱の並びの中の番号
    part: usize,
    /// 面の平面より手前の部分の頂点（面の座標 `[u, v]` と、平面からの高さ）。箱の頂点と、平面を横切る辺の交点
    points: Vec<([f64; 2], f64)>,
    /// 接地面（面の長方形で切る）。面に載る部品を日照に数えるときだけ
    footprint: Option<ConvexPolygon>,
}

/// 面 1 枚の、遮りうる箱の前処理。向き `d` の影は、頂点を `(d·u, d·v) · h / (n·d)` だけ面の中でずらして凸包を取るだけ。
#[derive(Debug, Clone)]
pub struct FaceOccluders {
    /// 面そのもの（升の長方形を形態係数の長方形に直すのに使う）
    face: BoxFace,
    normal: Vec3,
    u: Vec3,
    v: Vec3,
    area: f64,
    rectangle: ConvexPolygon,
    occluders: Vec<Occluder>,
    /// 面を覆う箱（面の平面に接する・横切る箱。熱の外面だけが [`FaceOccluders::with_covers`] で調べる）
    covers: Vec<Cover>,
    /// 面の前に平行に向き合う、接していない箱（`with_covers` が調べる）
    facing: Vec<Facing>,
    /// 接地面と覆い（向きによらない）の前処理。`lit_fraction` が最初に要るときに作る
    fixed_dark: OnceCell<FixedDark>,
}

impl FaceOccluders {
    /// 部品 `owner` の面 `face` の、遮りうる箱（自分以外で、面の平面より [`SURFACE_TOLERANCE`] を超えて手前に出る箱）を調べる。
    /// `credited` は [`ExternalGeometry::lit_fraction`] と同じ。
    pub fn new(
        geometry: &ExternalGeometry,
        owner: usize,
        face: &BoxFace,
        credited: Option<&[bool]>,
    ) -> Self {
        let rectangle = face.rectangle();
        let occluders = geometry
            .parts()
            .iter()
            .enumerate()
            .filter(|(j, _)| *j != owner)
            .filter_map(|(j, other)| {
                let points = above_plane(&other.shape, face)?;
                let footprint = credited.and_then(|mounted| {
                    let mounted = mounted.get(j).copied().unwrap_or(false);
                    footprint_of(&other.shape, face, mounted, &rectangle)
                });
                Some(Occluder {
                    part: j,
                    points,
                    footprint,
                })
            })
            .collect();
        Self {
            face: *face,
            normal: face.normal,
            u: face.u,
            v: face.v,
            area: face.area(),
            rectangle,
            occluders,
            covers: Vec::new(),
            facing: Vec::new(),
            fixed_dark: OnceCell::new(),
        }
    }

    /// 面を覆う箱を調べる（規則は `covers`）。`exempt` は [`covers::covers_of`] と同じ。覆いは [`FaceOccluders::covered_fraction`] で読み、
    /// 日照割合（[`FaceOccluders::lit_fraction`]）からも覆われた範囲を除く（覆われた面は光を受けない）。
    pub fn with_covers(
        mut self,
        geometry: &ExternalGeometry,
        owner: usize,
        face: &BoxFace,
        exempt: &[bool],
    ) -> Self {
        self.covers = covers_of(geometry, owner, face, exempt);
        let covered: Vec<usize> = self.covers.iter().map(|c| c.part).collect();
        self.facing = facing_of(geometry, owner, face, exempt, &covered);
        self
    }

    /// 面そのもの。
    pub fn face(&self) -> &BoxFace {
        &self.face
    }

    /// 面の面積 [m²]。
    pub fn area(&self) -> f64 {
        self.area
    }

    /// 覆われている割合（0〜1。覆いの範囲の和集合の面積 / 面の面積。1e-12 以内は 0 か 1 に丸める）。向きによらない。
    pub fn covered_fraction(&self) -> f64 {
        self.covered_fraction_among(&|_| true)
    }

    /// [`FaceOccluders::covered_fraction`] を、`keep(箱の番号)` が真の箱だけについて求める。
    pub fn covered_fraction_among(&self, keep: &dyn Fn(usize) -> bool) -> f64 {
        self.covered_fraction_in(keep, None)
    }

    /// 面の前に平行に向き合う箱への形態係数の和（1 で打ち切る。`keep(箱の番号)` が真の箱だけ。積み重なった箱は重ねて数えるので、多めに出る）。
    pub fn facing_fraction_among(&self, keep: &dyn Fn(usize) -> bool) -> f64 {
        self.facing_fraction_in(keep, None)
    }

    /// 面が環境（アルベド・惑星の赤外・宇宙）に開いている割合: `(1 − 覆われた割合) × (1 − 向き合う箱への形態係数の和)`
    /// （`keep` が真の箱だけ）。接して覆う範囲と向き合う面が同じ面積に重なっても二重に引かない（積）。
    pub fn exposed_fraction_among(&self, keep: &dyn Fn(usize) -> bool) -> f64 {
        self.exposed_fraction_in(keep, None)
    }

    /// 面に向き合う（接していない）箱の番号。近い順に、形態係数の和が 1 になるまで（それより奥の箱は、手前の箱の陰で効かない）。
    pub fn facers(&self) -> Vec<usize> {
        let mut near: Vec<&Facing> = self.facing.iter().collect();
        near.sort_by(|a, b| a.gap.total_cmp(&b.gap));
        let mut sum = 0.0;
        let mut out = Vec::new();
        for f in near {
            if sum >= 1.0 {
                break;
            }
            sum += f.factor;
            out.push(f.part);
        }
        out
    }

    /// 面を覆っている箱の番号。
    pub fn coverers(&self) -> Vec<usize> {
        self.covers.iter().map(|c| c.part).collect()
    }

    /// 遮る箱 `o` の影（面の長方形で切る）。`shift` は `(d·u, d·v) / (n·d)`。
    /// 影の頂点（ずらした点）の外接の長方形が、面の長方形から離れていれば、切り取った後は空（面積 0）なので凸包を作らない
    /// （面の長方形は軸に平行で、切り取りは 1 辺の外側に全部の頂点があれば空になる。離れの幅 [`SHADOW_MISS_MARGIN_M`] は丸めを十分に超える）。
    fn shadow(&self, o: &Occluder, shift: (f64, f64)) -> ConvexPolygon {
        let moved = |(p, h): &([f64; 2], f64)| [p[0] - shift.0 * h, p[1] - shift.1 * h];
        let (lo, hi) = (self.rectangle.vertices()[0], self.rectangle.vertices()[2]);
        let mut reach = [
            f64::INFINITY,
            f64::INFINITY,
            f64::NEG_INFINITY,
            f64::NEG_INFINITY,
        ];
        for q in o.points.iter().map(moved) {
            reach = [
                reach[0].min(q[0]),
                reach[1].min(q[1]),
                reach[2].max(q[0]),
                reach[3].max(q[1]),
            ];
        }
        let m = SHADOW_MISS_MARGIN_M;
        if reach[2] < lo[0] - m
            || reach[0] > hi[0] + m
            || reach[3] < lo[1] - m
            || reach[1] > hi[1] + m
        {
            return ConvexPolygon::hull(&[]);
        }
        let shifted: Vec<[f64; 2]> = o.points.iter().map(moved).collect();
        ConvexPolygon::hull(&shifted).intersection(&self.rectangle)
    }

    /// 光源の向き `direction` で面に影を落としている箱の番号（影が接地面の外に面積を持つもの）。面が光を受けなければ空。
    pub fn casters(&self, direction: Vec3) -> Vec<usize> {
        let Some(d) = direction.normalized().filter(|d| d.x.is_finite()) else {
            return Vec::new();
        };
        let cos = self.normal.dot(d);
        if cos <= GRAZING_COS {
            return Vec::new();
        }
        let shift = (d.dot(self.u) / cos, d.dot(self.v) / cos);
        self.occluders
            .iter()
            .filter(|o| {
                let shadow = self.shadow(o, shift);
                let outside = match &o.footprint {
                    Some(f) => {
                        union_measure(&[shadow, f.clone()]).area
                            - union_measure(std::slice::from_ref(f)).area
                    }
                    None => union_measure(&[shadow]).area,
                };
                outside > LIT_SNAP * self.area
            })
            .map(|o| o.part)
            .collect()
    }

    /// 光源の向き `direction` で面が光を受けるか（`n·d` が [`GRAZING_COS`] を超える。長さは問わない）。
    pub fn receives_light(&self, direction: Vec3) -> bool {
        direction
            .normalized()
            .is_some_and(|d| self.normal.dot(d) > GRAZING_COS)
    }

    /// 光源の向き `direction`（機体座標系。長さは問わない）での日照割合。向きが零か非有限なら 0。
    pub fn lit_fraction(&self, direction: Vec3) -> f64 {
        self.lit_fraction_all(direction)
    }

    /// [`FaceOccluders::lit_fraction`] を、`keep(箱の番号)` が真の箱（影を作る箱と、覆う箱）だけについて求める
    /// （一部の部品がなかったとしたときの日照割合。熱の外面を持たない部品が遮る分を知るのに使う）。
    pub fn lit_fraction_among(&self, direction: Vec3, keep: &dyn Fn(usize) -> bool) -> f64 {
        self.dark_shapes(direction, keep)
            .map_or(0.0, |shapes| shapes.lit(None, self.area))
    }

    /// 暗い範囲の多角形（面の長方形で切ったもの）。面が光を受けなければ `None`（日照割合 0）。
    fn dark_shapes(&self, direction: Vec3, keep: &dyn Fn(usize) -> bool) -> Option<DarkShapes> {
        let shift = self.light_shift(direction)?;
        Some(self.dark_from(self.shadows(shift, keep), keep))
    }
}

/// 箱の、面の平面より手前の部分の頂点（面の座標と高さ）。面の平面より `SURFACE_TOLERANCE` を超えて手前に出なければ `None`。
fn above_plane(shape: &OrientedBox, face: &BoxFace) -> Option<Vec<([f64; 2], f64)>> {
    let offset = face.normal.dot(face.center);
    let corners = shape.corners();
    let height: Vec<f64> = corners
        .iter()
        .map(|c| face.normal.dot(*c) - offset)
        .collect();
    if height.iter().all(|h| *h <= SURFACE_TOLERANCE) {
        return None;
    }
    let local = |p: Vec3, h: f64| (face.local(p), h);
    let mut points: Vec<([f64; 2], f64)> = (0..8)
        .filter(|&i| height[i] >= 0.0)
        .map(|i| local(corners[i], height[i]))
        .collect();
    for i in 0..8 {
        for k in 0..3 {
            let j = i | (1 << k);
            if j != i && (height[i] < 0.0) != (height[j] < 0.0) {
                let t = height[i] / (height[i] - height[j]);
                points.push(local(corners[i] + (corners[j] - corners[i]).scale(t), 0.0));
            }
        }
    }
    Some(points)
}

/// 面に載る部品の接地面（面の座標の凸多角形。面の長方形で切る）。`mounted`（面の部品に取り付けられた部品）は箱の面への正射影、
/// そうでなければ面の平面に接する・横切るときの接触面・断面（`SURFACE_TOLERANCE` 以内）。どちらでもなければ `None`。
fn footprint_of(
    shape: &OrientedBox,
    face: &BoxFace,
    mounted: bool,
    rectangle: &ConvexPolygon,
) -> Option<ConvexPolygon> {
    let points: Vec<Vec3> = if mounted {
        shape.corners().to_vec()
    } else {
        let offset = face.normal.dot(face.center);
        shape.plane_section(face.normal, offset, SURFACE_TOLERANCE)?
    };
    let local: Vec<[f64; 2]> = points.iter().map(|p| face.local(*p)).collect();
    Some(ConvexPolygon::hull(&local).intersection(rectangle))
}

/// 影を数えるための入力（部品の箱。面は部品の ID で箱に結びつける）。電源と熱が同じ形で持つ。
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct ShadowingInput {
    /// 寸法を持つ部品の箱（機体座標系。`structure.external_geometry` と同じ入力）
    pub boxes: Vec<InterferenceBox>,
    /// 影を数えるか。省略時は数える（[`DEFAULT_SHADOWING`]。箱があるとき）。設計の `analysis.<系>.shadowing` が切る
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub enabled: Option<bool>,
}

/// 影を数える既定（箱があれば数える。**既定はここだけ**）。
pub const DEFAULT_SHADOWING: bool = true;

impl ShadowingInput {
    /// 影を数えるか（省略は [`DEFAULT_SHADOWING`]）。
    pub fn is_enabled(&self) -> bool {
        self.enabled.unwrap_or(DEFAULT_SHADOWING)
    }
}
