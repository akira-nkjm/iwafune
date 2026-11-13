//! 解析の出力の語彙の表示名（日本語・英語）。解析はキー（時系列の種類・列挙の値）だけを返し、表示名はここで引く
//! （service が作る KPI の名前も）。
//! **解析の出力の表示名はここだけ**（設計データの語彙は `fune_model::builtin::labels`）。列挙は型ごとに漏れなく
//! `match` で書くので、解析に値が増えたら、ここが書かれるまでコンパイルが通らない。

use crate::dto::{AnalysisLabel, KpiKey};
use sat_analysis::aocs::disturbance::DisturbanceKind;
use sat_analysis::aocs::sensor_fov::SensorFovTarget;
use sat_analysis::aocs::slew_time::{SlewAssumption, SlewProfile};
use sat_analysis::aocs::unload_time::UnloadAssumption;
use sat_analysis::comm::data_throughput::DataThroughputLinkClosure;
use sat_analysis::comm::link_budget::LinkBudgetDirection;
use sat_analysis::orbit::decay::{DecayCondition, DragAreaSource};
use sat_analysis::power::energy_balance::{EnergyBalanceDuty, EnergyBalanceEvaluation};
use sat_analysis::power::solar::SolarLife;
use sat_analysis::spec::{ChannelKind, SpanFallback};
use sat_analysis::structure::modal_estimate::ModalEstimateKind;
use sat_astro::atmosphere::space_weather::{SolarActivity, WeatherSource};
use serde::de::DeserializeOwned;

fn label(ja: impl Into<String>, en: impl Into<String>) -> AnalysisLabel {
    AnalysisLabel {
        label: ja.into(),
        label_en: en.into(),
    }
}

/// 時系列の系列の表示名。対象（`subject`。部品・センサ・節点の ID）を持つ系列は名前に ID を入れる。
pub fn channel_label(kind: ChannelKind, subject: Option<&str>) -> AnalysisLabel {
    let s = subject.unwrap_or("");
    use ChannelKind as K;
    match kind {
        K::Beta => label("β 角", "Beta angle"),
        K::UmbraDuration => label("本影の時間", "Umbra duration"),
        K::PenumbraDuration => label("半影の時間", "Penumbra duration"),
        K::Generation => label("発電の合計", "Total generation"),
        K::Sunlit => label("日照率", "Sunlit fraction"),
        K::PanelGeneration => label(format!("{s} の発電"), format!("{s} generation")),
        K::Load => label("消費電力", "Load"),
        K::Soc => label("SOC", "SOC"),
        K::ChargePower => label("充電電力", "Charge power"),
        K::DischargePower => label("放電電力", "Discharge power"),
        K::ShuntPower => label("余剰電力", "Shunted power"),
        K::GravityGradientTorque => disturbance(DisturbanceKind::GravityGradient),
        K::AeroTorque => disturbance(DisturbanceKind::Aero),
        K::SrpTorque => disturbance(DisturbanceKind::Srp),
        K::MagneticTorque => disturbance(DisturbanceKind::Magnetic),
        K::ThrustTorque => disturbance(DisturbanceKind::Thrust),
        K::TotalDisturbanceTorque => label("擾乱トルクの合計", "Total disturbance torque"),
        K::MomentumX => label("角運動量 x（機体）", "Momentum x (body)"),
        K::MomentumY => label("角運動量 y（機体）", "Momentum y (body)"),
        K::MomentumZ => label("角運動量 z（機体）", "Momentum z (body)"),
        K::MomentumNorm => label("角運動量の大きさ", "Momentum magnitude"),
        K::WheelMomentum => label(format!("{s} の角運動量"), format!("{s} momentum")),
        K::SunAngle => sensor_angle(s, SensorFovTarget::Sun),
        K::EarthLimbAngle => sensor_angle(s, SensorFovTarget::EarthLimb),
        K::MoonAngle => sensor_angle(s, SensorFovTarget::Moon),
        K::NodeTemperature => label(s, s),
        K::AbsorbedHeat => label("外部入熱", "Absorbed heat"),
        K::Backlog => label("未送信データ", "Backlog"),
        K::Altitude => label("高度", "Altitude"),
    }
}

fn sensor_angle(sensor: &str, target: SensorFovTarget) -> AnalysisLabel {
    let t = sensor_target(target);
    label(
        format!("{sensor} と{}のなす角", t.label),
        format!("Angle from {sensor} to the {}", t.label_en.to_lowercase()),
    )
}

fn disturbance(k: DisturbanceKind) -> AnalysisLabel {
    match k {
        DisturbanceKind::GravityGradient => label("重力傾斜トルク", "Gravity-gradient torque"),
        DisturbanceKind::Aero => label("空力トルク", "Aerodynamic torque"),
        DisturbanceKind::Srp => label("太陽輻射圧トルク", "Solar radiation pressure torque"),
        DisturbanceKind::Magnetic => label("残留磁気トルク", "Residual magnetic torque"),
        DisturbanceKind::Thrust => label("推力誤差トルク", "Thrust misalignment torque"),
    }
}

fn sensor_target(t: SensorFovTarget) -> AnalysisLabel {
    match t {
        SensorFovTarget::Sun => label("太陽", "Sun"),
        SensorFovTarget::EarthLimb => label("地球の縁", "Earth limb"),
        SensorFovTarget::Moon => label("月", "Moon"),
    }
}

fn unload_assumption(a: UnloadAssumption) -> AnalysisLabel {
    match a {
        UnloadAssumption::MtqMeanField => label(
            "トルク = 有効率 × 最も弱い機体軸の双極子 × 軌道平均の |B|。B と直交する成分だけが使え、B の向きが軌道上で変わることを有効率で見込む",
            "Torque = efficiency × dipole of the weakest body axis × orbit-mean |B|. Only the component perpendicular to B acts; the efficiency accounts for B changing direction along the orbit",
        ),
        UnloadAssumption::RcsSimultaneousFiring => label(
            "−H 向きのトルク成分が正のスラスタをすべて同時に連続噴射する（オン・オフのみ）。H と直交するトルクと正味の推力は打ち消さずに残る",
            "All thrusters with a positive torque component along −H fire continuously at once (on/off only). Torque perpendicular to H and the net force are left uncancelled",
        ),
    }
}

fn slew_profile(p: SlewProfile) -> AnalysisLabel {
    match p {
        SlewProfile::Triangular => label(
            "三角形（加速して減速）",
            "Triangular (accelerate, decelerate)",
        ),
        SlewProfile::Trapezoidal => label(
            "台形（角運動量の上限で等速）",
            "Trapezoidal (coast at the momentum limit)",
        ),
    }
}

fn slew_assumption(a: SlewAssumption) -> AnalysisLabel {
    match a {
        SlewAssumption::EigenAxisRestToRest => label(
            "静止から静止まで固定した軸まわりに、RW の最大トルクで加速・減速する。ホイールの角運動量は 0 から始め、使ってよい割合まで使う。ジャイロ効果・擾乱・整定の時間は含めない",
            "Rest-to-rest rotation about a fixed axis, accelerating and braking at the RW torque limit. Wheels start from zero momentum and use up to the usable fraction. Gyroscopic coupling, disturbances and settling time are not included",
        ),
    }
}

fn span_fallback(f: SpanFallback) -> AnalysisLabel {
    match f {
        SpanFallback::OrbitMeanOverSpan => label("全期間の平均", "Mean over the whole span"),
        SpanFallback::SecularExtrapolated => {
            label("全期間からの外挿", "Extrapolated from the whole span")
        }
    }
}

fn link_direction(d: LinkBudgetDirection) -> AnalysisLabel {
    match d {
        LinkBudgetDirection::Uplink => label("アップリンク", "Uplink"),
        LinkBudgetDirection::Downlink => label("ダウンリンク", "Downlink"),
    }
}

fn solar_life(l: SolarLife) -> AnalysisLabel {
    match l {
        SolarLife::Bol => label("初期（BOL）", "Beginning of life"),
        SolarLife::Eol => label("寿命末期（EOL）", "End of life"),
    }
}

fn energy_balance_evaluation(e: EnergyBalanceEvaluation) -> AnalysisLabel {
    use EnergyBalanceEvaluation as E;
    match e {
        E::Continuous => label("解析期間ずっと続けたとき", "Run for the whole span"),
        E::NoBaseMode => label(
            "解析期間ずっと続けたとき（基本のモードが無いため最長継続時間を使わない）",
            "Run for the whole span (no base mode, so the max duration is not used)",
        ),
        E::BaseMode => label(
            "解析期間ずっと続けたとき（基本のモード）",
            "Run for the whole span (base mode)",
        ),
        E::MaxDuration => label(
            "基本のモードの上で最長継続時間だけ続けたとき（開始は周回内の最悪）",
            "Max duration on top of the base mode (worst start in the orbit)",
        ),
        E::Sequence => label("モードの並びに沿って回したとき", "Along the mode sequence"),
    }
}

fn energy_balance_duty(d: EnergyBalanceDuty) -> AnalysisLabel {
    match d {
        EnergyBalanceDuty::NotUsed => label("duty なし", "No duty"),
        EnergyBalanceDuty::Averaged => label(
            "duty は平均の消費電力として一定にかける（周回内の時間分布は扱わない）",
            "Duty is applied as a constant average power (no timing within the orbit)",
        ),
    }
}

fn modal_kind(k: ModalEstimateKind) -> AnalysisLabel {
    match k {
        ModalEstimateKind::Panel => label("パネルの面外一次", "Panel out-of-plane"),
        ModalEstimateKind::BeamLateral => label("梁の横一次", "Beam lateral"),
        ModalEstimateKind::BeamAxial => label("梁の機軸一次", "Beam axial"),
    }
}

fn solar_activity(a: SolarActivity) -> AnalysisLabel {
    match a {
        SolarActivity::Low => label("低い太陽活動", "Low solar activity"),
        SolarActivity::Moderate => label("中程度の太陽活動", "Moderate solar activity"),
        SolarActivity::High => label("高い太陽活動", "High solar activity"),
    }
}

fn weather_source(w: WeatherSource) -> AnalysisLabel {
    match w {
        WeatherSource::Observed => label("観測値", "Observed"),
        WeatherSource::Interpolated => label("補間値", "Interpolated"),
        WeatherSource::DailyPredicted => label("日ごとの予測", "Daily prediction"),
        WeatherSource::MonthlyPredicted => label("月ごとの予測", "Monthly prediction"),
        WeatherSource::Preset => label("太陽活動のプリセット", "Solar-activity preset"),
        WeatherSource::Manual => label("手で与えた値", "Manual value"),
    }
}

fn drag_area_source(s: DragAreaSource) -> AnalysisLabel {
    match s {
        DragAreaSource::Given => label("与えた面積", "Given area"),
        DragAreaSource::TumblingBox => label(
            "構体の外形の箱を向きでならした面積（表面積の 1/4）",
            "Body box averaged over orientations (a quarter of the surface area)",
        ),
    }
}

fn decay_condition(kind: &str) -> Option<AnalysisLabel> {
    // 内部タグの列挙はタグの値だけを見る（中身の太陽活動は別の欄の値として引く）
    let probe = match kind {
        "preset" => DecayCondition::Preset {
            activity: SolarActivity::Moderate,
        },
        "dated" => DecayCondition::Dated {
            fallback: SolarActivity::Moderate,
        },
        _ => return None,
    };
    Some(match probe {
        DecayCondition::Preset { .. } => label(
            "太陽活動のプリセットで一定",
            "Constant solar-activity preset",
        ),
        DecayCondition::Dated { .. } => label(
            "宇宙天気の表から日付で引いた値",
            "Dated values from the space-weather table",
        ),
    })
}

fn link_closure(kind: &str) -> Option<AnalysisLabel> {
    // 内部タグの列挙はタグの値だけを見る（中身の量は dims で整形する）
    let probe = match kind {
        "not_evaluated" => DataThroughputLinkClosure::NotEvaluated,
        "never_closes" => DataThroughputLinkClosure::NeverCloses,
        "closes_above" => DataThroughputLinkClosure::ClosesAbove {
            min_elevation: sat_units::typed::Qty::ZERO,
        },
        _ => return None,
    };
    Some(match probe {
        DataThroughputLinkClosure::NotEvaluated => label("回線を評価しない", "Not evaluated"),
        DataThroughputLinkClosure::ClosesAbove { .. } => {
            label("この仰角より上で閉じる", "Closes above this elevation")
        }
        DataThroughputLinkClosure::NeverCloses => label("天頂でも閉じない", "Never closes"),
    })
}

/// 列挙の値（serde の綴り）を型に読む。
fn parse<T: DeserializeOwned>(value: &str) -> Option<T> {
    serde_json::from_value(serde_json::Value::String(value.to_owned())).ok()
}

/// 出力の中の列挙の値の表示名。`ty` は Rust の型名（serde が書き出しに渡す名前）、`value` は JSON の綴り。
/// 語彙に無い型は `None`（ID などの文字列はそのまま出す）。
pub fn value_label(ty: &str, value: &str) -> Option<AnalysisLabel> {
    match ty {
        "DisturbanceKind" => parse(value).map(disturbance),
        "SensorFovTarget" => parse(value).map(sensor_target),
        "UnloadAssumption" => parse(value).map(unload_assumption),
        "SlewProfile" => parse(value).map(slew_profile),
        "SlewAssumption" => parse(value).map(slew_assumption),
        "SpanFallback" => parse(value).map(span_fallback),
        "LinkBudgetDirection" => parse(value).map(link_direction),
        "SolarLife" => parse(value).map(solar_life),
        "ModalEstimateKind" => parse(value).map(modal_kind),
        "EnergyBalanceEvaluation" => parse(value).map(energy_balance_evaluation),
        "EnergyBalanceDuty" => parse(value).map(energy_balance_duty),
        "DataThroughputLinkClosure" => link_closure(value),
        "SolarActivity" => parse(value).map(solar_activity),
        "WeatherSource" => parse(value).map(weather_source),
        "DragAreaSource" => parse(value).map(drag_area_source),
        "DecayCondition" => decay_condition(value),
        _ => None,
    }
}

/// KPI の表示名（`analysis::kpi`。キーはすべての系を通して一意）。
pub fn kpi_label(key: KpiKey) -> AnalysisLabel {
    use KpiKey as K;
    match key {
        K::LongestEclipse => label("最長蝕の時間", "Longest eclipse duration"),
        K::BetaMin => label("β の範囲（最小）", "Beta range (minimum)"),
        K::BetaMax => label("β の範囲（最大）", "Beta range (maximum)"),
        K::OrbitLifetime => label("軌道寿命（最短の条件）", "Orbital lifetime (shortest)"),
        K::DecayPerDay => label("1 日の高度低下（最大）", "Altitude decay per day (max)"),
        K::StationKeepingDeltaV => label(
            "軌道維持の ΔV / 年（最大）",
            "Station-keeping delta-v per year (max)",
        ),
        K::ModeLoad => label("モードの消費電力（バジェット）", "Mode load (budget)"),
        K::OrbitGeneration => label("軌道平均の発電", "Orbit-average generation"),
        K::OrbitBalance => label("軌道平均の収支（最悪）", "Orbit-average balance (worst)"),
        K::MaxDod => label("最大 DoD", "Max DoD"),
        K::SapMargin => label("太陽電池の余裕", "Solar Cells margin"),
        K::BatteryMargin => label("バッテリーの余裕", "Battery margin"),
        K::UnloadInterval => label("アンロード間隔", "Unload interval"),
        K::PeakMomentum => label("最大角運動量", "Peak momentum"),
        K::MtqUnloadTime => label("MTQ のアンロード時間", "MTQ unload time"),
        K::SensorIntrusion => label("センサ侵入の割合（最大）", "Sensor intrusion (max)"),
        K::SlewTime => label("姿勢変更の時間（最悪の軸）", "Slew time (worst axis)"),
        K::DataPerDay => label("1 日の発生量", "Generated per day"),
        K::StorageRequired => label("必要なストレージ", "Storage required"),
        K::StorageMargin => label("ストレージの容量の余裕", "Storage capacity margin"),
        K::LinkMargin => label("回線余裕（最小）", "Link margin (min)"),
        K::ContactPerDay => label("1 日の可視時間", "Contact per day"),
        K::DownlinkPerDay => label("1 日の送信可能量", "Downlink capacity per day"),
        K::MaxBacklog => label("最大の未送信", "Max backlog"),
        K::MaxTemperature => label("最高温度", "Max temperature"),
        K::MinTemperature => label("最低温度", "Min temperature"),
        K::NodesInRange => label("温度範囲の判定（節点）", "Temperature range checks (nodes)"),
        K::TransientMaxTemperature => label("非定常の最高温度", "Transient max temperature"),
        K::TotalMass => label("総質量", "Total mass"),
        K::MinSafetyMargin => label("最小の安全余裕（MS）", "Minimum margin of safety"),
        K::MinFrequency => label("最低の固有振動数", "Lowest natural frequency"),
    }
}

/// 期間不足の注意（key `span`）の項目名と文。
pub fn span_check(f: SpanFallback) -> (AnalysisLabel, AnalysisLabel) {
    let consequence = match f {
        SpanFallback::OrbitMeanOverSpan => label(
            "軌道平均は全期間の平均です",
            "orbit averages are taken over the whole span",
        ),
        SpanFallback::SecularExtrapolated => label(
            "永年成分は全期間から外挿しています",
            "the secular part is extrapolated from the whole span",
        ),
    };
    (
        label("解析期間", "Analysis span"),
        label(
            format!("解析期間が 1 周に満たないため、{}", consequence.label),
            format!(
                "The span is shorter than one orbit, so {}",
                consequence.label_en
            ),
        ),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn channels_name_their_subject() {
        let w = channel_label(ChannelKind::WheelMomentum, Some("RW_X"));
        assert_eq!(
            (w.label.as_str(), w.label_en.as_str()),
            ("RW_X の角運動量", "RW_X momentum")
        );
        let s = channel_label(ChannelKind::SunAngle, Some("STT"));
        assert_eq!(s.label, "STT と太陽のなす角");
        assert_eq!(s.label_en, "Angle from STT to the sun");
        assert_eq!(
            channel_label(ChannelKind::NodeTemperature, Some("PANEL")).label,
            "PANEL"
        );
    }

    #[test]
    fn values_are_looked_up_by_type_and_spelling() {
        let d = value_label("DisturbanceKind", "gravity_gradient").unwrap();
        assert_eq!(d.label, "重力傾斜トルク");
        assert_eq!(
            value_label("SensorFovTarget", "earth_limb")
                .unwrap()
                .label_en,
            "Earth limb"
        );
        assert!(value_label("DataThroughputLinkClosure", "closes_above").is_some());
        assert!(value_label("DisturbanceKind", "unknown").is_none());
        assert_eq!(
            value_label("DisturbanceKind", "thrust").unwrap().label,
            "推力誤差トルク"
        );
        assert!(value_label("SlewProfile", "trapezoidal").is_some());
        assert!(value_label("SlewAssumption", "eigen_axis_rest_to_rest").is_some());
        assert!(value_label("String", "RW_X").is_none());
        let high = value_label("SolarActivity", "high").unwrap();
        assert_eq!(high.label_en, "High solar activity");
        assert!(value_label("DecayCondition", "dated").is_some());
        let (l, m) = span_check(SpanFallback::OrbitMeanOverSpan);
        assert_eq!(l.label, "解析期間");
        assert!(m.label.contains("全期間の平均") && m.label_en.contains("whole span"));
    }
}
