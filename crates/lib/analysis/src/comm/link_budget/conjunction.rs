//! 太陽との合（太陽–地球–探査機の角 SEP が小さい期間）。ほかの天体を回る衛星の回線だけが持つ。
//! SEP の計算は `sat_astro::timeline::geometry`（`ground::earth_range` が標本にして最小と下回り時間を求める）。
//! ここは出力の形・しきい値の既定・判定だけ。雑音温度の上昇（太陽の雑音が受信に入る）は扱わない。

use crate::spec::{Check, Verdict, quantity_text, ratio_margin};
use sat_astro::ground::EarthRange;
use sat_astro::time::Epoch;
use sat_units::typed::{Angle, Qty, Time};
use serde::{Deserialize, Serialize};

/// 合とみなす SEP のしきい値の既定 3°。**既定値はここだけ。**
/// 太陽が受信ビームの近くに入り、太陽の雑音と電波の屈折で回線が劣化し始める目安として通例使われる角で、
/// 実際のしきい値は周波数帯とアンテナのビーム幅で決まる（ここでは判定の目安であって、劣化の量は求めない）。
pub const DEFAULT_CONJUNCTION_ANGLE: Qty<Angle> = Qty::from_raw(3.0 * std::f64::consts::PI / 180.0);

/// 判定のキー。名前を含まないので `sat_checkkey` の表には載らない。
pub(super) const CHECK_KEY: &str = "solar_conjunction";

/// 距離を評価した期間の中の、太陽との合の様子。
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct SolarConjunction {
    /// 合とみなす SEP のしきい値（[`DEFAULT_CONJUNCTION_ANGLE`]）
    pub threshold: Qty<Angle>,
    /// 期間中の SEP の最小（太陽–地球–探査機の角。合に最も近い値）
    pub sep_min: Qty<Angle>,
    /// SEP が最小になった時刻
    pub sep_min_epoch: Epoch,
    /// SEP がしきい値を下回っていた時間の合計（標本の間は直線で結んだ値）
    pub time_below_threshold: Qty<Time>,
}

impl SolarConjunction {
    /// 距離の範囲（`earth_range` の結果）から作る。
    pub(super) fn of(range: &EarthRange) -> Self {
        Self {
            threshold: DEFAULT_CONJUNCTION_ANGLE,
            sep_min: Qty::from_raw(range.sep_min_rad),
            sep_min_epoch: range.sep_min_epoch,
            time_below_threshold: Qty::from_raw(range.sep_below_s),
        }
    }

    /// 判定: SEP がしきい値を下回る時間があれば警告（回線は期間を通して要るとみなす）。
    pub(super) fn check(&self) -> Check {
        let below = self.sep_min.value() < self.threshold.value();
        let verdict = if below { Verdict::Warning } else { Verdict::Ok };
        let message = if below {
            format!(
                "SEP falls to {} on {}, below {} for {}",
                quantity_text::<Angle>(self.sep_min.value()),
                self.sep_min_epoch.to_iso(),
                quantity_text::<Angle>(self.threshold.value()),
                quantity_text::<Time>(self.time_below_threshold.value())
            )
        } else {
            format!(
                "Minimum SEP {} on {} (threshold {})",
                quantity_text::<Angle>(self.sep_min.value()),
                self.sep_min_epoch.to_iso(),
                quantity_text::<Angle>(self.threshold.value())
            )
        };
        // 余裕 = 最小の SEP / しきい値 − 1
        Check::new(CHECK_KEY, "Solar conjunction", verdict, message)
            .with_ratio_margin(ratio_margin(self.sep_min.value(), self.threshold.value()))
    }
}
