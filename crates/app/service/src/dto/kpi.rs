//! KPI（系の代表値）と塗りの比の型。どの値を代表にするか・判定・塗りの比・判定の数え方は
//! `fune_service::analysis::kpi` が決め、CLI と Web はここに入った値を並べるだけにする。

use super::QuantityView;
use sat_analysis::spec::Verdict;
use serde::{Deserialize, Serialize};
use ts_rs::TS;

/// KPI の名前のキー。キーはすべての系を通して一意で、どの解析が出すかは `analysis::kpi` の表が決める。
/// 表示名は `analysis::labels::kpi_label`（日英）。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, TS)]
#[serde(rename_all = "snake_case")]
pub enum KpiKey {
    /// 季節の最長蝕（本影と半影の合計）。
    LongestEclipse,
    /// 季節の β の最小値。
    BetaMin,
    /// 季節の β の最大値。
    BetaMax,
    /// バジェットのモード（ケースのモード、無ければ成長余裕込みの合計が最大のモード）の消費電力
    ModeLoad,
    /// 軌道平均の発電
    OrbitGeneration,
    /// 軌道平均の電力収支（ケースのうち最悪）
    OrbitBalance,
    /// 最大 DoD（ケースのうち最大）
    MaxDod,
    /// SAP 面積の余裕。
    SapMargin,
    /// 電池容量の余裕。
    BatteryMargin,
    /// RW のアンロード間隔
    UnloadInterval,
    /// RW の最大角運動量
    PeakMomentum,
    /// MTQ でのアンロード時間
    MtqUnloadTime,
    /// センサの禁止角への侵入の割合（センサのうち最大）
    SensorIntrusion,
    /// 姿勢変更の時間（最悪の軸）
    SlewTime,
    /// 1 日のデータ発生量
    DataPerDay,
    /// 必要なストレージ
    StorageRequired,
    /// ストレージの容量の余裕
    StorageMargin,
    /// 回線余裕（回線のうち最小）
    LinkMargin,
    /// 1 日の可視時間（すべての局）
    ContactPerDay,
    /// 1 日の送信可能量
    DownlinkPerDay,
    /// 最大の未送信データ
    MaxBacklog,
    /// 定常の最高温度（節点のうち最高）
    MaxTemperature,
    /// 定常の最低温度（節点のうち最低）
    MinTemperature,
    /// 温度範囲（保管・動作・性能）の判定（節点と種類ごとの判定を数える）
    NodesInRange,
    /// 非定常の最高温度（節点のうち最高）
    TransientMaxTemperature,
    /// 総質量
    TotalMass,
    /// 最小の安全余裕（取付のうち最小）
    MinSafetyMargin,
    /// 最低の固有振動数
    MinFrequency,
    /// マヌーバの遷移の時間（マヌーバのうち最長）
    LongestTransfer,
    /// マージン込みの ΔV
    DvWithMargin,
    /// 積む推薬（マージンと残る分込み）
    LoadedPropellant,
    /// タンクの容積の余裕（推薬のうち最小）
    TankVolumeMargin,
    /// 総インパルスの定格に対する余裕（スラスタのうち最小）
    ImpulseMargin,
}

/// KPI の定義（`GET /analyses` の `AnalysisEntry.kpis`。結果が届く前に名前を並べるのに使う）。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
pub struct KpiDef {
    pub key: KpiKey,
    /// 表示名
    pub label: String,
    /// 英語の表示名
    pub label_en: String,
}

/// KPI 1 つの値（`AnalysisRunResult.kpis`）。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
pub struct KpiView {
    pub key: KpiKey,
    /// 表示名
    pub label: String,
    /// 英語の表示名
    pub label_en: String,
    /// 代表値（出力に値が無ければ null。判定を数える KPI も null で、数は `tally`）
    pub value: Option<QuantityView>,
    /// 判定（判定の対象でない KPI は null）
    pub verdict: Option<Verdict>,
    /// 塗りの比（目盛りのバーに出すもの。比の無い KPI は null）
    pub fill: Option<FillRatio>,
    /// 判定を数えた結果（数える KPI だけ）
    pub tally: Option<KpiTally>,
}

/// 目盛りのバーの塗りの比（0〜1 に収めた値）と、その根拠。
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize, TS)]
pub struct FillRatio {
    /// 0〜1（範囲の外は端に寄せる）
    pub ratio: f64,
    pub basis: FillBasis,
}

/// 塗りの比の根拠（文言は CLI と Web が作る）。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "snake_case")]
pub enum FillBasis {
    /// 値そのものが比（DoD・余裕）
    Value,
    /// 上限に対する使用量（バジェットの合計 / 上限）
    UsedOverLimit,
    /// OK の判定の割合（`tally` の OK の数 / 判定の数）
    OkShare,
}

/// 判定の数え方: 数えた判定と、そのうち OK でないもの（注意・違反）の数。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
pub struct KpiTally {
    /// 数えた判定の数
    pub checked: u32,
    /// そのうち OK でない（注意か違反の）数
    pub flagged: u32,
}

impl FillRatio {
    /// 値そのものを比にする（0〜1 に収める）。値が有限でなければ None。
    pub fn of_value(value: f64, basis: FillBasis) -> Option<Self> {
        value.is_finite().then(|| Self {
            ratio: value.clamp(0.0, 1.0),
            basis,
        })
    }

    /// `part / whole` を比にする（0〜1 に収める）。`whole` が 0 以下か、比が有限でなければ None。
    pub fn of_share(part: f64, whole: f64, basis: FillBasis) -> Option<Self> {
        if whole > 0.0 {
            Self::of_value(part / whole, basis)
        } else {
            None
        }
    }

    /// バジェットの塗り（成長余裕込みの合計 / 上限）。上限が無ければ None。
    pub fn used_over_limit(used: f64, limit: Option<f64>) -> Option<Self> {
        Self::of_share(used, limit?, FillBasis::UsedOverLimit)
    }
}

impl KpiTally {
    /// OK の判定の割合（判定が無ければ None）。
    pub fn ok_share(self) -> Option<FillRatio> {
        FillRatio::of_share(
            f64::from(self.checked.saturating_sub(self.flagged)),
            f64::from(self.checked),
            FillBasis::OkShare,
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ratios_are_clamped_to_zero_and_one() {
        let over = FillRatio::used_over_limit(11.3, Some(3.0)).unwrap();
        assert_eq!(over.ratio, 1.0);
        assert_eq!(over.basis, FillBasis::UsedOverLimit);
        let half = FillRatio::used_over_limit(6.0, Some(12.0)).unwrap();
        assert!((half.ratio - 0.5).abs() < 1e-12);
        assert_eq!(
            FillRatio::of_value(-0.036, FillBasis::Value).unwrap().ratio,
            0.0
        );
        assert!(FillRatio::used_over_limit(1.0, None).is_none());
        assert!(FillRatio::used_over_limit(1.0, Some(0.0)).is_none());
        assert!(FillRatio::of_value(f64::NAN, FillBasis::Value).is_none());
    }

    #[test]
    fn tally_share_counts_ok_checks() {
        let t = KpiTally {
            checked: 9,
            flagged: 4,
        };
        assert!((t.ok_share().unwrap().ratio - 5.0 / 9.0).abs() < 1e-12);
        let none = KpiTally {
            checked: 0,
            flagged: 0,
        };
        assert!(none.ok_share().is_none());
    }
}
