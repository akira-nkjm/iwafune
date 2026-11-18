//! グラフのチャンネルと共通横軸・時間帯の表示定義。
use super::label;
use crate::analysis::channels::display_series;
use crate::dto::*;
use sat_analysis::power::{
    energy_balance::EnergyBalanceOutput, orbit_generation::OrbitGenerationOutput,
};

/// 電力収支のケース `index`（モード `mode`）のグラフ。
pub(super) fn build(out: &EnergyBalanceOutput, index: usize, mode: &str) -> Vec<PowerChartView> {
    let Some(case) = out.cases.get(index) else {
        return Vec::new();
    };
    let series = display_series(&case.series);
    let domain = [
        series.t.first().copied().unwrap_or(0.0),
        series.t.last().copied().unwrap_or(0.0),
    ];
    let mut bands = vec![PowerChartBand {
        start: domain[0],
        end: domain[1],
        eclipse: false,
        mode_index: index,
        label: mode.into(),
    }];
    bands.extend(out.eclipses.iter().map(|w| PowerChartBand {
        start: w.start.value(),
        end: w.end.value(),
        eclipse: true,
        mode_index: index,
        label: String::new(),
    }));
    [
        (
            "power",
            label("発電と負荷", "Generation and load"),
            vec!["generation", "load"],
            false,
        ),
        (
            "battery",
            label("充放電とシャント", "Charge, discharge and shunt"),
            vec!["charge_power", "discharge_power", "shunt_power"],
            false,
        ),
        (
            "soc",
            label("充電率（SOC）", "State of charge (SOC)"),
            vec!["soc"],
            true,
        ),
    ]
    .into_iter()
    .map(|(id, label, keys, soc)| {
        let mut channels: Vec<_> = series
            .channels
            .iter()
            .filter(|c| {
                keys.contains(&c.channel.key.as_str())
                    || (id == "power"
                        && c.channel.kind == sat_analysis::spec::ChannelKind::PanelGeneration)
            })
            .cloned()
            .collect();
        if let Some(first) = channels.first() {
            let peak = channels
                .iter()
                .flat_map(|c| &c.channel.values)
                .fold(0.0_f64, |m, v| m.max(v.abs()));
            if let Some((unit, scale)) =
                crate::analysis::channels::linear_unit(peak, &first.channel.dimension)
            {
                for channel in &mut channels {
                    channel.display_unit = sat_units::format::unit_label(&unit);
                    channel.display_scale = scale;
                }
            }
        }
        let reference = if soc {
            channels
                .first()
                .map(|c| out.min_allowed_soc.value() * c.display_scale)
        } else {
            None
        };
        PowerChartView {
            id: id.into(),
            label,
            series: DisplaySeries {
                t: series.t.clone(),
                channels,
            },
            domain,
            show_time_axis: soc,
            bands: bands.clone(),
            reference,
            reference_label: soc.then(|| super::label("許容 DoD", "Allowed DoD")),
        }
    })
    .collect()
}

/// 電力収支が未実行でも、軌道発電の結果だけで発電のグラフを表示できる。
pub(super) fn orbital(out: &OrbitGenerationOutput) -> PowerPageView {
    let mut series = display_series(&out.series);
    series.channels.retain(|c| c.channel.key != "sunlit");
    let domain = [
        series.t.first().copied().unwrap_or(0.0),
        series.t.last().copied().unwrap_or(0.0),
    ];
    let mut bands: Vec<_> = out
        .eclipses
        .iter()
        .map(|w| PowerChartBand {
            start: w.start.value(),
            end: w.end.value(),
            eclipse: true,
            mode_index: 0,
            label: String::new(),
        })
        .collect();
    bands.insert(
        0,
        PowerChartBand {
            start: domain[0],
            end: domain[1],
            eclipse: false,
            mode_index: 0,
            label: String::new(),
        },
    );
    PowerPageView {
        modes: vec![PowerModeView {
            id: "orbit".into(),
            label: match out.life {
                sat_analysis::power::solar::SolarLife::Bol => "BOL",
                sat_analysis::power::solar::SolarLife::Eol => "EOL",
            }
            .into(),
            verdict: None,
            sunlit: None,
            eclipse: None,
            charts: vec![PowerChartView {
                id: "generation".into(),
                label: label("太陽電池の発電", "Solar Cells generation"),
                series,
                domain,
                show_time_axis: true,
                bands,
                reference: None,
                reference_label: None,
            }],
        }],
        ..Default::default()
    }
}
