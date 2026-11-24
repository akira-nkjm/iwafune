use super::*;
use crate::dto::PowerBudgetKind;
use sat_analysis::power::{energy_balance::EnergyBalanceCaseResult, mode_load::ModeLoadMode};
use sat_analysis::power::{
    energy_balance::{EnergyBalanceInput, EnergyBalanceLoad, EnergyBalanceOutput},
    sizing::SizingOutput,
};
use sat_analysis::spec::Check;
use sat_units::typed::Fraction;
use serde_json::json;

fn fixture() -> (ModeLoadMode, EnergyBalanceCaseResult) {
    let loads = serde_json::from_value(json!({
        "id":"nominal", "label":"Nominal", "total":30, "total_with_growth":30,
        "total_eclipse":15,"total_eclipse_with_growth":15,"peak_with_growth":30,
        "groups":[],"unassigned":[],"missing_power":[],"checks":[],
        "parts":[
          {"part":"CPU","label":"CPU","group":"cdh","states":[],"mean":10,"mean_with_growth":10,"mean_eclipse":5,"mean_eclipse_with_growth":5,"growth_margin":0,"spare_power":null},
          {"part":"RADIO","label":"Radio","group":"comm","states":[],"mean":20,"mean_with_growth":20,"mean_eclipse":10,"mean_eclipse_with_growth":10,"growth_margin":0,"spare_power":null}
        ]
    })).unwrap();
    let case = serde_json::from_value(json!({
        "id":"nominal","label":"Nominal","load_sunlit":30,"load_eclipse":15,
        "orbit_average_generation":30,"sunlit_average_generation":60,
        "sunlit_average_charge":20,"sunlit_average_discharge":0,"sunlit_average_shunt":10,
        "orbit_average_load":22.5,"orbit_average_balance":7.5,
        "energy_margin_per_orbit":750,"energy_margin_over_span":null,
        "min_soc":0.8,"max_dod":0.2,"max_eclipse_dod":0.2,"final_soc":1,
        "peak_charge_power":20,"peak_discharge_power":15,"peak_discharge_current":1.5,
        "orbits":[],"series":{"t":[],"channels":[]},"checks":[]
    }))
    .unwrap();
    (loads, case)
}

#[test]
fn budget_by_hand_keeps_power_and_shares_one_scale_across_modes() {
    use sat_analysis::power::energy_balance::LoadGrowth;
    let (loads, case) = fixture();
    // 2 モード分（同じ値）を並べ、同じ目盛りで幅を付ける
    let sun = budget::finish(vec![
        Some(budget::collect(&loads, &case, false, LoadGrowth::Included)),
        Some(budget::collect(&loads, &case, false, LoadGrowth::Included)),
    ]);
    let s = sun[0].as_ref().unwrap();
    assert_eq!(s.supply.si, 60.0);
    // 系の消費 + 充電 + 余り = 日照中の発電 60 W
    assert_eq!(s.segments.iter().map(|g| g.value.si).sum::<f64>(), 60.0);
    // いちばん長い棒の幅の合計は 1、各区間は電力に比例
    assert!((s.segments.iter().map(|g| g.fraction).sum::<f64>() - 1.0).abs() < 1e-12);
    for g in &s.segments {
        assert!((g.fraction - g.value.si / 60.0).abs() < 1e-12);
    }
    let kinds: Vec<_> = s.segments.iter().map(|g| g.kind).collect();
    assert_eq!(kinds.first(), Some(&PowerBudgetKind::Load));
    assert!(kinds.contains(&PowerBudgetKind::Charge));
    let eclipse = budget::finish(vec![Some(budget::collect(
        &loads,
        &case,
        true,
        LoadGrowth::Included,
    ))]);
    let e = eclipse[0].as_ref().unwrap();
    assert_eq!(e.segments.iter().map(|g| g.value.si).sum::<f64>(), 15.0);
    assert!(e.segments.iter().all(|g| g.kind == PowerBudgetKind::Load));
}

#[test]
fn builder_returns_selected_mode_values_bands_and_soc_limit() {
    use sat_analysis::power::{
        energy_balance::{EnergyBalanceCase, LoadGrowth},
        solar::{SolarArray, SolarLife},
    };
    use sat_analysis::spec::{Channel, ChannelKind, Series};
    use sat_astro::{
        attitude::AttitudeLaw, math::Vec3, orbit::OrbitSpec, time::Epoch, timeline::TimelineSpec,
    };
    use sat_units::typed::Qty;
    let (loads, mut case) = fixture();
    let orbit = OrbitSpec::circular(
        Epoch::parse("2027-04-01T00:00:00Z").unwrap(),
        Qty::from_raw(500e3),
        Qty::from_raw(0.0),
        Qty::from_raw(0.0),
    );
    let input = EnergyBalanceInput {
        timeline: TimelineSpec::new(
            orbit,
            AttitudeLaw::SunPointing {
                axis: Vec3::Z,
                secondary: Vec3::X,
            },
            Qty::from_raw(20.0),
            Qty::from_raw(10.0),
        ),
        array: SolarArray {
            panels: Vec::new(),
            solar_flux: None,
            conversion_efficiency: None,
            shadowing: None,
        },
        batteries: Vec::new(),
        life: Some(SolarLife::Bol),
        loads: Default::default(),
        load_growth: Some(LoadGrowth::Included),
        cases: vec![EnergyBalanceCase {
            max_duration: None,
            timing: Default::default(),
            id: "nominal".into(),
            label: "Nominal".into(),
            load: EnergyBalanceLoad::Mode {
                mode: "nominal".into(),
            },
            attitude: None,
        }],
        sequence: Vec::new(),
        sequence_timing: Default::default(),
        base_mode: None,
        mode_attitudes: Default::default(),
    };
    case.series = Series {
        t: vec![0.0, 10.0, 20.0],
        channels: vec![
            Channel::of::<sat_units::typed::Power>(
                "generation",
                ChannelKind::Generation,
                vec![60.0, 60.0, 0.0],
            ),
            Channel::of::<sat_units::typed::Power>(
                "load",
                ChannelKind::Load,
                vec![30.0, 30.0, 15.0],
            ),
            Channel::of::<Fraction>("soc", ChannelKind::Soc, vec![1.0, 1.0, 0.8]),
        ],
    };
    case.checks = vec![Check::new("dod", "DoD", Verdict::Ok, "within limit")];
    let out = EnergyBalanceOutput {
        life: SolarLife::Bol,
        mode_loads: sat_analysis::power::mode_load::ModeLoadOutput { modes: vec![loads] },
        dod_limit: Fraction::new(0.4),
        min_allowed_soc: Fraction::new(0.6),
        period: Qty::from_raw(20.0),
        complete_orbits: 1,
        span_fallback: None,
        orbit_average_generation: Qty::from_raw(30.0),
        sunlit_fraction: Fraction::new(0.5),
        eclipses: vec![sat_astro::timeline::Window {
            start: Qty::from_raw(10.0),
            end: Qty::from_raw(20.0),
            duration: Qty::from_raw(10.0),
        }],
        series: Series {
            t: vec![0.0, 10.0, 20.0],
            channels: vec![Channel::of::<Fraction>(
                "sunlit",
                ChannelKind::Sunlit,
                vec![1.0, 0.0, 1.0],
            )],
        },
        cases: vec![case],
        sequence: None,
        checks: Vec::new(),
        shadowing: None,
    };
    let modes = balance(&input, &out);
    let mode = &modes[0];
    assert_eq!(mode.id, "nominal");
    assert_eq!(mode.verdict, Some(Verdict::Ok));
    assert_eq!(mode.charts.len(), 3);
    assert_eq!(mode.charts[0].domain, [0.0, 20.0]);
    assert_eq!(mode.charts[0].series.channels.len(), 2);
    assert_eq!(mode.charts[0].bands[0].start, 0.0);
    assert_eq!(mode.charts[0].bands[0].end, 20.0);
    assert_eq!(mode.charts[0].bands[1].start, 10.0);
    assert!(mode.charts[0].bands[1].eclipse);
    assert_eq!(mode.charts[2].reference, Some(60.0));
    assert!(mode.charts[2].show_time_axis);
    assert!(!mode.charts[0].show_time_axis);
    // 表示名と表示単位は出力の JSON に付く（`fix_channels`）ものと同じ（グラフは型のままの系列から付ける）
    assert_eq!(mode.charts[0].series.channels[0].label, "発電の合計");
    assert_eq!(mode.charts[0].series.channels[0].display_unit, "W");
}

#[test]
fn sizing_keeps_the_analysis_verdicts_per_mode() {
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
    let modes = sizing(&out);
    assert_eq!(modes[0].verdict, Some(Verdict::Violation));
}
