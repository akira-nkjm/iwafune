//! ある距離での回線の評価（C/N0・Eb/N0・余裕・要求余裕を残して通せる最大のデータレート）。
//! **距離を決めて回線の式を評価するのはここだけ**（式そのものは [`crate::comm::link`]）。最大の距離（判定）と最小の距離
//! （最良の端。ほかの天体を回る衛星だけ）で同じ関数を使うので、データレートの距離による変化（距離の 2 乗に反比例）は式から出る。

use super::LinkBudgetLink;
use crate::comm::link::{LinkEvaluation, eb_over_n0_db, max_data_rate_bps};
use sat_units::typed::Decibel;

/// 距離 1 つでの回線の評価。dB の値は電力比の 10 log10。
#[derive(Debug, Clone, Copy)]
pub(super) struct AtRange {
    /// EIRP・損失・G/T・C/N0（[`crate::comm::link::RfLinkParams::evaluate`]）
    pub eval: LinkEvaluation,
    /// Eb/N0 [dB]
    pub eb_n0_db: f64,
    /// 回線余裕 = Eb/N0 − 所要 Eb/N0 [dB]
    pub margin_db: f64,
    /// 要求余裕を残して通せる最大のデータレート [bit/s]
    pub max_data_rate_bps: f64,
}

/// 距離 `range_m` [m] での `link` の評価（データレートは `link.data_rate`、要求余裕は `required_margin`）。
pub(super) fn at_range(link: &LinkBudgetLink, required_margin: Decibel, range_m: f64) -> AtRange {
    let eval = link.rf.evaluate(range_m);
    let eb_n0_db = eb_over_n0_db(eval.c_over_n0_dbhz, link.data_rate.value());
    AtRange {
        eval,
        eb_n0_db,
        margin_db: eb_n0_db - link.required_eb_n0_db.value(),
        max_data_rate_bps: max_data_rate_bps(
            eval.c_over_n0_dbhz,
            link.required_eb_n0_db.value(),
            required_margin.value(),
        ),
    }
}
