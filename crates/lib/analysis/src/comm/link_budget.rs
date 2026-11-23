//! 回線バジェット（`comm.link_budget`）。回線ごとに EIRP・自由空間損失・G/T・C/N0・Eb/N0 と余裕を求める。
//! 計算の部品は [`crate::comm::link`]、距離の求め方（地球を回る衛星の斜距離／他の天体を回る衛星の地球からの距離）は [`range`]。
//!
//! - 回線余裕は最大の距離（最低仰角の斜距離、または期間中の地球からの最大距離）で判定する。
//! - 下りは、最小の距離（天頂、または期間中の最小距離）での地表の電力束密度（PFD）も求める。限度値 `pfd_limit_db` が
//!   書かれているときだけ判定する（限度値は周波数帯・業務・到来角で規則が決めるので、解析は既定値を持たない）。

mod range;

pub use range::{DEFAULT_RANGE_SPAN, LinkDistanceModel};

use crate::comm::link::{
    DEFAULT_PFD_REFERENCE_BANDWIDTH, RfLinkParams, eb_over_n0_db, max_data_rate_bps, pfd_dbw_m2,
    required_margin,
};
use crate::spec::quantity_text;
use crate::spec::{
    Analysis, AnalysisArea, AnalysisError, AnalysisKind, Check, System, margin_verdict,
};
use range::{deep_space_range, link_range};
use sat_astro::ephem::CentralBody;
use sat_astro::ground::EarthRange;
use sat_astro::orbit::Trajectory;
use sat_astro::time::Epoch;
use sat_units::typed::{
    Angle, DataRate, Decibel, DecibelHertz, DecibelPerKelvin, DecibelWatt, Frequency, Length, Qty,
    Time,
};
use serde::{Deserialize, Serialize};

/// 回線の向き。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(rename_all = "snake_case")]
pub enum LinkBudgetDirection {
    /// 地上 → 衛星
    Uplink,
    /// 衛星 → 地上
    Downlink,
}

/// 電力束密度（PFD）の判定で、限度値に近い（余裕がこれ未満）ときに警告にする幅 [dB]。
pub const PFD_WARNING_BAND: Decibel = Decibel::new(1.0);

/// 1 回線の入力。
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct LinkBudgetLink {
    /// 回線の名前（例: `"UHF down"`）
    pub name: String,
    /// 相手の地上局の ID（回線 × 地上局の組ごとに 1 行）
    pub station: String,
    pub direction: LinkBudgetDirection,
    /// 送受信端と経路の RF パラメーター（tx は送信側、rx は受信側）
    pub rf: RfLinkParams,
    /// データレート
    pub data_rate: Qty<DataRate>,
    /// 変調・符号化で決まる所要 Eb/N0
    pub required_eb_n0_db: Decibel,
    /// 衛星の高度（中心天体の半径から）
    pub altitude: Qty<Length>,
    /// 軌道の中心天体（斜距離に半径を使う。省略時は地球）
    #[serde(default)]
    pub central: CentralBody,
    /// 評価する仰角（最悪は運用の最低仰角）
    pub elevation: Qty<Angle>,
    /// 要求する回線余裕（この回線 × 地上局。省略時は入力全体の [`LinkBudgetInput::required_margin_db`]）
    #[serde(default)]
    pub required_margin_db: Option<Decibel>,
    /// 地表の電力束密度（PFD）の限度 [dBW/m²（基準帯域あたり）]。下りで書かれたときだけ判定する（既定値は持たない）
    #[serde(default)]
    pub pfd_limit_db: Option<Decibel>,
    /// PFD の基準帯域（省略時 [`DEFAULT_PFD_REFERENCE_BANDWIDTH`]）
    #[serde(default)]
    pub pfd_reference_bandwidth: Option<Qty<Frequency>>,
    /// 信号の占有帯域（PFD を基準帯域あたりに直すのに使う。省略時は割らない＝安全側）
    #[serde(default)]
    pub occupied_bandwidth: Option<Qty<Frequency>>,
}

/// 回線バジェットの入力。
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct LinkBudgetInput {
    pub links: Vec<LinkBudgetLink>,
    /// 要求する回線余裕（回線ごとの値を持たない回線に使う。既定 [`crate::comm::link::DEFAULT_REQUIRED_MARGIN`]）
    #[serde(default)]
    pub required_margin_db: Option<Decibel>,
    /// 軌道（中心天体が地球でない回線があるとき、地球から衛星までの距離を求めるのに使う。開始は軌道の元期）
    #[serde(default)]
    pub orbit: Option<Trajectory>,
    /// 距離を評価する期間（ミッション期間。省略時 [`DEFAULT_RANGE_SPAN`]。中心天体が地球でない回線だけ使う）
    #[serde(default)]
    pub duration: Option<Qty<Time>>,
}

impl LinkBudgetInput {
    /// 要求する回線余裕（省略時 [`crate::comm::link::DEFAULT_REQUIRED_MARGIN`]）。
    pub fn required_margin_db(&self) -> Decibel {
        required_margin(self.required_margin_db)
    }
}

/// 1 回線の結果（dB の値はすべて電力比の 10 log10）。
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct LinkBudgetLinkResult {
    pub name: String,
    /// 相手の地上局の ID（入力の `station`）
    pub station: String,
    pub direction: LinkBudgetDirection,
    /// 回線余裕を判定した距離（最大の距離。地球を回る衛星は最低仰角の斜距離、ほかの天体を回る衛星は期間中の地球からの最大距離）
    pub slant_range: Qty<Length>,
    /// 距離の求め方
    pub distance_model: LinkDistanceModel,
    /// 最小の距離（地球を回る衛星は天頂の斜距離 = 高度、ほかの天体を回る衛星は期間中の最小距離）。PFD はこの距離で求める
    pub range_min: Qty<Length>,
    /// 最小の距離になった時刻（ほかの天体を回る衛星だけ）
    pub range_min_epoch: Option<Epoch>,
    /// 最大の距離になった時刻（ほかの天体を回る衛星だけ）
    pub range_max_epoch: Option<Epoch>,
    /// 最大の距離を光が進む時間（片道）
    pub light_time: Qty<Time>,
    /// EIRP [dBW]
    pub eirp_dbw: DecibelWatt,
    /// 自由空間損失 [dB]
    pub fspl_db: Decibel,
    /// 自由空間損失以外の経路の損失の合計 [dB]
    pub extra_loss_db: Decibel,
    /// 受信系の G/T [dB/K]
    pub g_over_t_dbk: DecibelPerKelvin,
    /// G/T が地上局などの与えられた値か（`false` なら利得・損失・雑音温度から求めた値）
    pub g_over_t_given: bool,
    /// C/N0 [dBHz]
    pub c_over_n0_dbhz: DecibelHertz,
    /// Eb/N0 [dB]
    pub eb_n0_db: Decibel,
    /// 所要 Eb/N0 [dB]
    pub required_eb_n0_db: Decibel,
    /// 回線余裕 = Eb/N0 − 所要 Eb/N0 [dB]
    pub margin_db: Decibel,
    /// 判定に使った要求余裕 [dB]（回線ごとの値、無ければ入力全体の値、それも無ければ既定値）
    pub required_margin_db: Decibel,
    /// 要求余裕を残して通せる最大のデータレート
    pub max_data_rate: Qty<DataRate>,
    /// 地表の電力束密度 [dBW/m²（基準帯域あたり）]（下りだけ。最小の距離・大気損失なしの最悪の値）
    pub pfd_dbw_m2: Option<Decibel>,
    /// 電力束密度の基準帯域（下りだけ）
    pub pfd_reference_bandwidth: Option<Qty<Frequency>>,
    /// 電力束密度の限度 [dBW/m²（基準帯域あたり）]（入力に書かれたときだけ）
    pub pfd_limit_db: Option<Decibel>,
    /// 電力束密度の余裕 = 限度 − PFD [dB]（限度が書かれたときだけ）
    pub pfd_margin_db: Option<Decibel>,
}

/// 回線バジェットの出力。
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct LinkBudgetOutput {
    pub links: Vec<LinkBudgetLinkResult>,
    /// 入力全体の要求余裕 [dB]（回線ごとの値を持たない回線に使った値。入力が無ければ既定値）。回線ごとに使った値は各回線の結果の `required_margin_db`
    pub required_margin_db: Decibel,
    /// 回線 × 地上局の組ごとの余裕の判定（key は `margin:<name>@<局>`）と、限度が書かれた下りの PFD の判定（`pfd:<name>@<局>`）
    pub checks: Vec<Check>,
}

/// 回線バジェット。
pub struct LinkBudget;

impl Analysis for LinkBudget {
    const ID: &'static str = "comm.link_budget";
    const TITLE: &'static str = "回線バジェット";
    const AREA: AnalysisArea = AnalysisArea::System(System::Comm);
    const KIND: AnalysisKind = AnalysisKind::Steady;
    const DESCRIPTION: &'static str = "アップリンク・ダウンリンクの回線マージン";
    const TITLE_EN: &'static str = "Link budget";
    const DESCRIPTION_EN: &'static str = "Uplink and downlink margins";
    type Input = LinkBudgetInput;
    type Output = LinkBudgetOutput;

    fn run(input: &Self::Input) -> Result<Self::Output, AnalysisError> {
        if input.links.is_empty() {
            return Err(AnalysisError::InvalidInput("No links".to_owned()));
        }
        let mut links = Vec::with_capacity(input.links.len());
        let mut checks = Vec::with_capacity(input.links.len());
        let required = input.required_margin_db();
        let deep = deep_space_range(&input.links, input.orbit.as_ref(), input.duration)?;
        for link in &input.links {
            let required = link.required_margin_db.unwrap_or(required);
            let result = evaluate(link, required, deep.as_ref())?;
            checks.push(margin_check(&result, required));
            checks.extend(pfd_check(&result));
            links.push(result);
        }
        Ok(LinkBudgetOutput {
            links,
            required_margin_db: required,
            checks,
        })
    }
}

fn evaluate(
    link: &LinkBudgetLink,
    required_margin: Decibel,
    deep: Option<&EarthRange>,
) -> Result<LinkBudgetLinkResult, AnalysisError> {
    link.rf.validate(&link.name)?;
    let rate = link.data_rate.value();
    let altitude = link.altitude.value();
    if !(rate > 0.0 && altitude > 0.0) {
        return Err(AnalysisError::InvalidInput(format!(
            "{}: data rate and altitude must be positive",
            link.name
        )));
    }
    let range = link_range(link, deep)?;
    let eval = link.rf.evaluate(range.max_m);
    let eb_n0 = eb_over_n0_db(eval.c_over_n0_dbhz, rate);
    let pfd = pfd_of(link, eval.eirp_dbw, &range);
    Ok(LinkBudgetLinkResult {
        name: link.name.clone(),
        station: link.station.clone(),
        direction: link.direction,
        slant_range: range.max(),
        distance_model: range.model,
        range_min: range.min(),
        range_min_epoch: range.min_epoch,
        range_max_epoch: range.max_epoch,
        light_time: range.light_time(),
        eirp_dbw: (eval.eirp_dbw).into(),
        fspl_db: (eval.fspl_db).into(),
        extra_loss_db: (eval.extra_loss_db).into(),
        g_over_t_dbk: (eval.g_over_t_dbk).into(),
        g_over_t_given: eval.g_over_t_given,
        c_over_n0_dbhz: (eval.c_over_n0_dbhz).into(),
        eb_n0_db: (eb_n0).into(),
        required_eb_n0_db: link.required_eb_n0_db,
        margin_db: (eb_n0 - link.required_eb_n0_db.value()).into(),
        required_margin_db: required_margin,
        max_data_rate: Qty::from_raw(max_data_rate_bps(
            eval.c_over_n0_dbhz,
            link.required_eb_n0_db.value(),
            required_margin.value(),
        )),
        pfd_dbw_m2: pfd.as_ref().map(|p| p.pfd),
        pfd_reference_bandwidth: pfd.as_ref().map(|p| p.reference),
        pfd_limit_db: pfd.as_ref().and_then(|p| p.limit),
        pfd_margin_db: pfd
            .as_ref()
            .and_then(|p| p.limit.map(|l| (l.value() - p.pfd.value()).into())),
    })
}

/// 下りの地表の電力束密度（最小の距離で）。
struct Pfd {
    pfd: Decibel,
    reference: Qty<Frequency>,
    limit: Option<Decibel>,
}

/// 下りの PFD を求める（上りは地表に電力を出さないので `None`）。
fn pfd_of(link: &LinkBudgetLink, eirp_dbw: f64, range: &range::LinkRange) -> Option<Pfd> {
    if link.direction != LinkBudgetDirection::Downlink {
        return None;
    }
    let reference = link
        .pfd_reference_bandwidth
        .unwrap_or(DEFAULT_PFD_REFERENCE_BANDWIDTH);
    Some(Pfd {
        pfd: pfd_dbw_m2(
            eirp_dbw,
            range.min_m,
            link.occupied_bandwidth.map(|b| b.value()),
            reference.value(),
        )
        .into(),
        reference,
        limit: link.pfd_limit_db,
    })
}

/// PFD の判定（限度が書かれた下りだけ。key は `pfd:<name>@<局>`）。余裕 = 限度 − PFD が負なら違反、[`PFD_WARNING_BAND`] 未満なら警告。
fn pfd_check(result: &LinkBudgetLinkResult) -> Option<Check> {
    let (pfd, limit, margin) = (
        result.pfd_dbw_m2?,
        result.pfd_limit_db?,
        result.pfd_margin_db?,
    );
    let pair = format!("{}@{}", result.name, result.station);
    Some(Check::new(
        &sat_checkkey::link_pfd(&result.name, &result.station),
        &format!("{pair} power flux density"),
        margin_verdict(margin.value(), PFD_WARNING_BAND.value()),
        format!(
            "PFD {} (limit {}, margin {})",
            quantity_text::<Decibel>(pfd.value()),
            quantity_text::<Decibel>(limit.value()),
            quantity_text::<Decibel>(margin.value())
        ),
    ))
}

fn margin_check(result: &LinkBudgetLinkResult, required: Decibel) -> Check {
    let pair = format!("{}@{}", result.name, result.station);
    Check::new(
        &sat_checkkey::link_margin(&result.name, &result.station),
        &format!("{pair} link margin"),
        margin_verdict(result.margin_db.value(), required.value()),
        format!(
            "Margin {} (required at least {})",
            quantity_text::<Decibel>(result.margin_db.value()),
            quantity_text::<Decibel>(required.value())
        ),
    )
}

#[cfg(test)]
mod tests;
