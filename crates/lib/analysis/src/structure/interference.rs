//! 部品どうしの干渉の確認（`structure.interference`）。寸法（`size`）を持つ部品の箱を機体座標に置き、箱どうしの重なりを組ごとに調べる。
//!
//! **重なりは警告で、エラーにも解析の失敗にもしない。** 箱は実際の形（円筒・切り欠き・肉抜き）より大きく取っているので、
//! 箱が重なっても実物は当たらないことがある。判定の文にも箱での近似であることを書く。
//!
//! 規則（モデル化の既定はここに持つ。入力の許容差は `Option`）:
//! - **許容差**: 重なりの深さが許容差以下なら、干渉に数えない（接している、わずかな食い込みは取付の面の普通の姿）。
//!   既定は [`DEFAULT_TOLERANCE`]（0.1 mm。CAD の寸法の丸めと取付の面の合わせの余裕で、箱の近似の粗さ（mm〜cm）より十分小さい）。
//! - **取付の祖先・子孫の組は判定しない**: 部品は取付先に対して置かれ、フレームやパネルは箱の中に子を収めるのが普通で、
//!   取付の面では接して食い込む。祖先の箱は子孫を含んでいてよい箱として扱い、祖先と子孫の組（取付の連鎖の上にある 2 つ。
//!   間に箱を持たない部品があってもよい）は数えない。子孫どうし（兄弟）や別の枝の部品との組は、いつも判定する。
//!   限界: 子が自分の祖先の箱から外へはみ出す・祖先の箱の中で別の部品に当たらない、は祖先との組では見えない（箱では収容と干渉を区別できない）。
//! - **員数（`count`）が 2 以上の部品は 1 つの位置に重ねて置く**: 同じ部品どうし（自分自身・複製）は比べない。ほかの部品との組は 1 つの箱で判定する。
//! - 寸法が無い（または正でない）部品は判定せず、一覧に残す。箱が 2 つに満たなければ、重なり無しの成功にする。
//!
//! 幾何は [`obb`]（向きつきの箱の分離軸の判定）。

pub mod obb;

use crate::spec::quantity_text;
use crate::spec::{
    Analysis, AnalysisArea, AnalysisError, AnalysisKind, Check, System, Verdict,
    ensure_non_negative,
};
use obb::{OrientedBox, separation};
use sat_astro::math::{Quat, Vec3};
use sat_units::typed::{Dimensionless, Length, Qty};
use serde::{Deserialize, Serialize};

/// 重なりの深さの許容差の既定 [m]（0.1 mm）。これ以下の重なりは干渉に数えない。
pub const DEFAULT_TOLERANCE: Qty<Length> = Qty::from_raw(1e-4);

/// 判定する箱 1 つ（寸法を持つ部品）。
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct InterferenceBox {
    pub id: String,
    /// 員数（同じ位置に重ねて置く台数。同じ部品どうしは比べない。省略で 1）
    #[serde(default = "crate::spec::default_count")]
    pub count: u32,
    /// 部品の原点（箱の中心）の位置（機体座標系）[m]
    pub position: Vec3,
    /// 部品 → 機体座標系の回転
    pub orientation: Quat,
    /// 箱の辺の長さ（部品座標系）[m]
    pub size: Vec3,
    /// 取付先をたどった部品の ID（近いほうから。寸法を持たない部品も含む）。この上にある部品との組は判定しない
    #[serde(default)]
    pub mount_ancestors: Vec<String>,
}

/// 干渉の確認の入力。
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct InterferenceInput {
    /// 寸法を持つ部品の箱
    pub boxes: Vec<InterferenceBox>,
    /// 寸法を持たないので判定しない部品の ID
    #[serde(default)]
    pub unsized_parts: Vec<String>,
    /// 重なりの深さの許容差（無ければ [`DEFAULT_TOLERANCE`]）
    #[serde(default)]
    pub tolerance: Option<Qty<Length>>,
}

/// 重なっている組 1 つ。
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct InterferencePair {
    /// 組の 2 つの部品（ID の昇順）
    pub first: String,
    pub second: String,
    /// 重なりの深さ（最も浅く分離できる量）
    pub depth: Qty<Length>,
    /// 分離の向き（機体座標系の単位ベクトル。`first` から `second` へ。`second` をこの向きに深さだけ動かせば重なりが解ける）
    pub separation_axis: Vec3,
}

/// 離れている組のうち、最も近い組。
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct InterferenceClearance {
    pub first: String,
    pub second: String,
    /// 最良の分離軸での隙間（真の最短距離の下限。接していれば 0）
    pub gap: Qty<Length>,
}

/// 判定しなかった部品の理由。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(rename_all = "snake_case")]
pub enum InterferenceSkipReason {
    /// 寸法（`size`）が無い
    NoSize,
    /// 寸法が正の有限値でない
    InvalidSize,
}

/// 判定しなかった部品。
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct InterferenceSkipped {
    pub id: String,
    pub reason: InterferenceSkipReason,
}

/// 員数が 2 以上で、1 つの箱として判定した部品（複製どうしは比べない）。
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct InterferenceRepeated {
    pub id: String,
    pub count: u32,
}

/// 干渉の確認の出力。
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct InterferenceOutput {
    /// 使った許容差
    pub tolerance: Qty<Length>,
    /// 判定した部品の数（寸法を持つ部品）
    pub parts_judged: u32,
    /// 判定した組の数
    pub pairs_judged: u32,
    /// 取付の祖先・子孫なので判定しなかった組の数
    pub pairs_in_mount_chain: u32,
    /// 重なっている組（深い順）
    pub overlaps: Vec<InterferencePair>,
    /// 重なっていない組のうち最も近いもの（組が無ければ `None`）
    pub min_clearance: Option<InterferenceClearance>,
    /// 判定しなかった部品
    pub skipped: Vec<InterferenceSkipped>,
    /// 員数が 2 以上の部品
    pub repeated: Vec<InterferenceRepeated>,
    /// 判定（重なりは組ごとの警告。重なりが無ければ ok の 1 つ）
    pub checks: Vec<Check>,
}

/// 干渉の確認。
pub struct Interference;

impl Analysis for Interference {
    const ID: &'static str = "structure.interference";
    const TITLE: &'static str = "部品の干渉";
    const AREA: AnalysisArea = AnalysisArea::System(System::Structure);
    const KIND: AnalysisKind = AnalysisKind::Steady;
    const DESCRIPTION: &'static str =
        "寸法を持つ部品の箱どうしが重なっていないか（箱での近似。重なりは警告）";
    const TITLE_EN: &'static str = "Part interference";
    const DESCRIPTION_EN: &'static str = "Whether the boxes of parts that have a size overlap (a box approximation; an overlap is a warning)";
    type Input = InterferenceInput;
    type Output = InterferenceOutput;

    fn run(input: &Self::Input) -> Result<Self::Output, AnalysisError> {
        let tolerance = input.tolerance.unwrap_or(DEFAULT_TOLERANCE);
        ensure_non_negative("Interference tolerance", tolerance.value())?;
        let (judged, mut skipped) = usable_boxes(input);
        let scan = scan_pairs(&judged, tolerance.value());
        skipped.sort_by(|a, b| a.id.cmp(&b.id));
        let repeated = judged
            .iter()
            .filter(|b| b.count > 1)
            .map(|b| InterferenceRepeated {
                id: b.id.clone(),
                count: b.count,
            })
            .collect();
        let checks = checks(&scan, judged.len(), tolerance);
        Ok(InterferenceOutput {
            tolerance,
            parts_judged: count(judged.len()),
            pairs_judged: count(scan.judged),
            pairs_in_mount_chain: count(scan.related),
            overlaps: scan.overlaps,
            min_clearance: scan.clearance,
            skipped,
            repeated,
            checks,
        })
    }
}

fn count(n: usize) -> u32 {
    u32::try_from(n).unwrap_or(u32::MAX)
}

/// 判定できる箱（ID の昇順）と、判定しない部品（寸法が無い・正の有限値でない）。
fn usable_boxes(input: &InterferenceInput) -> (Vec<&InterferenceBox>, Vec<InterferenceSkipped>) {
    let mut skipped: Vec<InterferenceSkipped> = input
        .unsized_parts
        .iter()
        .map(|id| InterferenceSkipped {
            id: id.clone(),
            reason: InterferenceSkipReason::NoSize,
        })
        .collect();
    let mut judged = Vec::new();
    for b in &input.boxes {
        let positive = [b.size.x, b.size.y, b.size.z]
            .iter()
            .all(|v| v.is_finite() && *v > 0.0);
        let finite = [b.position.x, b.position.y, b.position.z]
            .iter()
            .all(|v| v.is_finite());
        if positive && finite {
            judged.push(b);
        } else {
            skipped.push(InterferenceSkipped {
                id: b.id.clone(),
                reason: InterferenceSkipReason::InvalidSize,
            });
        }
    }
    judged.sort_by(|a, b| a.id.cmp(&b.id));
    (judged, skipped)
}

/// 全組の走査の結果。
struct Scan {
    judged: usize,
    related: usize,
    overlaps: Vec<InterferencePair>,
    clearance: Option<InterferenceClearance>,
}

/// 取付の連鎖の上にある（片方がもう片方の祖先の）組か。
fn in_mount_chain(a: &InterferenceBox, b: &InterferenceBox) -> bool {
    a.mount_ancestors.contains(&b.id) || b.mount_ancestors.contains(&a.id)
}

/// 全組を調べる（組の順は ID の昇順で固定。軸は `a` から `b` へ）。
fn scan_pairs(boxes: &[&InterferenceBox], tolerance: f64) -> Scan {
    let placed: Vec<OrientedBox> = boxes
        .iter()
        .map(|b| OrientedBox::from_size(b.position, b.orientation, b.size))
        .collect();
    let mut scan = Scan {
        judged: 0,
        related: 0,
        overlaps: Vec::new(),
        clearance: None,
    };
    for i in 0..boxes.len() {
        for j in (i + 1)..boxes.len() {
            let (a, b) = (boxes[i], boxes[j]);
            // 同じ ID の箱は同じ部品として比べない
            if a.id == b.id {
                continue;
            }
            if in_mount_chain(a, b) {
                scan.related += 1;
                continue;
            }
            scan.judged += 1;
            let s = separation(&placed[i], &placed[j]);
            if s.depth() > tolerance {
                scan.overlaps.push(InterferencePair {
                    first: a.id.clone(),
                    second: b.id.clone(),
                    depth: Qty::from_raw(s.depth()),
                    separation_axis: s.axis,
                });
            } else {
                let gap = s.distance.max(0.0);
                let nearer = scan.clearance.as_ref().is_none_or(|c| gap < c.gap.value());
                if nearer {
                    scan.clearance = Some(InterferenceClearance {
                        first: a.id.clone(),
                        second: b.id.clone(),
                        gap: Qty::from_raw(gap),
                    });
                }
            }
        }
    }
    scan.overlaps.sort_by(|x, y| {
        y.depth
            .value()
            .total_cmp(&x.depth.value())
            .then_with(|| (&x.first, &x.second).cmp(&(&y.first, &y.second)))
    });
    scan
}

/// 向きの成分の文（丸めた -0 は 0 にする）。
fn direction_text(axis: Vec3) -> String {
    let part = |v: f64| quantity_text::<Dimensionless>(if v.abs() < 1e-9 { 0.0 } else { v });
    format!("({}, {}, {})", part(axis.x), part(axis.y), part(axis.z))
}

/// 判定。重なりは組ごとの警告で、無ければ ok の 1 つ。**どれも違反にはしない。**
fn checks(scan: &Scan, parts: usize, tolerance: Qty<Length>) -> Vec<Check> {
    if scan.overlaps.is_empty() {
        let message = if parts < 2 {
            "Fewer than two parts have a size, so there is nothing to compare".to_owned()
        } else {
            format!(
                "No boxes overlap by more than the tolerance {} ({parts} parts, {} pairs judged; boxes are an approximation of the real shapes)",
                quantity_text::<Length>(tolerance.value()),
                scan.judged
            )
        };
        return vec![Check::new(
            "interference",
            "Part interference",
            Verdict::Ok,
            message,
        )];
    }
    scan.overlaps
        .iter()
        .map(|p| {
            Check::new(
                &sat_checkkey::part_overlap(&p.first, &p.second),
                &format!("Boxes of {} and {} overlap", p.first, p.second),
                Verdict::Warning,
                format!(
                    "Overlap {} (tolerance {}): moving {} by this amount along {} separates the boxes. Boxes are larger than the real shapes, so this may not be a real contact (box approximation)",
                    quantity_text::<Length>(p.depth.value()),
                    quantity_text::<Length>(tolerance.value()),
                    p.second,
                    direction_text(p.separation_axis)
                ),
            )
        })
        .collect()
}

#[cfg(test)]
mod tests;
