//! 節点の温度と動作範囲の比較（`thermal.steady` と `thermal.transient` で共有）。**温度範囲の判定はここだけ。**

use crate::spec::quantity_text;
use crate::spec::{Check, Verdict, margin_verdict};
use crate::thermal::network::ThermalTempRange;

/// 動作範囲の端からこの幅 [K] 以内なら警告にする。出力の `range_warning_band` にも出す（Web はこれを使う）。
pub const RANGE_WARNING_BAND_K: f64 = 5.0;

/// 最低 `lo`・最高 `hi` [K] を動作範囲と比べる（端から [`RANGE_WARNING_BAND_K`] 以内は警告）。範囲が無ければ Ok。
pub(crate) fn range_verdict(lo: f64, hi: f64, range: Option<ThermalTempRange>) -> Verdict {
    let Some(r) = range else {
        return Verdict::Ok;
    };
    let margin = (lo - r.min.value()).min(r.max.value() - hi);
    margin_verdict(margin, RANGE_WARNING_BAND_K)
}

/// 動作範囲との比較の判定（範囲が無ければ `None`）。温度の表示は sat-units に委譲する。
pub(crate) fn range_check(
    node: &str,
    lo: f64,
    hi: f64,
    range: Option<ThermalTempRange>,
) -> Option<Check> {
    let r = range?;
    Some(Check::new(
        &format!("range:{node}"),
        &format!("{node} の温度"),
        range_verdict(lo, hi, range),
        format!(
            "{}〜{}（動作範囲 {}〜{}）",
            quantity_text(lo, "Temperature"),
            quantity_text(hi, "Temperature"),
            quantity_text(r.min.value(), "Temperature"),
            quantity_text(r.max.value(), "Temperature")
        ),
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    use sat_units::typed::Qty;

    /// 動作範囲 270〜320 K: 280〜300 K は端から 10 K 離れて Ok、316 K は上端まで 4 K（< 5 K）で警告、
    /// 321 K は超過。範囲が無ければ判定は Ok で、判定文は出さない。
    #[test]
    fn verdict_by_distance_to_the_ends() {
        let r = Some(ThermalTempRange {
            min: Qty::from_raw(270.0),
            max: Qty::from_raw(320.0),
        });
        assert_eq!(range_verdict(280.0, 300.0, r), Verdict::Ok);
        assert_eq!(range_verdict(280.0, 316.0, r), Verdict::Warning);
        assert_eq!(range_verdict(280.0, 321.0, r), Verdict::Violation);
        assert_eq!(range_verdict(268.0, 300.0, r), Verdict::Violation);
        assert_eq!(range_verdict(0.0, 1.0e4, None), Verdict::Ok);
        assert_eq!(
            range_check("box", 280.0, 316.0, r).unwrap().key,
            "range:box"
        );
        assert!(range_check("box", 280.0, 316.0, None).is_none());
    }
}
