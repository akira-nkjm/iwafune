//! 解析の共通の型。**解析の trait・エラー・時系列・区間・判定の型はここだけで定義する。**
//!
//! 時系列の集計の部品（日照の判定・周回の切り出しと平均・時間の割合・最長の区間・期間不足の扱い）は子の `orbit` に置き、ここで再公開する。
//! **解析は表示名を持たない**: 時系列は種類（[`ChannelKind`]）と対象の ID、列挙は値の綴りだけを返し、日本語・英語の表示名は service が引く。

use sat_units::typed::{Fraction, NamedKind};
use serde::{Deserialize, Serialize, de::DeserializeOwned};

/// 解析が属する系。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(rename_all = "snake_case")]
pub enum System {
    Power,
    Attitude,
    Cdh,
    Comm,
    Thermal,
    Structure,
}

/// 定常か時系列か。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(rename_all = "snake_case")]
pub enum AnalysisKind {
    Steady,
    TimeSeries,
}

/// 解析の説明（一覧の表示と `describe` に使う）。
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct AnalysisInfo {
    /// 例: `"power.energy_balance"`
    pub id: String,
    pub title: String,
    /// 英語の題名（GUI の英語表示）
    pub title_en: String,
    pub system: System,
    pub kind: AnalysisKind,
    pub description: String,
    /// 英語の説明（GUI の英語表示）
    pub description_en: String,
}

/// すべての解析が実装する trait。
pub trait Analysis {
    /// 例: `"power.energy_balance"`（`<系>.<名前>`）
    const ID: &'static str;
    const TITLE: &'static str;
    const SYSTEM: System;
    const KIND: AnalysisKind;
    const DESCRIPTION: &'static str;
    /// 英語の題名と説明（GUI の英語表示）
    const TITLE_EN: &'static str;
    const DESCRIPTION_EN: &'static str;

    type Input: Serialize + DeserializeOwned;
    type Output: Serialize;

    /// 解析を実行する。純粋な関数として振る舞う（I/O しない、乱数を使わない）。
    fn run(input: &Self::Input) -> Result<Self::Output, AnalysisError>;

    fn info() -> AnalysisInfo {
        AnalysisInfo {
            id: Self::ID.to_owned(),
            title: Self::TITLE.to_owned(),
            title_en: Self::TITLE_EN.to_owned(),
            system: Self::SYSTEM,
            kind: Self::KIND,
            description: Self::DESCRIPTION.to_owned(),
            description_en: Self::DESCRIPTION_EN.to_owned(),
        }
    }
}

/// 解析のエラー。
#[derive(Debug, thiserror::Error)]
pub enum AnalysisError {
    #[error("入力が不正です: {0}")]
    InvalidInput(String),
    #[error("未知の解析です: {0}")]
    Unknown(String),
    #[error("入力の JSON を解釈できません: {0}")]
    Json(String),
    #[error(transparent)]
    Astro(#[from] sat_astro::AstroError),
}

/// 時系列。値は SI で持ち、表示情報は service が付加する。
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct Series {
    /// 元期からの経過 [s]
    pub t: Vec<f64>,
    pub channels: Vec<Channel>,
}

/// 時系列の 1 系列。表示名は持たない（service が [`ChannelKind`] と `subject` から語彙で引く）。
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct Channel {
    /// 機械向けの名前（系列の中で一意。例: `"generation"`、`"wheel:RW_X"`）
    pub key: String,
    /// 何の系列か（表示名の語彙のキー）
    pub kind: ChannelKind,
    /// 系列の対象の ID（部品・センサ・節点など。種類だけで決まる系列は `None`）
    pub subject: Option<String>,
    /// 次元の名前（`sat_units::named_dimensions` の名前。例: `"Power"`）。[`Channel::of`] が型から取る
    pub dimension: String,
    /// SI 値
    pub values: Vec<f64>,
}

/// 時系列の系列の種類。**表示名は持たない**（service の語彙が日本語と英語で引く）。解析ごとの系列もここに並べる。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(rename_all = "snake_case")]
pub enum ChannelKind {
    /// 発電電力（パネルの合計）
    Generation,
    /// 日照率
    Sunlit,
    /// パネル 1 枚の発電（`subject` はパネル）
    PanelGeneration,
    /// 消費電力
    Load,
    /// 電池の SOC
    Soc,
    /// 充電電力
    ChargePower,
    /// 放電電力
    DischargePower,
    /// 余剰電力（シャント）
    ShuntPower,
    /// 重力傾斜トルク
    GravityGradientTorque,
    /// 空力トルク
    AeroTorque,
    /// 太陽輻射圧トルク
    SrpTorque,
    /// 残留磁気トルク
    MagneticTorque,
    /// 擾乱トルクの合計
    TotalDisturbanceTorque,
    /// 機体の角運動量の x 成分（機体座標系）
    MomentumX,
    /// 機体の角運動量の y 成分
    MomentumY,
    /// 機体の角運動量の z 成分
    MomentumZ,
    /// 機体の角運動量の大きさ
    MomentumNorm,
    /// ホイール 1 台の角運動量（`subject` はホイール）
    WheelMomentum,
    /// センサの視線と太陽のなす角（`subject` はセンサ）
    SunAngle,
    /// センサの視線と地球の縁のなす角
    EarthLimbAngle,
    /// センサの視線と月のなす角
    MoonAngle,
    /// 節点の温度（`subject` は節点）
    NodeTemperature,
    /// 外部入熱の合計
    AbsorbedHeat,
    /// 未送信データ
    Backlog,
}

impl Channel {
    /// 系列を作る。次元の名前は型 `K` から取る（`Channel::of::<Power>(…)`。力のモーメントは `TorqueKind`）。
    pub fn of<K: NamedKind>(key: impl Into<String>, kind: ChannelKind, values: Vec<f64>) -> Self {
        Self {
            key: key.into(),
            kind,
            subject: None,
            dimension: K::name().to_owned(),
            values,
        }
    }

    /// 系列の対象の ID を添える。
    #[must_use]
    pub fn about(mut self, subject: &str) -> Self {
        self.subject = Some(subject.to_owned());
        self
    }
}

/// 判定文の量を文字列にする。名前は型 `K` から取り（`quantity_text::<Power>(x)`）、換算と桁の整形は sat-units に委譲する。
/// 整形できない名前なら単位を付けず、sat-units の有効数字で数だけを返す。
pub fn quantity_text<K: NamedKind>(si: f64) -> String {
    use sat_units::format::{SIGNIFICANT_DIGITS, format_number, format_quantity};
    format_quantity(si, K::name())
        .map(|q| q.text)
        .unwrap_or_else(|_| format_number(si, SIGNIFICANT_DIGITS))
}

/// 判定の結果。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(rename_all = "snake_case")]
pub enum Verdict {
    Ok,
    Warning,
    Violation,
}

/// 要求や制約との比較（例: DoD が上限を超えていないか）。
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct Check {
    pub key: String,
    pub label: String,
    pub verdict: Verdict,
    pub message: String,
}

impl Verdict {
    /// 最も悪い判定。判定が無ければ None。
    pub fn worst(verdicts: impl IntoIterator<Item = Self>) -> Option<Self> {
        verdicts.into_iter().max_by_key(|v| match v {
            Self::Ok => 0,
            Self::Warning => 1,
            Self::Violation => 2,
        })
    }
}

impl Check {
    pub fn new(key: &str, label: &str, verdict: Verdict, message: impl Into<String>) -> Self {
        Self {
            key: key.to_owned(),
            label: label.to_owned(),
            verdict,
            message: message.into(),
        }
    }
}

impl Verdict {
    /// 満たしていれば `Ok`、満たしていなければ `Violation`（二段階の判定）。
    pub fn from_ok(ok: bool) -> Self {
        if ok { Self::Ok } else { Self::Violation }
    }
}

/// 余裕の三段階の判定。`margin < 0` なら `Violation`、`0 ≤ margin < warn_threshold` なら `Warning`、それ以上は `Ok`。
/// `margin` と `warn_threshold`（警告の幅）は同じ尺度（比の余裕 [`ratio_margin`] でも、温度差などの SI の量でもよい）。
/// **余裕の判定はこれを使う**（二段階にしたいときは警告の幅を 0 にする）。
pub fn margin_verdict(margin: f64, warn_threshold: f64) -> Verdict {
    if margin.is_nan() || margin < 0.0 {
        Verdict::Violation
    } else if margin < warn_threshold {
        Verdict::Warning
    } else {
        Verdict::Ok
    }
}

/// 比の余裕 `能力 / 要求 − 1`（0 以上で足りる。例: 0.25 = 要求の 1.25 倍）。**余裕の定義はこれだけ**
/// （構造の MS・固有振動数・RW のトルク・ストレージ・送信量で共有する）。
/// 要求が 0 のとき（余裕が無限大）や値が数でないときは `None`（JSON で非有限の数を出さない）。
pub fn ratio_margin(capability: f64, demand: f64) -> Option<Fraction> {
    let m = capability / demand - 1.0;
    m.is_finite().then_some(Fraction::new(m))
}

/// 比の余裕 [`ratio_margin`] の判定（[`margin_verdict`]、警告の幅 `warning_band`）。余裕が無限大（`None`）なら `Ok`。
pub fn ratio_margin_verdict(margin: Option<Fraction>, warning_band: Fraction) -> Verdict {
    margin.map_or(Verdict::Ok, |m| {
        margin_verdict(m.value(), warning_band.value())
    })
}

/// 員数（入力の `count`）の既定値 1。**`count` の serde の既定はこれだけを使う**（`#[serde(default = "crate::spec::default_count")]`）。
pub fn default_count() -> u32 {
    1
}

/// `v` が 0〜1 の有限値か調べる（割合・効率・比）。違えば `name` を含む `InvalidInput`。
pub fn ensure_fraction(name: &str, v: f64) -> Result<(), AnalysisError> {
    if v.is_finite() && (0.0..=1.0).contains(&v) {
        Ok(())
    } else {
        Err(AnalysisError::InvalidInput(format!(
            "{name} は 0〜1 にしてください（{v}）"
        )))
    }
}

/// `v` が正の有限値か調べる（容量・電圧・推力など）。違えば `name` を含む `InvalidInput`。
pub fn ensure_positive(name: &str, v: f64) -> Result<(), AnalysisError> {
    if v.is_finite() && v > 0.0 {
        Ok(())
    } else {
        Err(AnalysisError::InvalidInput(format!(
            "{name} は正の値にしてください（{v}）"
        )))
    }
}

/// `v` が 0 以上の有限値か調べる（面積・成長余裕・倍率など）。違えば `name` を含む `InvalidInput`。
pub fn ensure_non_negative(name: &str, v: f64) -> Result<(), AnalysisError> {
    if v.is_finite() && v >= 0.0 {
        Ok(())
    } else {
        Err(AnalysisError::InvalidInput(format!(
            "{name} は 0 以上にしてください（{v}）"
        )))
    }
}

mod orbit;
pub use orbit::{
    SUNLIT_THRESHOLD, SpanFallback, is_sunlit, longest_window, masked_time_mean, orbit_mean,
    orbit_ranges, orbit_segment, orbit_segments, span_fallback, time_fraction,
};

/// 時系列の中の区間と、その抽出。実体は sat-astro（下の層）にあり、ここでは再公開するだけ。
pub use sat_astro::timeline::{Window, windows};

/// 標本の間を線形補間して状態を積分する 1 刻み。実体は `sat_astro::ode`（積分器はそこだけ）にあり、ここでは再公開する。
pub use sat_astro::ode::rk4_linear_step;

#[cfg(test)]
mod tests {
    use super::*;
    use sat_units::typed::{Decibel, Energy, Fraction, Length, TorqueKind};

    #[test]
    fn quantity_text_takes_the_name_from_the_type() {
        assert_eq!(quantity_text::<Fraction>(0.125), "12.5 %");
        assert_eq!(quantity_text::<Energy>(3600.0), "1.00 Wh");
        assert_eq!(quantity_text::<Length>(0.012), "12.0 mm");
        // 力のモーメントは TorqueKind（次元の型 Torque は Energy と同じ型）
        assert_eq!(quantity_text::<TorqueKind>(0.002), "2.00 mN·m");
        assert_eq!(quantity_text::<Decibel>(13.54), "13.5 dB");
    }

    #[test]
    fn channels_take_the_dimension_from_the_type() {
        let c = Channel::of::<sat_units::typed::Power>("p", ChannelKind::Load, vec![1.0]);
        assert_eq!((c.dimension.as_str(), c.subject), ("Power", None));
        let w = Channel::of::<TorqueKind>("t", ChannelKind::AeroTorque, vec![]).about("RW");
        assert_eq!(
            (w.dimension.as_str(), w.subject.as_deref()),
            ("Torque", Some("RW"))
        );
    }

    #[test]
    fn verdict_helpers() {
        assert_eq!(Verdict::worst([]), None);
        assert_eq!(
            Verdict::worst([Verdict::Warning, Verdict::Ok]),
            Some(Verdict::Warning)
        );
        assert_eq!(
            Verdict::worst([Verdict::Violation, Verdict::Warning]),
            Some(Verdict::Violation)
        );
        assert_eq!(Verdict::from_ok(true), Verdict::Ok);
        assert_eq!(Verdict::from_ok(false), Verdict::Violation);
        assert_eq!(margin_verdict(-0.01, 0.2), Verdict::Violation);
        assert_eq!(margin_verdict(0.1, 0.2), Verdict::Warning);
        assert_eq!(margin_verdict(0.2, 0.2), Verdict::Ok);
        assert_eq!(margin_verdict(f64::NAN, 0.2), Verdict::Violation);
        assert!(ensure_fraction("効率", 0.5).is_ok());
        assert!(ensure_fraction("効率", 1.5).is_err());
        assert!(ensure_positive("容量", 0.0).is_err());
        assert!(ensure_positive("容量", f64::INFINITY).is_err());
    }

    /// 比の余裕 = 能力 / 要求 − 1: 200 N を 1.25 × 68.65 N で受けると 200 / 85.81 − 1 = 1.331。
    /// 要求が 0 なら無限大なので None（判定は ok）。
    #[test]
    fn ratio_margin_by_hand() {
        let m = ratio_margin(200.0, 1.25 * 68.65).unwrap();
        assert!((m.value() - (200.0 / 85.8125 - 1.0)).abs() < 1e-12);
        assert!((ratio_margin(1.0, 0.8).unwrap().value() - 0.25).abs() < 1e-12);
        assert_eq!(ratio_margin(1.0, 0.0), None);
        assert_eq!(ratio_margin(0.0, 0.0), None);
        let band = Fraction::new(0.25);
        assert_eq!(ratio_margin_verdict(None, band), Verdict::Ok);
        assert_eq!(
            ratio_margin_verdict(Some(Fraction::new(0.1)), band),
            Verdict::Warning
        );
        assert_eq!(
            ratio_margin_verdict(Some(Fraction::new(-0.1)), band),
            Verdict::Violation
        );
        assert_eq!(
            ratio_margin_verdict(Some(Fraction::new(0.25)), band),
            Verdict::Ok
        );
    }
}
