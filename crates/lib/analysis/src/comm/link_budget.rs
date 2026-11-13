//! 回線バジェット（`comm.link_budget`）。回線ごとに EIRP・自由空間損失・G/T・C/N0・Eb/N0 と余裕を求める。
//! 計算の部品は [`crate::comm::link`] を使う。

use crate::comm::link::{RfLinkParams, eb_over_n0_db, max_data_rate_bps, required_margin};
use crate::spec::quantity_text;
use crate::spec::{
    Analysis, AnalysisArea, AnalysisError, AnalysisKind, Check, System, margin_verdict,
};
use sat_astro::ephem::CentralBody;
use sat_astro::ground::slant_range_m;
use sat_units::typed::{
    Angle, DataRate, Decibel, DecibelHertz, DecibelPerKelvin, DecibelWatt, Length, Qty,
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

/// 1 回線の入力。
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct LinkBudgetLink {
    /// 回線の名前（例: `"UHF down"`）
    pub name: String,
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
}

/// 回線バジェットの入力。
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct LinkBudgetInput {
    pub links: Vec<LinkBudgetLink>,
    /// 要求する回線余裕（既定 [`crate::comm::link::DEFAULT_REQUIRED_MARGIN`]）
    #[serde(default)]
    pub required_margin_db: Option<Decibel>,
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
    pub direction: LinkBudgetDirection,
    /// 斜距離
    pub slant_range: Qty<Length>,
    /// EIRP [dBW]
    pub eirp_dbw: DecibelWatt,
    /// 自由空間損失 [dB]
    pub fspl_db: Decibel,
    /// 自由空間損失以外の経路の損失の合計 [dB]
    pub extra_loss_db: Decibel,
    /// 受信系の G/T [dB/K]
    pub g_over_t_dbk: DecibelPerKelvin,
    /// C/N0 [dBHz]
    pub c_over_n0_dbhz: DecibelHertz,
    /// Eb/N0 [dB]
    pub eb_n0_db: Decibel,
    /// 所要 Eb/N0 [dB]
    pub required_eb_n0_db: Decibel,
    /// 回線余裕 = Eb/N0 − 所要 Eb/N0 [dB]
    pub margin_db: Decibel,
    /// 要求余裕を残して通せる最大のデータレート
    pub max_data_rate: Qty<DataRate>,
}

/// 回線バジェットの出力。
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct LinkBudgetOutput {
    pub links: Vec<LinkBudgetLinkResult>,
    /// 判定に使った要求余裕 [dB]（入力が無ければ既定値）
    pub required_margin_db: Decibel,
    /// 回線ごとの余裕の判定（key は `margin:<name>`）
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
            return Err(AnalysisError::InvalidInput("回線がありません".to_owned()));
        }
        let mut links = Vec::with_capacity(input.links.len());
        let mut checks = Vec::with_capacity(input.links.len());
        let required = input.required_margin_db();
        for link in &input.links {
            let result = evaluate(link, required)?;
            checks.push(margin_check(&result, required));
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
) -> Result<LinkBudgetLinkResult, AnalysisError> {
    link.rf.validate(&link.name)?;
    let rate = link.data_rate.value();
    let altitude = link.altitude.value();
    if !(rate > 0.0 && altitude > 0.0) {
        return Err(AnalysisError::InvalidInput(format!(
            "{}: データレートと高度は正の値にしてください",
            link.name
        )));
    }
    let range = slant_range_m(link.central.radius_m(), altitude, link.elevation.value());
    let eval = link.rf.evaluate(range);
    let eb_n0 = eb_over_n0_db(eval.c_over_n0_dbhz, rate);
    Ok(LinkBudgetLinkResult {
        name: link.name.clone(),
        direction: link.direction,
        slant_range: Qty::from_raw(range),
        eirp_dbw: (eval.eirp_dbw).into(),
        fspl_db: (eval.fspl_db).into(),
        extra_loss_db: (eval.extra_loss_db).into(),
        g_over_t_dbk: (eval.g_over_t_dbk).into(),
        c_over_n0_dbhz: (eval.c_over_n0_dbhz).into(),
        eb_n0_db: (eb_n0).into(),
        required_eb_n0_db: link.required_eb_n0_db,
        margin_db: (eb_n0 - link.required_eb_n0_db.value()).into(),
        max_data_rate: Qty::from_raw(max_data_rate_bps(
            eval.c_over_n0_dbhz,
            link.required_eb_n0_db.value(),
            required_margin.value(),
        )),
    })
}

fn margin_check(result: &LinkBudgetLinkResult, required: Decibel) -> Check {
    Check::new(
        &format!("margin:{}", result.name),
        &format!("{} の回線余裕", result.name),
        margin_verdict(result.margin_db.value(), required.value()),
        format!(
            "余裕 {}（要求 {} 以上）",
            quantity_text::<Decibel>(result.margin_db.value()),
            quantity_text::<Decibel>(required.value())
        ),
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::spec::Verdict;

    fn db(v: f64) -> Decibel {
        Decibel::new(v)
    }

    fn uhf_down() -> LinkBudgetLink {
        LinkBudgetLink {
            name: "UHF down".to_owned(),
            direction: LinkBudgetDirection::Downlink,
            rf: RfLinkParams {
                frequency: Qty::from_raw(437.0e6),
                tx_power: Qty::from_raw(1.0),
                tx_gain_db: db(0.0),
                tx_loss_db: Some(db(1.0)),
                rx_gain_db: db(12.0),
                rx_loss_db: Some(db(0.0)),
                system_noise_temperature: Qty::from_raw(500.0),
                atmospheric_loss_db: Some(db(0.5)),
                polarization_loss_db: Some(db(3.0)),
                pointing_loss_db: Some(db(0.5)),
                other_loss_db: Some(db(0.0)),
            },
            data_rate: Qty::from_raw(9600.0),
            required_eb_n0_db: db(10.0),
            altitude: Qty::from_raw(1.0e6),
            central: CentralBody::Earth,
            elevation: Qty::from_raw(std::f64::consts::FRAC_PI_2),
        }
    }

    /// 手計算: EIRP = 0 + 0 − 1 = −1 dBW、FSPL = 145.25 dB、その他 4 dB、G/T = 12 − 26.99 = −14.99 dB/K、
    /// C/N0 = −1 − 149.25 − 14.99 + 228.60 = 63.36 dBHz、Eb/N0 = 63.36 − 39.82 = 23.54 dB、余裕 13.54 dB。
    #[test]
    fn hand_calculated_downlink() {
        let out = LinkBudget::run(&LinkBudgetInput {
            links: vec![uhf_down()],
            required_margin_db: None,
        })
        .unwrap();
        let r = &out.links[0];
        assert!((r.slant_range.value() - 1.0e6).abs() < 1e-3);
        assert!((r.eirp_dbw.value() + 1.0).abs() < 1e-9);
        assert!((r.fspl_db.value() - 145.25).abs() < 0.02);
        assert!((r.g_over_t_dbk.value() + 14.99).abs() < 0.01);
        assert!(
            (r.c_over_n0_dbhz.value() - 63.36).abs() < 0.03,
            "{}",
            r.c_over_n0_dbhz.value()
        );
        assert!(
            (r.eb_n0_db.value() - 23.54).abs() < 0.03,
            "{}",
            r.eb_n0_db.value()
        );
        assert!((r.margin_db.value() - 13.54).abs() < 0.03);
        assert_eq!(out.checks[0].verdict, Verdict::Ok);
        // 最大レート: 10^((63.36 − 10 − 3)/10) ≈ 108 kbit/s
        let max = r.max_data_rate.value();
        assert!((max / 1.08e5 - 1.0).abs() < 0.01, "{max}");
    }

    #[test]
    fn low_margin_is_flagged() {
        let mut link = uhf_down();
        link.rf.tx_power = Qty::from_raw(0.01); // −20 dB
        link.elevation = Qty::from_raw(0.0);
        let out = LinkBudget::run(&LinkBudgetInput {
            links: vec![link],
            required_margin_db: None,
        })
        .unwrap();
        assert!(out.links[0].margin_db.value() < 0.0);
        assert_eq!(out.checks[0].verdict, Verdict::Violation);
    }

    /// 地平線の斜距離は中心天体の半径で決まる: 月（R = 1737.4 km）の高度 1000 km は
    /// √(2737.4² − 1737.4²) km ≈ 2115.5 km（地球なら √(7378.1² − 6378.1²) km ≈ 3709.0 km）。
    #[test]
    fn horizon_distance_follows_the_central_body() {
        let range = |central: CentralBody| {
            let mut link = uhf_down();
            link.elevation = Qty::from_raw(0.0);
            link.central = central;
            let input = LinkBudgetInput {
                links: vec![link],
                required_margin_db: None,
            };
            LinkBudget::run(&input).unwrap().links[0]
                .slant_range
                .value()
        };
        assert!((range(CentralBody::Moon) - 2115.5e3).abs() < 0.5e3);
        assert!((range(CentralBody::Earth) - 3709.0e3).abs() < 0.5e3);
    }
}
