//! 解析の出力の語彙の表示名（日本語・英語）。解析はキー（時系列の種類・列挙の値）だけを返し、表示名はここで引く
//! （service が作る KPI の名前も）。
//! **解析の出力の表示名はここだけ**（設計データの語彙は `fune_model::builtin::labels`）。列挙は型ごとに漏れなく
//! `match` で書くので、解析に値が増えたら、ここが書かれるまでコンパイルが通らない。

use crate::dto::{AnalysisLabel, AocsUnloadMeans, KpiKey};
use sat_analysis::aocs::disturbance::{DisturbanceKind, SurfaceValueSource};
use sat_analysis::aocs::sensor_fov::SensorFovTarget;
use sat_analysis::aocs::slew_time::{SlewAssumption, SlewProfile};
use sat_analysis::aocs::unload_time::{MtqFieldGap, MtqFieldReason, UnloadAssumption};
use sat_analysis::comm::data_throughput::DataThroughputLinkClosure;
use sat_analysis::comm::link_budget::{LinkBudgetDirection, LinkDistanceModel};
use sat_analysis::power::energy_balance::{
    DurationTiming, EnergyBalanceDuty, EnergyBalanceEvaluation,
};
use sat_analysis::power::sizing::{SizingLifetimeSource, SizingSunDistanceSource};
use sat_analysis::power::solar::SolarLife;
use sat_analysis::spec::{ChannelKind, SpanFallback};
use sat_analysis::structure::cone_clearance::{ConeKind, ConeSkipReason};
use sat_analysis::structure::external_geometry::{BodyAxis, FaceDirection};
use sat_analysis::structure::interference::InterferenceSkipReason;
use sat_analysis::structure::launch_fit::{
    LaunchFitDirection, LaunchFitEnvelopeSource, LaunchFitSource,
};
use sat_analysis::structure::modal_estimate::ModalEstimateKind;
use sat_analysis::structure::modal_fem::{ModalFemAxis, ModalFemModeClass};
use sat_analysis::thermal::ThermalHeaterState;
use sat_analysis::thermal::network::ThermalCouplingSource;
use serde::de::DeserializeOwned;

pub(crate) fn label(ja: impl Into<String>, en: impl Into<String>) -> AnalysisLabel {
    AnalysisLabel {
        label: ja.into(),
        label_en: en.into(),
    }
}

/// 熱の結合の出どころの表示名。
pub fn coupling_source(s: ThermalCouplingSource) -> AnalysisLabel {
    use ThermalCouplingSource as S;
    match s {
        S::Mount => label("取付の既定値", "Mount default"),
        S::Contact => label("接触面積", "Contact area"),
        S::Explicit => label("書いた値", "Written value"),
        S::Enclosure => label("箱の放射", "Enclosure radiation"),
        S::ViewFactor => label("形態係数", "View factors"),
    }
}

/// 熱の結合の種類の表示名（伝導・放射）。
pub fn coupling_kind(k: crate::dto::ThermalCouplingKind) -> AnalysisLabel {
    use crate::dto::ThermalCouplingKind as K;
    match k {
        K::Conductive => label("伝導", "Conduction"),
        K::Radiative => label("放射", "Radiation"),
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
        K::HeaterPower => label(format!("{s} の発熱"), format!("{s} heater power")),
        K::Backlog => label("未送信データ", "Backlog"),
        K::Altitude => label("高度", "Altitude"),
        K::Inclination => label("傾斜角", "Inclination"),
        K::Mass => label("質量", "Mass"),
        K::HeliocentricX => match s {
            "spacecraft" => label("宇宙機の日心 x", "Spacecraft heliocentric x"),
            "departure" => label("出発天体の日心 x", "Departure body heliocentric x"),
            _ => label("到着天体の日心 x", "Arrival body heliocentric x"),
        },
        K::HeliocentricY => match s {
            "spacecraft" => label("宇宙機の日心 y", "Spacecraft heliocentric y"),
            "departure" => label("出発天体の日心 y", "Departure body heliocentric y"),
            _ => label("到着天体の日心 y", "Arrival body heliocentric y"),
        },
        K::ArrivalRange => label("到着天体までの距離", "Distance to the arrival body"),
        K::SunDistance => label("太陽までの距離", "Distance to the Sun"),
        K::EarthDistance => label("地球までの距離", "Distance to the Earth"),
        K::SunEarthProbeAngle => {
            label("太陽–地球–探査機の角（SEP）", "Sun–Earth–probe angle (SEP)")
        }
        K::SunProbeEarthAngle => {
            label("太陽–探査機–地球の角（SPE）", "Sun–probe–Earth angle (SPE)")
        }
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

fn surface_value_source(s: SurfaceValueSource) -> AnalysisLabel {
    match s {
        SurfaceValueSource::Written => label("設定に書いた値", "Written in the settings"),
        SurfaceValueSource::Geometry => label(
            "部品の箱の投影から向きごとに出した値",
            "Per direction from the projection of the part boxes",
        ),
        SurfaceValueSource::CentreOfMass => label(
            "重心に置いた（トルクは 0）",
            "Placed at the centre of mass (zero torque)",
        ),
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

fn mtq_field_reason(r: MtqFieldReason) -> AnalysisLabel {
    match r {
        MtqFieldReason::Absent => label(
            "中心天体に全球の磁場が無いので、MTQ では角運動量を捨てられません",
            "The central body has no global magnetic field, so the magnetorquers cannot unload momentum",
        ),
        MtqFieldReason::NotModelled => label(
            "中心天体の磁場のモデルが無い（地球の磁場だけを扱う）ので、MTQ でのアンロードを評価していません",
            "There is no magnetic field model for the central body (only Earth's field is modelled), so magnetorquer unloading is not evaluated",
        ),
    }
}

/// MTQ を評価できなかったことの文。RCS がすべての向きを捨てられるなら、アンロードは RCS で行うと添える。
fn mtq_field_gap(g: MtqFieldGap) -> AnalysisLabel {
    let reason = mtq_field_reason(g.reason);
    if !g.covered_by_rcs {
        return reason;
    }
    label(
        format!(
            "{}。RCS がすべての向きの角運動量を捨てられるので、アンロードは RCS で行います",
            reason.label
        ),
        format!(
            "{}. RCS can unload momentum in every direction, so unloading is done by RCS",
            reason.label_en
        ),
    )
}

/// MTQ を中心天体の磁場で評価できなかったことの判定（key `mtq_field_model`）の項目名と文。
pub fn mtq_field_check(g: MtqFieldGap) -> (AnalysisLabel, AnalysisLabel) {
    (
        label(
            "MTQ の評価に使う磁場",
            "Magnetic field for the magnetorquers",
        ),
        mtq_field_gap(g),
    )
}

/// アンロードの手段の表示名（AOCS のモードカード）。
pub fn unload_means(means: AocsUnloadMeans) -> AnalysisLabel {
    match means {
        AocsUnloadMeans::Mtq => label("MTQ", "MTQ"),
        AocsUnloadMeans::Rcs => label("RCS（最悪の向き）", "RCS (worst direction)"),
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
        SpanFallback::OneOffSpan => label(
            "全期間の平均（1 回限りの軌道）",
            "Mean over the whole span (one-off orbit)",
        ),
    }
}

fn link_direction(d: LinkBudgetDirection) -> AnalysisLabel {
    match d {
        LinkBudgetDirection::Uplink => label("アップリンク", "Uplink"),
        LinkBudgetDirection::Downlink => label("ダウンリンク", "Downlink"),
    }
}

fn link_distance_model(m: LinkDistanceModel) -> AnalysisLabel {
    match m {
        LinkDistanceModel::EarthOrbit => label(
            "地球周回の斜距離（局から見た）",
            "Earth-orbit slant range (from the station)",
        ),
        LinkDistanceModel::EarthToBody => label(
            "地球から天体周回の衛星までの距離（地心）",
            "Distance from Earth to the spacecraft around another body (geocentre)",
        ),
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
            "基本のモードの上で最長継続時間だけ続けたとき（開始はモードの指定に従う。指定が無ければ周回内の最悪）",
            "Max duration on top of the base mode (start follows the mode's timing; worst start in the orbit if none)",
        ),
        E::Sequence => label("モードの並びに沿って回したとき", "Along the mode sequence"),
    }
}

fn duration_timing(t: DurationTiming) -> AnalysisLabel {
    match t {
        DurationTiming::Fixed => label(
            "開始を指定（食の出入りからの位置）",
            "Start fixed (relative to an eclipse event)",
        ),
        DurationTiming::Windowed => label(
            "使う時間帯の中でずらした",
            "Shifted within the allowed time window",
        ),
        DurationTiming::Unspecified => label(
            "タイミング未指定（1 周の中の最悪）",
            "Timing unspecified (worst start in the orbit)",
        ),
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

fn sizing_sun_distance_source(s: SizingSunDistanceSource) -> AnalysisLabel {
    match s {
        SizingSunDistanceSource::Setting => label("設定", "Setting"),
        SizingSunDistanceSource::Nominal => {
            label("中心天体の慣例値", "Conventional value of the central body")
        }
        SizingSunDistanceSource::SpanMax => label(
            "期間の中の太陽距離の最大（最悪の日）",
            "Farthest Sun distance over the span (worst day)",
        ),
    }
}

fn sizing_lifetime_source(s: SizingLifetimeSource) -> AnalysisLabel {
    match s {
        SizingLifetimeSource::SeasonSpan => label("季節の走査期間", "Season scan span"),
        SizingLifetimeSource::DesignLife => label("設計寿命", "Design life"),
    }
}

fn heater_state(s: ThermalHeaterState) -> AnalysisLabel {
    match s {
        ThermalHeaterState::Disabled => label("無効（モードで切っている）", "Disabled by mode"),
        ThermalHeaterState::Off => label("オフ（加熱不要）", "Off (no heating needed)"),
        ThermalHeaterState::Regulating => label("制御中", "Regulating"),
        ThermalHeaterState::Saturated => {
            label("飽和（容量が足りない）", "Saturated (capacity too small)")
        }
    }
}

fn modal_kind(k: ModalEstimateKind) -> AnalysisLabel {
    match k {
        ModalEstimateKind::Panel => label("パネルの面外一次", "Panel out-of-plane"),
        ModalEstimateKind::BeamLateral => label("梁の横一次", "Beam lateral"),
        ModalEstimateKind::BeamAxial => label("梁の機軸一次", "Beam axial"),
    }
}

fn launch_fit_direction(d: LaunchFitDirection) -> AnalysisLabel {
    match d {
        LaunchFitDirection::Lateral => label("機軸に直交する方向", "Lateral"),
        LaunchFitDirection::Axial => label("機軸方向", "Axial"),
    }
}

fn launch_fit_source(s: LaunchFitSource) -> AnalysisLabel {
    match s {
        LaunchFitSource::BeamEstimate => label("梁の見積もり", "Beam estimate"),
        LaunchFitSource::FiniteElement => label("粗い有限要素", "Coarse finite element"),
    }
}

fn launch_fit_envelope_source(s: LaunchFitEnvelopeSource) -> AnalysisLabel {
    match s {
        LaunchFitEnvelopeSource::RootSize => label("構体（取付の根）の寸法", "Root part size"),
        LaunchFitEnvelopeSource::Parts => label("全部品の箱", "Part boxes"),
    }
}

fn interference_skip_reason(r: InterferenceSkipReason) -> AnalysisLabel {
    match r {
        InterferenceSkipReason::NoSize => label("寸法が無い", "No size"),
        InterferenceSkipReason::InvalidSize => label("寸法が正の値でない", "Size is not positive"),
    }
}

fn body_axis(a: BodyAxis) -> AnalysisLabel {
    match a {
        BodyAxis::X => label("X 軸", "X axis"),
        BodyAxis::Y => label("Y 軸", "Y axis"),
        BodyAxis::Z => label("Z 軸", "Z axis"),
    }
}

/// 有限要素の固有値解析の軸は機体の軸（`BodyAxis` と同じ表示名）。
fn modal_fem_axis(a: ModalFemAxis) -> AnalysisLabel {
    body_axis(match a {
        ModalFemAxis::X => BodyAxis::X,
        ModalFemAxis::Y => BodyAxis::Y,
        ModalFemAxis::Z => BodyAxis::Z,
    })
}

fn modal_fem_mode_class(c: ModalFemModeClass) -> AnalysisLabel {
    match c {
        ModalFemModeClass::Rigid => label("剛体（拘束のないモード）", "Rigid body"),
        ModalFemModeClass::Axial => label("全体・機軸方向", "Global, axial"),
        ModalFemModeClass::Lateral => label("全体・機軸に直交", "Global, lateral"),
        ModalFemModeClass::Local => label("局所（パネルなど）", "Local (panels and the like)"),
    }
}

fn face_direction(d: FaceDirection) -> AnalysisLabel {
    match d {
        FaceDirection::NegX => label("−X 面", "−X faces"),
        FaceDirection::PosX => label("+X 面", "+X faces"),
        FaceDirection::NegY => label("−Y 面", "−Y faces"),
        FaceDirection::PosY => label("+Y 面", "+Y faces"),
        FaceDirection::NegZ => label("−Z 面", "−Z faces"),
        FaceDirection::PosZ => label("+Z 面", "+Z faces"),
    }
}

fn cone_kind(k: ConeKind) -> AnalysisLabel {
    match k {
        ConeKind::Plume => label("噴流", "Plume"),
        ConeKind::FieldOfView => label("視野", "Field of view"),
        ConeKind::Beam => label("アンテナのビーム", "Antenna beam"),
    }
}

fn cone_skip_reason(r: ConeSkipReason) -> AnalysisLabel {
    match r {
        ConeSkipReason::NoAxis => label("軸の向きが読めない", "No axis direction"),
        ConeSkipReason::InvalidAxis => label("軸の向きが零または不正", "Axis is zero or invalid"),
        ConeSkipReason::NoHalfAngle => label("半頂角が無い", "No half angle"),
        ConeSkipReason::InvalidHalfAngle => label(
            "半頂角が 0 より大きく 90° より小さくない",
            "Half angle is not between 0 and 90 degrees",
        ),
    }
}

mod orbit;
mod propulsion;

fn link_closure(kind: &str) -> Option<AnalysisLabel> {
    // 内部タグの列挙はタグの値だけを見る（中身の量は dims で整形する）
    let probe = match kind {
        "not_evaluated" => DataThroughputLinkClosure::NotEvaluated,
        "never_closes" => DataThroughputLinkClosure::NeverCloses,
        "no_band_settings" => DataThroughputLinkClosure::NoBandSettings,
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
        DataThroughputLinkClosure::NoBandSettings => label(
            "この帯域の設定が地上局に無い（数えない）",
            "No settings for this band (not counted)",
        ),
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
        "SurfaceValueSource" => parse(value).map(surface_value_source),
        "SensorFovTarget" => parse(value).map(sensor_target),
        "UnloadAssumption" => parse(value).map(unload_assumption),
        "MtqFieldReason" => parse(value).map(mtq_field_reason),
        "SlewProfile" => parse(value).map(slew_profile),
        "SlewAssumption" => parse(value).map(slew_assumption),
        "SpanFallback" => parse(value).map(span_fallback),
        "LinkBudgetDirection" => parse(value).map(link_direction),
        "LinkDistanceModel" => parse(value).map(link_distance_model),
        "SolarLife" => parse(value).map(solar_life),
        "ModalEstimateKind" => parse(value).map(modal_kind),
        "LaunchFitDirection" => parse(value).map(launch_fit_direction),
        "LaunchFitEnvelopeSource" => parse(value).map(launch_fit_envelope_source),
        "LaunchFitSource" => parse(value).map(launch_fit_source),
        "InterferenceSkipReason" => parse(value).map(interference_skip_reason),
        "BodyAxis" => parse(value).map(body_axis),
        "ModalFemAxis" => parse(value).map(modal_fem_axis),
        "ModalFemModeClass" => parse(value).map(modal_fem_mode_class),
        "FaceDirection" => parse(value).map(face_direction),
        "ConeKind" => parse(value).map(cone_kind),
        "ConeSkipReason" => parse(value).map(cone_skip_reason),
        "EnergyBalanceEvaluation" => parse(value).map(energy_balance_evaluation),
        "EnergyBalanceDuty" => parse(value).map(energy_balance_duty),
        "DurationTiming" => parse(value).map(duration_timing),
        "SizingLifetimeSource" => parse(value).map(sizing_lifetime_source),
        "SizingSunDistanceSource" => parse(value).map(sizing_sun_distance_source),
        "ThermalHeaterState" => parse(value).map(heater_state),
        "DataThroughputLinkClosure" => link_closure(value),
        "SolarActivity" => parse(value).map(orbit::solar_activity),
        "WeatherSource" => parse(value).map(orbit::weather_source),
        "DragAreaSource" => parse(value).map(orbit::drag_area_source),
        "GeometrySpanBasis" => parse(value).map(orbit::geometry_span_basis),
        "DesignMarginSource" => parse(value).map(orbit::design_margin_source),
        "ShieldingSource" => parse(value).map(orbit::shielding_source),
        "DoseLegStatus" => parse(value).map(orbit::dose_leg_status),
        "DoseNotEvaluated" => parse(value).map(orbit::dose_not_evaluated),
        "DoseInterpolation" => parse(value).map(orbit::dose_interpolation),
        "DecayCondition" => orbit::decay_condition(value),
        "DvCategory" => parse(value).map(propulsion::category),
        "TransferMethod" => parse(value).map(propulsion::method),
        "ManeuverNote" => parse(value).map(propulsion::note),
        "EphemerisSource" => parse(value).map(propulsion::ephemeris_source),
        "ThrustSteering" => parse(value).map(propulsion::steering),
        "ThrustBalanceNote" => parse(value).map(propulsion::thrust_balance_note),
        "LaunchWindowNote" => parse(value).map(propulsion::launch_window_note),
        "LaunchWindowPointKind" => parse(value).map(propulsion::launch_window_point_kind),
        "LaunchWindowQuantity" => parse(value).map(propulsion::launch_window_quantity),
        "TransferCheckNote" => parse(value).map(propulsion::transfer_check_note),
        "UnloadingNote" => parse(value).map(propulsion::unloading),
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
        K::SunlitFraction => label("日照率", "Sunlit fraction"),
        K::OrbitBalance => label(
            "電力収支（軌道平均・最悪）",
            "Power balance (orbit average, worst)",
        ),
        K::MaxDod => label("最大 DoD", "Max DoD"),
        K::SapMargin => label("太陽電池の余裕", "Solar Cells margin"),
        K::BatteryMargin => label("バッテリーの余裕", "Battery margin"),
        K::UnloadInterval => label("アンロード間隔", "Unload interval"),
        K::PeakMomentum => label("最大角運動量", "Peak momentum"),
        K::MtqUnloadTime => label("MTQ のアンロード時間", "MTQ unload time"),
        K::RcsUnloadTime => label("RCS のアンロード時間", "RCS unload time"),
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
        K::MaxHeaterDuty => label("ヒータのデューティ（最大）", "Heater duty (max)"),
        K::TransientMaxHeaterDuty => label(
            "非定常のヒータのデューティ（最大・最後の 1 周）",
            "Transient heater duty (max, last orbit)",
        ),
        K::TotalMass => label("総質量", "Total mass"),
        K::MinSafetyMargin => label("最小の安全余裕（MS）", "Minimum margin of safety"),
        K::MinFrequency => label("最低の固有振動数", "Lowest natural frequency"),
        K::LongestTransfer => label("最長の遷移時間", "Longest transfer time"),
        K::DvWithMargin => label("ΔV（マージン込み）", "Delta-V (with margin)"),
        K::LoadedPropellant => label("積む推薬", "Loaded propellant"),
        K::TankVolumeMargin => label("タンクの容積の余裕", "Tank volume margin"),
        K::ImpulseMargin => label("総インパルスの余裕", "Total impulse margin"),
        K::ManeuversEnd => label("マヌーバが終わる時刻", "Maneuvers end"),
        K::ComShift => label(
            "推薬による重心の移動",
            "Center-of-mass shift from propellant",
        ),
        K::ThrustMaxDod => label("噴射中の最大 DoD", "Max DoD while thrusting"),
        K::LaunchFitMargin => label("打ち上げ適合の余裕（最小）", "Launch fit margin (min)"),
        K::InterferenceOverlaps => label("部品の重なり（組）", "Part overlaps (pairs)"),
        K::ConeObstructions => label("円錐の遮り", "Cone obstructions"),
        K::MeanProjectedArea => label("投影面積（平均）", "Projected area (mean)"),
        K::FirstFrequency => label(
            "最低の固有振動数（有限要素）",
            "Lowest natural frequency (finite element)",
        ),
        K::TransferMiss => label("到着のずれ（最大）", "Arrival miss distance (max)"),
        K::LaunchWindowDeltaV => label("窓の ΔV（最小）", "Launch window delta-v (min)"),
        K::TotalDoseMargin => label("総線量の余裕（最小）", "Total dose margin (min)"),
        K::GreatestEarthDistance => {
            label("地球までの距離（最大）", "Distance from the Earth (max)")
        }
    }
}

/// 円制限三体問題の軌道を暦に載せた軌道の説明（key `orbit_model`）の項目名と文。
pub fn orbit_model_check() -> (AnalysisLabel, AnalysisLabel) {
    (
        label("軌道のモデル", "Orbit model"),
        label(
            "軌道は円制限三体問題（CR3BP）のラグランジュ点またはハロー軌道を、暦の 2 天体の位置と速度に載せたものです（力モデルの伝搬ではなく、太陽・月・惑星の摂動と太陽輻射圧は入りません）。軌道維持は見積もらないので、ΔV の余裕としてマヌーバに書いてください",
            "The orbit is a libration point or halo orbit of the circular restricted three-body problem (CR3BP) carried on the ephemeris positions and velocities of the two bodies (not a force-model propagation: third-body perturbations and solar radiation pressure are not included). Station keeping is not estimated, so write it as a delta-V allowance in a maneuver",
        ),
    )
}

/// 期間不足の注意（key `span`）の項目名と文。1 回限りの軌道（`OneOffSpan`）は期間不足ではなく、周回が無いことの説明。
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
        SpanFallback::OneOffSpan => {
            return (
                label("解析期間", "Analysis span"),
                label(
                    "軌道は 1 回限りの区間（惑星間の巡航の、両端の天体の影響圏のあいだなど）で周回が無いため、軌道平均と 1 周あたりの量は区間全体で求めています（影響圏の中の双曲線の弧は解析しません）",
                    "The orbit is a one-off arc (such as an interplanetary cruise between the spheres of influence of its end bodies; the hyperbolic legs inside them are not analysed) with no repeating orbit, so orbit averages and per-orbit quantities are taken over the whole arc",
                ),
            );
        }
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
mod tests;
