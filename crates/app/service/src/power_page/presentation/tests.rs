use super::*;
use sat_analysis::power::{energy_balance::EnergyBalanceCaseResult, mode_load::ModeLoadMode};
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
fn flow_by_hand_preserves_power_and_proportional_widths() {
    let (loads, case) = fixture();
    let sun = flow::build(
        &loads,
        &case,
        false,
        sat_analysis::power::energy_balance::LoadGrowth::Included,
    );
    let value = |id: &str| {
        sun.nodes
            .iter()
            .find(|n| n.id == id)
            .unwrap()
            .value
            .as_ref()
            .unwrap()
            .si
    };
    assert_eq!(value("source"), 60.0);
    assert_eq!(
        value("cdh") + value("comm") + value("charge") + value("shunt"),
        60.0
    );
    assert_eq!(sun.edges[0].width, 22.0);
    assert_eq!(sun.edges[2].width, sun.edges[1].width * 2.0);
    assert_eq!(sun.nodes[0].label.label_en, "Solar Cells");
    let eclipse = flow::build(
        &loads,
        &case,
        true,
        sat_analysis::power::energy_balance::LoadGrowth::Included,
    );
    assert_eq!(eclipse.nodes[0].label.label_en, "Battery");
    assert_eq!(
        eclipse
            .nodes
            .iter()
            .filter_map(|n| n.value.as_ref())
            .skip(1)
            .map(|v| v.si)
            .sum::<f64>(),
        15.0
    );
    assert!(!eclipse.nodes.iter().any(|n| n.id == "charge"));
}

#[test]
fn bullet_positions_and_missing_orbit_are_explicit() {
    let dod = bullet::<Fraction>(
        "dod",
        label("DoD", "DoD"),
        Some(0.2),
        0.4,
        false,
        Some(Verdict::Ok),
    );
    assert!((dod.marker.unwrap() - 100.0 * 0.2 / 0.48).abs() < 1e-10);
    assert!((dod.band_end - 100.0 * 0.4 / 0.48).abs() < 1e-10);
    assert_eq!(dod.band_start, 0.0);
    let margin = bullet::<Energy>(
        "margin",
        label("収支", "Margin"),
        Some(-10.0),
        0.0,
        true,
        Some(Verdict::Violation),
    );
    assert_eq!(margin.limit_marker, 50.0);
    assert!(margin.marker.unwrap() < margin.band_start);
    let missing = bullet::<Energy>(
        "margin",
        label("収支", "Margin"),
        None,
        0.0,
        true,
        Some(Verdict::Warning),
    );
    assert!(missing.marker.is_none());
    assert!(missing.value.is_none());
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
        },
        batteries: Vec::new(),
        life: SolarLife::Bol,
        loads: Default::default(),
        load_growth: LoadGrowth::Included,
        cases: vec![EnergyBalanceCase {
            id: "nominal".into(),
            label: "Nominal".into(),
            load: EnergyBalanceLoad::Mode {
                mode: "nominal".into(),
            },
            attitude: None,
        }],
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
    };
    let mut json = serde_json::to_value(out).unwrap();
    crate::analysis::channels::fix_channels(&mut json);
    let page = build(
        "power.energy_balance",
        Some(&serde_json::to_value(input).unwrap()),
        Some(&json),
    )
    .unwrap();
    let mode = &page.modes[0];
    assert_eq!(mode.id, "nominal");
    assert_eq!(mode.verdict, Some(Verdict::Ok));
    assert_eq!(mode.bars[0].value.as_ref().unwrap().si, 0.2);
    assert_eq!(mode.bars[1].value.as_ref().unwrap().si, 750.0);
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
    assert!(build("thermal.steady", None, Some(&json)).is_none());
}

#[test]
fn sizing_builder_keeps_design_and_required_values_and_analysis_verdicts() {
    let json = json!({
        "mode_checks":{"nominal":[
            {"key":"sap_margin","label":"Area","verdict":"ok","message":"3 >= 2"},
            {"key":"battery_margin","label":"Capacity","verdict":"violation","message":"100 < 120"}
        ]},
        "required_generation":60,"required_area":2,"required_capacity":120,
        "required_capacity_per_string":120,"design_area":3,"design_capacity":100,
        "sap_margin":0.5,"battery_margin":-0.16666666666666666,"strings":null,
        "area_mode":"nominal","capacity_mode":"nominal","sunlit_load":30,"eclipse_load":15,
        "modes":[{"id":"nominal","sunlit_load":30,"eclipse_load":15,"incidence_factor":1,"required_generation":60,"required_area":2,"required_capacity":120}],
        "sunlit_duration":60,"eclipse_duration":40,
        "longest":{"epoch":"2027-04-01T00:00:00Z","beta":0,"umbra":40,"penumbra":0},
        "used":{"sunlit_efficiency":1,"eclipse_efficiency":1,"battery_efficiency":1,
          "annual_degradation":0,"inherent_degradation":1,"lifetime_years":1,"life_factor":1,
          "dod_max":0.4,"battery_strings":1,"sun_distance":149597870700.0,"panels":[]},
        "checks":[]
    });
    let page = build("power.sizing", None, Some(&json)).unwrap();
    let mode = &page.modes[0];
    assert_eq!(mode.verdict, Some(Verdict::Violation));
    let area = &mode.bars[0];
    assert_eq!(area.value.as_ref().unwrap().si, 3.0);
    assert_eq!(area.limit.si, 2.0);
    assert_eq!(area.verdict, Some(Verdict::Ok));
    assert!(area.marker.unwrap() > area.limit_marker);
    let capacity = &mode.bars[1];
    assert_eq!(capacity.value.as_ref().unwrap().si, 100.0);
    assert_eq!(capacity.limit.si, 120.0);
    assert_eq!(capacity.verdict, Some(Verdict::Violation));
    assert!(capacity.marker.unwrap() < capacity.band_start);
}
