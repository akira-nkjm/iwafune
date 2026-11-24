//! 打ち上げ適合の外形の判定。収納状態の外形（軸ごとの長さ）を、許容の箱と **回転せずに軸ごとに** 比べる。
//!
//! 外形の出どころは 2 つ（[`LaunchFitEnvelopeSource`]）。**どちらを使うかはここで決める**（優先は部品の箱）:
//! - 部品の箱 ([`LaunchFitInput::part_boxes`] があるとき): 寸法を持つ全部品の向きつきの箱（機体座標）を囲む、軸に平行な最小の箱
//!   （[`axis_aligned_extents`]）。軸ごとの長さ = 最大 − 最小。端を決めた部品（最小・最大に届く箱）を判定の文に書く。
//!   包絡の中心の機体原点からのずれは情報として添える（判定には使わない。収納した機体は中心に収まらないのが普通）。
//!   寸法を持たない部品は測れないので、一覧にする。
//! - 取付の根の部品の寸法（`size`。部品の箱が無いとき）: 従来どおり。
//!
//! 比べるのは長さ（包絡の幅）で、許容の箱の中での置き場所ではない（打ち上げ機の座標系での位置は設計に無い）。

use super::{FIT_MARGIN_WARNING_BAND, LaunchFitInput, positive};
use crate::spec::quantity_text;
use crate::spec::{AnalysisError, Check, Verdict, ratio_margin, ratio_margin_verdict};
use crate::structure::interference::obb::{OrientedBox, axis_aligned_extents};
use crate::structure::interference::{InterferenceBox, InterferenceSkipped, usable_boxes};
use sat_astro::math::Vec3;
use sat_units::typed::{Fraction, Length, Qty};
use serde::{Deserialize, Serialize};

/// 軸の名前（機体座標。外形の並びと同じ順）。
const AXES: [&str; 3] = ["x", "y", "z"];

/// 2 つの端が同じとみなす差 [m]（1 nm。同じ面に揃えた箱の丸め誤差）。
const TIE: f64 = 1e-9;

/// 許容の外形（機体座標の軸ごとの長さ [m]）。
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct LaunchFitEnvelope {
    /// 値の出所
    pub origin: String,
    /// x, y, z 方向の許容の長さ [m]（z が発射軸）
    pub size: Vec3,
}

/// 収納外形の出どころ。
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(rename_all = "snake_case")]
pub enum LaunchFitEnvelopeSource {
    /// 取付の根の部品の寸法（`size`）
    #[default]
    RootSize,
    /// 寸法を持つ全部品の箱を囲む範囲（打ち上げ条件が形態を指しているとき）
    Parts,
}

impl LaunchFitEnvelopeSource {
    /// 既定（根の寸法）か。出力に書かない（形態を指さない設計の結果を変えないため）。
    pub fn is_root_size(&self) -> bool {
        *self == Self::RootSize
    }
}

/// 外形を出す部品の箱（機体座標）。
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct LaunchFitPartBoxes {
    /// 寸法を持つ部品の箱
    pub boxes: Vec<InterferenceBox>,
    /// 寸法を持たないので測れない部品の ID
    #[serde(default)]
    pub unsized_parts: Vec<String>,
}

/// 外形の 1 軸の判定。
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct LaunchFitAxis {
    /// `x`・`y`・`z`
    pub axis: String,
    pub size: Qty<Length>,
    pub limit: Qty<Length>,
    /// 余裕 = 許容 / 外形 − 1
    pub margin: Option<Fraction>,
    pub verdict: Verdict,
    /// 最小の端に届く部品（部品の箱から出したときだけ）
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    #[cfg_attr(feature = "ts", ts(optional, as = "Option<Vec<String>>"))]
    pub at_min: Vec<String>,
    /// 最大の端に届く部品（部品の箱から出したときだけ）
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    #[cfg_attr(feature = "ts", ts(optional, as = "Option<Vec<String>>"))]
    pub at_max: Vec<String>,
    /// 包絡の中心の機体原点からのずれ（符号つき。部品の箱から出したときだけ。情報で、判定には使わない）
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub centre_offset: Option<Qty<Length>>,
}

/// 外形の判定の結果。
pub(super) struct Judged {
    pub origin: String,
    pub source: LaunchFitEnvelopeSource,
    pub axes: Vec<LaunchFitAxis>,
    pub skipped: Vec<InterferenceSkipped>,
}

/// 1 軸の測定（部品の箱から出したもの）。
struct Measured {
    lo: f64,
    hi: f64,
    at_min: Vec<String>,
    at_max: Vec<String>,
}

/// 外形を判定する。許容の外形が無い、または測る外形が無ければ `None`（外形は判定しない）。
pub(super) fn judge(input: &LaunchFitInput) -> Result<Option<Judged>, AnalysisError> {
    let Some(envelope) = &input.envelope else {
        return Ok(None);
    };
    let limits = [envelope.size.x, envelope.size.y, envelope.size.z];
    let (source, sizes, measured, skipped) = match &input.part_boxes {
        Some(parts) => {
            let (m, skipped) = measure_parts(parts)?;
            let sizes = m.each_ref().map(|a| a.hi - a.lo);
            (LaunchFitEnvelopeSource::Parts, sizes, Some(m), skipped)
        }
        None => {
            let Some(size) = input.size else {
                return Ok(None);
            };
            let sizes = [size.x, size.y, size.z];
            (LaunchFitEnvelopeSource::RootSize, sizes, None, Vec::new())
        }
    };
    let mut axes = Vec::new();
    for (i, ((name, s), l)) in AXES.iter().zip(sizes).zip(limits).enumerate() {
        positive(&format!("Outer dimension {name}"), s)?;
        positive(&format!("Envelope {name}"), l)?;
        let margin = ratio_margin(l, s);
        let m = measured.as_ref().map(|m| &m[i]);
        axes.push(LaunchFitAxis {
            axis: (*name).to_owned(),
            size: Qty::from_raw(s),
            limit: Qty::from_raw(l),
            margin,
            verdict: ratio_margin_verdict(margin, FIT_MARGIN_WARNING_BAND),
            at_min: m.map(|m| m.at_min.clone()).unwrap_or_default(),
            at_max: m.map(|m| m.at_max.clone()).unwrap_or_default(),
            centre_offset: m.map(|m| Qty::from_raw((m.lo + m.hi) / 2.0)),
        });
    }
    Ok(Some(Judged {
        origin: envelope.origin.clone(),
        source,
        axes,
        skipped,
    }))
}

/// 部品の箱の包絡（軸ごとの最小・最大と、その端に届く部品）。測れない部品は一覧にして返す。
fn measure_parts(
    parts: &LaunchFitPartBoxes,
) -> Result<([Measured; 3], Vec<InterferenceSkipped>), AnalysisError> {
    let (judged, mut skipped) = usable_boxes(&parts.boxes, &parts.unsized_parts);
    skipped.sort_by(|a, b| a.id.cmp(&b.id));
    // 員数が 2 以上の部品は同じ位置・姿勢に重なるので、箱は 1 つ
    let placed: Vec<OrientedBox> = judged
        .iter()
        .map(|b| OrientedBox::from_size(b.position, b.orientation, b.size))
        .collect();
    let Some((lo, hi)) = axis_aligned_extents(&placed) else {
        return Err(AnalysisError::InvalidInput(
            "No part with a size to measure the stowed envelope".to_owned(),
        ));
    };
    let own: Vec<(Vec3, Vec3)> = placed
        .iter()
        .filter_map(|b| axis_aligned_extents(std::slice::from_ref(b)))
        .collect();
    let at = |axis: usize, pick_max: bool| -> Vec<String> {
        let target = if pick_max {
            component(hi, axis)
        } else {
            component(lo, axis)
        };
        judged
            .iter()
            .zip(&own)
            .filter(|(_, (l, h))| {
                let v = component(if pick_max { *h } else { *l }, axis);
                (v - target).abs() <= TIE
            })
            .map(|(b, _)| b.id.clone())
            .collect()
    };
    let measured = std::array::from_fn(|i| Measured {
        lo: component(lo, i),
        hi: component(hi, i),
        at_min: at(i, false),
        at_max: at(i, true),
    });
    Ok((measured, skipped))
}

/// ベクトルの軸番号（0 = x、1 = y、2 = z）の成分。
fn component(v: Vec3, axis: usize) -> f64 {
    [v.x, v.y, v.z][axis]
}

/// 端の部品の並び（`A` か `A, B`）。
fn ids_text(ids: &[String]) -> String {
    ids.join(", ")
}

/// 軸の判定の文。部品の箱から出したときは、両端の位置とその部品・中心のずれも書く。
pub(super) fn axis_check(a: &LaunchFitAxis, origin: &str) -> Check {
    let allowed = format!(
        "allowed {} from {origin}",
        quantity_text::<Length>(a.limit.value())
    );
    let message = match a.centre_offset {
        Some(centre) => {
            let half = a.size.value() / 2.0;
            let (lo, hi) = (centre.value() - half, centre.value() + half);
            format!(
                "{} (from {} at {} to {} at {}; centre {} from the body origin) ({allowed})",
                quantity_text::<Length>(a.size.value()),
                quantity_text::<Length>(lo),
                ids_text(&a.at_min),
                quantity_text::<Length>(hi),
                ids_text(&a.at_max),
                quantity_text::<Length>(centre.value()),
            )
        }
        None => format!("{} ({allowed})", quantity_text::<Length>(a.size.value())),
    };
    Check::new(
        &format!("envelope:{}", a.axis),
        &format!("Envelope {}", a.axis),
        a.verdict,
        message,
    )
}

#[cfg(test)]
mod tests;
