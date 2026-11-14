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
fn sizing_lifetime_source_has_both_languages() {
    let d = value_label("SizingLifetimeSource", "design_life").unwrap();
    assert_eq!(
        (d.label.as_str(), d.label_en.as_str()),
        ("設計寿命", "Design life")
    );
    let s = value_label("SizingLifetimeSource", "season_span").unwrap();
    assert_eq!(s.label_en, "Season scan span");
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
    assert_eq!(
        value_label("DvCategory", "deorbit").unwrap().label_en,
        "Deorbit"
    );
    assert!(value_label("UnloadingNote", "no_rcs").is_some());
    let (l, m) = span_check(SpanFallback::OrbitMeanOverSpan);
    assert_eq!(l.label, "解析期間");
    assert!(m.label.contains("全期間の平均") && m.label_en.contains("whole span"));
}
