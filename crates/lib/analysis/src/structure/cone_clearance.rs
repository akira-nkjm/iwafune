//! 円錐の遮りの確認（`structure.cone_clearance`）。スラスタの噴流と、センサの視野を、**同じ 1 つの判定**で見る。
//! どちらも「ある点から、ある向きに、ある半頂角で広がる円錐」で、その中に機体の部品の箱が入っていないかを調べる。
//! 円錐を持つもの（担い手。[`ConeCarrier`]）の種類は [`ConeKind`] で区別するだけで、判定・規則・出力は共通。
//! 担い手の属性の対応（どの属性が軸・半頂角か、軸の符号）は射影の表にだけある（アンテナのビームなどは表の 1 行を足すだけ）。
//!
//! **噴流や視野を遮る部品が見つかっても警告で、エラーにも解析の失敗にもしない。** 箱は実際の形より大きいので、
//! 箱が円錐に入っても実物は入らないことがある。判定の文にも箱での近似であることを書く（干渉の確認 `structure.interference` と同じ立場）。
//! 噴流が当たった面への影響（熱・力・汚染）と、天体が視野に入る時間（`aocs.sensor_fov`）は、この解析の範囲外。
//!
//! 規則（モデル化の既定はここに持つ。入力は `Option`）:
//! - **頂点**: 担い手の軸が担い手自身の箱から出る点（箱を持たなければ、部品の原点）。噴射口や受光面は、箱の軸に向いた面にあるとみなす。
//! - **担い手自身の箱は判定しない。取付の連鎖の上にある部品（担い手の取付先の祖先と、担い手を取付先とする子孫）も判定しない**:
//!   スラスタやセンサは取付先の箱の面に付くのが普通で、フレームやパネルの箱は担い手を含むので、いつも円錐に入ってしまう。
//!   兄弟・別の枝の部品（展開したパドルを含む）は、いつも判定する。限界: 担い手が自分の取付先の中へ向いていても見えない。
//! - **距離の上限**: 頂点からの距離がこれ以下の点だけを見る。入力が無ければ、機体の大きさから決める
//!   （全部品の箱と担い手の位置を含む軸平行な箱の対角線。機体の中の 2 点の距離はこれを超えないので、既定では何も除かない）。
//! - **半頂角が無い担い手は判定せず、一覧に残す**（噴流の半頂角は任意の属性なので、何も仮定しない）。軸が無い・値が不正なものも同じ。
//! - 寸法が無い（または正でない）部品は判定せず、一覧に残す（箱の選び方は干渉の確認と共通）。
//!
//! 幾何は [`cone`]（円錐と向きつきの箱の厳密な交差）。箱は干渉の確認と同じ `OrientedBox`・`InterferenceBox`。

pub mod cone;

use super::interference::obb::OrientedBox;
use super::interference::{
    InterferenceBox, InterferenceSkipped, count, on_mount_chain, usable_boxes,
};
use crate::spec::quantity_text;
use crate::spec::{
    Analysis, AnalysisArea, AnalysisError, AnalysisKind, Check, System, Verdict,
    ensure_non_negative,
};
use cone::{Cone, contact};
use sat_astro::math::Vec3;
use sat_units::typed::{Angle, Length, Qty};
use serde::{Deserialize, Serialize};
use std::f64::consts::FRAC_PI_2;

/// 円錐の種類。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(rename_all = "snake_case")]
pub enum ConeKind {
    /// スラスタの噴流（推力の向きの反対へ広がる）
    Plume,
    /// センサの視野（視線のまわり）
    FieldOfView,
}

impl ConeKind {
    /// 判定文の名前（小文字、先頭を大文字にしたもの）。
    fn text(self) -> (&'static str, &'static str) {
        match self {
            Self::Plume => ("plume", "Plume"),
            Self::FieldOfView => ("field of view", "Field of view"),
        }
    }
}

/// 円錐を持つもの 1 つ（スラスタ・センサなど）。
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct ConeCarrier {
    pub id: String,
    pub kind: ConeKind,
    /// 部品の原点の位置（機体座標系）[m]
    pub position: Vec3,
    /// 円錐が開く向き（機体座標系。噴流なら推力の反対。長さは問わず、零は不正）。読めなければ `None`
    #[serde(default)]
    pub axis: Option<Vec3>,
    /// 半頂角（読めなければ `None`）
    #[serde(default)]
    pub half_angle: Option<Qty<Angle>>,
    /// 取付先をたどった部品の ID（近いほうから）。この上にある部品と、これを取付先とする部品は判定しない
    #[serde(default)]
    pub mount_ancestors: Vec<String>,
}

/// 円錐の遮りの確認の入力。
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct ConeClearanceInput {
    /// 寸法を持つ部品の箱（干渉の確認と同じ）
    pub boxes: Vec<InterferenceBox>,
    /// 寸法を持たないので判定しない部品の ID
    #[serde(default)]
    pub unsized_parts: Vec<String>,
    /// 円錐を持つもの
    pub carriers: Vec<ConeCarrier>,
    /// 頂点からの距離の上限（無ければ機体の大きさから決める）
    #[serde(default)]
    pub range: Option<Qty<Length>>,
}

/// 判定しなかった担い手の理由。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(rename_all = "snake_case")]
pub enum ConeSkipReason {
    /// 円錐の軸（向きの属性）が読めない
    NoAxis,
    /// 軸が零・非有限
    InvalidAxis,
    /// 半頂角（属性）が無い
    NoHalfAngle,
    /// 半頂角が 0 より大きく 90° より小さい値でない
    InvalidHalfAngle,
}

/// 判定しなかった担い手。
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct ConeCarrierSkipped {
    pub id: String,
    pub kind: ConeKind,
    pub reason: ConeSkipReason,
}

/// 円錐の中に入る部品 1 つ。
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct ConeObstruction {
    /// 円錐に入る箱の部品
    pub part: String,
    /// 軸から箱の点までの最小の角度（軸が箱に当たれば 0）
    pub min_angle: Qty<Angle>,
    /// 頂点から、箱のうち円錐の中にある部分までの最短距離
    pub min_distance: Qty<Length>,
}

/// 判定した円錐 1 つ。
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct ConeReport {
    /// 担い手の部品
    pub id: String,
    pub kind: ConeKind,
    pub half_angle: Qty<Angle>,
    /// 軸の向き（機体座標系の単位ベクトル）
    pub cone_axis: Vec3,
    /// 頂点（機体座標系）[m]
    pub apex: Vec3,
    /// 頂点からの距離の上限（使った値）
    pub range: Qty<Length>,
    /// 判定した箱の数（担い手自身と取付の連鎖の上の部品を除く）
    pub parts_judged: u32,
    /// 取付の連鎖の上にあるので判定しなかった箱の数（担い手自身は数えない）
    pub parts_in_mount_chain: u32,
    /// 円錐に入る部品（頂点に近い順）
    pub obstructions: Vec<ConeObstruction>,
}

/// 円錐の遮りの確認の出力。
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct ConeClearanceOutput {
    /// 使った距離の上限
    pub range: Qty<Length>,
    /// 上限が既定（機体の大きさから決めた値）か
    pub range_is_default: bool,
    /// 箱を持つ部品の数
    pub parts_with_box: u32,
    /// 判定した円錐の数
    pub cones_judged: u32,
    /// 円錐に部品が入っている担い手の数
    pub cones_obstructed: u32,
    /// 円錐ごとの結果（担い手の ID の昇順）
    pub cones: Vec<ConeReport>,
    /// 判定しなかった担い手
    pub skipped_carriers: Vec<ConeCarrierSkipped>,
    /// 寸法が無いので判定しなかった部品
    pub skipped: Vec<InterferenceSkipped>,
    /// 判定（円錐に入る部品ごとの警告。無ければ ok の 1 つ）
    pub checks: Vec<Check>,
}

/// 円錐の遮りの確認。
pub struct ConeClearance;

impl Analysis for ConeClearance {
    const ID: &'static str = "structure.cone_clearance";
    const TITLE: &'static str = "噴流・視野の遮り";
    const AREA: AnalysisArea = AnalysisArea::System(System::Structure);
    const KIND: AnalysisKind = AnalysisKind::Steady;
    const DESCRIPTION: &'static str = "スラスタの噴流の円錐と、センサの視野の円錐に、機体の部品の箱が入っていないか（箱での近似。入っていれば警告）";
    const TITLE_EN: &'static str = "Plume and FoV clearance";
    const DESCRIPTION_EN: &'static str = "Whether the boxes of the spacecraft's own parts lie inside a thruster plume cone or a sensor field-of-view cone (a box approximation; a hit is a warning)";
    type Input = ConeClearanceInput;
    type Output = ConeClearanceOutput;

    fn run(input: &Self::Input) -> Result<Self::Output, AnalysisError> {
        if let Some(r) = input.range {
            ensure_non_negative("Cone clearance range", r.value())?;
        }
        let (judged, mut skipped) = usable_boxes(&input.boxes, &input.unsized_parts);
        skipped.sort_by(|a, b| a.id.cmp(&b.id));
        let range = input
            .range
            .unwrap_or_else(|| default_range(&judged, &input.carriers));
        let placed: Vec<OrientedBox> = judged
            .iter()
            .map(|b| OrientedBox::from_size(b.position, b.orientation, b.size))
            .collect();
        let mut carriers: Vec<&ConeCarrier> = input.carriers.iter().collect();
        carriers.sort_by(|a, b| a.id.cmp(&b.id));
        let mut cones = Vec::new();
        let mut skipped_carriers = Vec::new();
        for c in carriers {
            match cone_of(c, &judged, &placed, range) {
                Ok(cone) => cones.push(report(c, &cone, &judged, &placed, range)),
                Err(reason) => skipped_carriers.push(ConeCarrierSkipped {
                    id: c.id.clone(),
                    kind: c.kind,
                    reason,
                }),
            }
        }
        let checks = checks(&cones, skipped_carriers.len(), judged.len(), range);
        Ok(ConeClearanceOutput {
            range,
            range_is_default: input.range.is_none(),
            parts_with_box: count(judged.len()),
            cones_judged: count(cones.len()),
            cones_obstructed: count(cones.iter().filter(|c| !c.obstructions.is_empty()).count()),
            cones,
            skipped_carriers,
            skipped,
            checks,
        })
    }
}

/// 距離の上限の既定: 全部品の箱と担い手の位置を含む、軸平行な箱の対角線 [m]。
fn default_range(boxes: &[&InterferenceBox], carriers: &[ConeCarrier]) -> Qty<Length> {
    let mut lo = Vec3::new(f64::INFINITY, f64::INFINITY, f64::INFINITY);
    let mut hi = Vec3::new(f64::NEG_INFINITY, f64::NEG_INFINITY, f64::NEG_INFINITY);
    let mut include = |p: Vec3, ext: Vec3| {
        lo = Vec3::new(
            lo.x.min(p.x - ext.x),
            lo.y.min(p.y - ext.y),
            lo.z.min(p.z - ext.z),
        );
        hi = Vec3::new(
            hi.x.max(p.x + ext.x),
            hi.y.max(p.y + ext.y),
            hi.z.max(p.z + ext.z),
        );
    };
    for b in boxes {
        let o = OrientedBox::from_size(b.position, b.orientation, b.size);
        let (u, h) = (o.axes(), o.half());
        // 箱の世界座標の各軸への張り出し `Σ_i h_i |u_i[k]|`
        let ext = Vec3::new(
            (0..3).map(|i| h[i] * u[i].x.abs()).sum(),
            (0..3).map(|i| h[i] * u[i].y.abs()).sum(),
            (0..3).map(|i| h[i] * u[i].z.abs()).sum(),
        );
        include(b.position, ext);
    }
    for c in carriers {
        if c.position.to_array().iter().all(|v| v.is_finite()) {
            include(c.position, Vec3::ZERO);
        }
    }
    if lo.x > hi.x {
        return Qty::from_raw(0.0);
    }
    Qty::from_raw((hi - lo).norm())
}

/// 担い手の円錐（頂点は軸が自身の箱から出る点）。作れなければ理由。
fn cone_of(
    c: &ConeCarrier,
    boxes: &[&InterferenceBox],
    placed: &[OrientedBox],
    range: Qty<Length>,
) -> Result<Cone, ConeSkipReason> {
    let axis = c.axis.ok_or(ConeSkipReason::NoAxis)?;
    let half = c.half_angle.ok_or(ConeSkipReason::NoHalfAngle)?.value();
    let unit = axis
        .normalized()
        .filter(|u| u.to_array().iter().all(|v| v.is_finite()))
        .ok_or(ConeSkipReason::InvalidAxis)?;
    if !(half.is_finite() && half > 0.0 && half < FRAC_PI_2) {
        return Err(ConeSkipReason::InvalidHalfAngle);
    }
    // 自身の箱があれば、軸が箱から出る点（箱の中心から軸の向きの支持量だけ進んだ点）
    let own = boxes.iter().position(|b| b.id == c.id);
    let exit = own.map_or(0.0, |i| {
        let (u, h) = (placed[i].axes(), placed[i].half());
        (0..3).map(|k| h[k] * unit.dot(u[k]).abs()).sum()
    });
    let apex = c.position + unit.scale(exit);
    Cone::new(apex, unit, half, Some(range.value())).ok_or(ConeSkipReason::InvalidAxis)
}

/// 円錐 1 つを全部品の箱と比べる。
fn report(
    c: &ConeCarrier,
    cone: &Cone,
    boxes: &[&InterferenceBox],
    placed: &[OrientedBox],
    range: Qty<Length>,
) -> ConeReport {
    let mut judged = 0;
    let mut related = 0;
    let mut obstructions = Vec::new();
    for (b, shape) in boxes.iter().zip(placed) {
        if b.id == c.id {
            continue;
        }
        if on_mount_chain(&c.id, &c.mount_ancestors, &b.id, &b.mount_ancestors) {
            related += 1;
            continue;
        }
        judged += 1;
        if let Some(hit) = contact(cone, shape) {
            obstructions.push(ConeObstruction {
                part: b.id.clone(),
                min_angle: Qty::from_raw(hit.min_angle),
                min_distance: Qty::from_raw(hit.min_distance),
            });
        }
    }
    obstructions.sort_by(|x, y| {
        x.min_distance
            .value()
            .total_cmp(&y.min_distance.value())
            .then_with(|| x.part.cmp(&y.part))
    });
    ConeReport {
        id: c.id.clone(),
        kind: c.kind,
        half_angle: Qty::from_raw(cone.half_angle),
        cone_axis: cone.axis,
        apex: cone.apex,
        range,
        parts_judged: count(judged),
        parts_in_mount_chain: count(related),
        obstructions,
    }
}

/// 判定。円錐に入る部品は（担い手, 部品）ごとの警告で、無ければ ok の 1 つ。**どれも違反にはしない。**
fn checks(cones: &[ConeReport], skipped: usize, parts: usize, range: Qty<Length>) -> Vec<Check> {
    let warnings: Vec<Check> = cones.iter().flat_map(obstruction_checks).collect();
    if !warnings.is_empty() {
        return warnings;
    }
    let message = if cones.is_empty() {
        format!(
            "No thruster plume or sensor field of view could be judged ({skipped} carriers skipped, see the list)"
        )
    } else {
        format!(
            "No part box lies inside the {} cones judged (range {}, {parts} parts with a box, {skipped} carriers skipped; boxes are an approximation of the real shapes)",
            cones.len(),
            quantity_text::<Length>(range.value())
        )
    };
    vec![Check::new(
        "cone_clearance",
        "Plume and field-of-view clearance",
        Verdict::Ok,
        message,
    )]
}

/// 円錐 1 つの警告（入る部品ごと）。
fn obstruction_checks(c: &ConeReport) -> Vec<Check> {
    c.obstructions
        .iter()
        .map(|o| {
            Check::new(
                &sat_checkkey::cone_obstruction(&c.id, &o.part),
                &format!("{} of {} reaches {}", c.kind.text().1, c.id, o.part),
                Verdict::Warning,
                format!(
                    "The box of {} lies inside the {} cone of {} (half angle {}, range {}): {} from the axis at the nearest, {} from the apex at the nearest. Boxes are larger than the real shapes, so the real part may not be in the cone (box approximation)",
                    o.part,
                    c.kind.text().0,
                    c.id,
                    quantity_text::<Angle>(c.half_angle.value()),
                    quantity_text::<Length>(c.range.value()),
                    quantity_text::<Angle>(o.min_angle.value()),
                    quantity_text::<Length>(o.min_distance.value())
                ),
            )
        })
        .collect()
}

#[cfg(test)]
mod tests;
