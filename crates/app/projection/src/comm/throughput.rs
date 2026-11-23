//! `comm.data_throughput` の射影（1 日の送信量。数える下り回線の選び方と、局ごとの回線の条件）。

use super::{CommLinkSettings, CommSettings, comm, rf_params, settings, split_by_band, with_band};
use crate::context::Context;
use crate::missing::MissingInputs;
use crate::quantity::db;
use crate::{cdh, timeline};
use fune_model::Target;
use fune_model::design::LinkDirection;
use sat_analysis::comm::DataThroughputInput;
use sat_analysis::comm::data_throughput::{DataThroughputLink, DataThroughputStationLink};
use sat_analysis::spec::Analysis;
use sat_astro::ground::GroundStation;
use sat_units::typed::{DataRate, Decibel, Qty};

/// `comm.data_throughput`: 1 日分。発生量は `cdh.data_budget` を回して求める。
pub fn data_throughput(ctx: &Context<'_>) -> Result<DataThroughputInput, MissingInputs> {
    let mut m = MissingInputs::new();
    let s = settings(ctx, &mut m);
    let ids = s.map(|s| s.stations.clone()).unwrap_or_default();
    let stations = ctx.stations(&ids, &mut m);
    let sampling = timeline::day_sampling(ctx, &mut m);
    let (down, other_downlinks) = data_downlink(s, &mut m);
    let rate = down.and_then(|(_, l)| {
        let part = ctx.resolved.parts.get(l.part.as_str())?;
        crate::parts::part_qty::<DataRate>(
            ctx,
            part,
            "data_rate_down",
            "Downlink data rate",
            &mut m,
        )
    });
    let (stations, without_band) = match down {
        Some((name, l)) => {
            let (with, without) = split_by_band(ctx, &stations, &l.band);
            if with.is_empty() {
                with_band(ctx, name, &l.band, &stations, &mut m);
            }
            let ids: Vec<String> = without.iter().map(|st| st.id.clone()).collect();
            (with.into_iter().cloned().collect(), ids)
        }
        None => (stations, Vec::new()),
    };
    let station_links = down
        .map(|(name, l)| station_links(ctx, name, l, &stations, &mut m))
        .unwrap_or_default();
    let generated = generated_per_day(ctx, &mut m);
    let required_margin_db = tightest_margin(ctx, down.map(|(n, _)| n.as_str()), &stations);
    m.finish(|| {
        Some(DataThroughputInput {
            sampling: sampling?,
            stations,
            downlink_rate: rate?,
            usable_pass_fraction: None,
            generated_per_day: generated?,
            initial_backlog: None,
            storage_capacity: None,
            station_links,
            stations_without_band: without_band,
            downlink: down.map(|(name, _)| name.clone()),
            other_downlinks,
            required_margin_db,
        })
    })
}

/// 送信量に数える下り回線と、数えない下り回線の名前。`data_downlink` があればその回線、無ければ名前順の先頭。
/// 指定が無い回線・下りでない回線を指していれば不足入力（`check` も誤りにする）。
fn data_downlink<'a>(
    s: Option<&'a CommSettings>,
    m: &mut MissingInputs,
) -> (Option<(&'a String, &'a CommLinkSettings)>, Vec<String>) {
    let downs: Vec<_> = s
        .into_iter()
        .flat_map(|s| s.links.iter())
        .filter(|(_, l)| l.direction == LinkDirection::Down)
        .collect();
    let chosen = match s.and_then(|s| s.data_downlink.as_deref()) {
        Some(name) => m.need(
            downs.iter().find(|(n, _)| n.as_str() == name).copied(),
            comm(&["data_downlink"]),
            "Downlink named by data_downlink (a link with direction = \"down\")",
        ),
        None => m.need(
            downs.first().copied(),
            comm(&["link"]),
            "Downlink (direction = \"down\")",
        ),
    };
    let others = downs
        .iter()
        .filter(|(n, _)| chosen.is_none_or(|(c, _)| c != *n))
        .map(|(n, _)| (*n).clone())
        .collect();
    (chosen, others)
}

/// 下り回線 1 本の、局ごとの回線の条件（地上側の利得・雑音温度・G/T は局ごとに違う）。
fn station_links(
    ctx: &Context<'_>,
    name: &str,
    link: &CommLinkSettings,
    stations: &[GroundStation],
    m: &mut MissingInputs,
) -> Vec<DataThroughputStationLink> {
    stations
        .iter()
        .filter_map(|st| {
            let (rf, _) = rf_params(ctx, name, link, &st.id, m)?;
            Some(DataThroughputStationLink {
                station: st.id.clone(),
                link: DataThroughputLink {
                    rf,
                    required_eb_n0_db: db(link.ebn0_required.as_ref())?,
                },
            })
        })
        .collect()
}

fn generated_per_day(
    ctx: &Context<'_>,
    m: &mut MissingInputs,
) -> Option<Qty<sat_units::typed::Information>> {
    match cdh::data_budget(ctx) {
        Ok(input) => match sat_analysis::cdh::DataBudget::run(&input) {
            Ok(out) => Some(out.total_per_day),
            Err(e) => {
                let target = Target::Analysis {
                    id: sat_analysis::cdh::DataBudget::ID.to_owned(),
                };
                m.push(target, format!("Cannot compute the data volume ({e})"));
                None
            }
        },
        Err(more) => {
            m.extend(more);
            None
        }
    }
}

/// 回線の余裕の要求（送信量に数える下り回線の、数える局ごとの `margin:<回線>@<局>` の基準）のうち最も厳しいもの。
/// この解析は 1 つの値をすべての局に使うので、局ごとの要求が違えば厳しい方に合わせる。要求が無ければ `None`（既定値は解析が持つ）。
fn tightest_margin(
    ctx: &Context<'_>,
    downlink: Option<&str>,
    stations: &[GroundStation],
) -> Option<Decibel> {
    let link = downlink?;
    stations
        .iter()
        .filter_map(|st| super::margin_criterion(ctx, link, &st.id))
        .max_by(|a, b| a.value().total_cmp(&b.value()))
}
