use super::*;
use sat_analysis::power::sizing::SizingOutput;
use serde_json::json;

#[test]
fn verdict_meter_is_ratio_to_limit_and_signed_when_limit_is_zero() {
    // DoD 0.2 / 上限 0.4 = 50 %: メーターは 0〜150 % なので塗りは 50/150、限界の線は 100/150
    let dod = verdict_view::<Fraction>(
        label("DoD", "DoD"),
        Some(0.2),
        0.4,
        LimitKind::AtMost,
        Some(Verdict::Ok),
    );
    assert!((dod.fill_end - 100.0 * 0.5 / 1.5).abs() < 1e-10);
    assert!((dod.limit_at - 100.0 / 1.5).abs() < 1e-10);
    assert!(dod.note.label.contains("上限の"));
    // 面積 2990 / 必要 5090 = 58.7 %
    let area = verdict_view::<sat_units::typed::Area>(
        label("面積", "Area"),
        Some(2990.0),
        5090.0,
        LimitKind::AtLeast,
        Some(Verdict::Violation),
    );
    assert!((area.fill_end - 100.0 * (2990.0 / 5090.0) / 1.5).abs() < 1e-10);
    assert!(area.note.label_en.contains("of required"));
    // 収支 −10（限界 0）: 0 を中央に、左へ伸ばす。|v|×1.15 が端
    let margin = verdict_view::<Energy>(
        label("収支", "Margin"),
        Some(-10.0),
        0.0,
        LimitKind::AtLeast,
        Some(Verdict::Violation),
    );
    assert_eq!(margin.limit_at, 50.0);
    assert_eq!(margin.fill_end, 50.0);
    assert!(margin.note.label.contains("不足"));
    let missing = verdict_view::<Energy>(
        label("収支", "Margin"),
        None,
        0.0,
        LimitKind::AtLeast,
        Some(Verdict::Warning),
    );
    assert!(missing.value.is_none());
    assert_eq!(missing.fill_end, 0.0);
}

#[test]
fn sizing_bars_keep_design_and_required_values_and_analysis_verdicts() {
    let json = json!({
        "mode_checks":{"nominal":[
            {"key":"sap_margin","label":"Area","verdict":"ok","message":"3 >= 2"},
            {"key":"battery_margin","label":"Capacity","verdict":"violation","message":"100 < 120"}
        ]},
        "basis":"time_series",
        "required_area":2,"required_capacity":120,
        "required_capacity_per_string":120,"design_area":3,"design_capacity":100,
        "sap_margin":0.5,"battery_margin":-0.16666666666666666,"strings":null,
        "area_mode":"nominal","capacity_mode":"nominal",
        "smad":{"required_generation":60,"required_area":1.5,"required_capacity":90,
          "sap_margin":1.0,"battery_margin":0.1111111111111111,"area_mode":"nominal",
          "capacity_mode":"nominal","sunlit_load":30,"eclipse_load":15},
        "time_series":null,
        "modes":[{"id":"nominal","sunlit_load":30,"eclipse_load":15,"incidence_factor":1,"required_generation":60,
          "required_area":1.5,"required_capacity":90,"series_area":2,"series_capacity":120,
          "area_ratio":1.3333333333333333,"capacity_ratio":1.3333333333333333}],
        "sunlit_duration":60,"eclipse_duration":40,
        "longest":{"epoch":"2027-04-01T00:00:00Z","beta":0,"umbra":40,"penumbra":0},
        "used":{"sunlit_efficiency":1,"eclipse_efficiency":1,"battery_efficiency":1,
          "annual_degradation":0,"inherent_degradation":1,"lifetime_years":1,"life_factor":1,
          "dod_max":0.4,"battery_strings":1,"sun_distance":149597870700.0,"panels":[]},
        "checks":[]
    });
    let out: SizingOutput = serde_json::from_value(json).unwrap();
    let checks = out.mode_checks.get("nominal");
    let area = &area_bar(&out, out.required_area, checks);
    assert_eq!(area.value.as_ref().unwrap().si, 3.0);
    assert_eq!(area.limit.si, 2.0);
    assert_eq!(area.verdict, Some(Verdict::Ok));
    assert!(area.fill_end > area.limit_at);
    let capacity = &capacity_bar(&out, out.required_capacity, checks);
    assert_eq!(capacity.value.as_ref().unwrap().si, 100.0);
    assert_eq!(capacity.limit.si, 120.0);
    assert_eq!(capacity.verdict, Some(Verdict::Violation));
    assert!(capacity.fill_end < capacity.limit_at);
}
