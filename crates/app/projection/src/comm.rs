//! 通信の射影: `comm.link_budget`・`comm.visibility`・`comm.data_throughput`。回線の計算は sat-analysis に任せる。

use crate::context::Context;
use crate::missing::MissingInputs;
use crate::quantity::{db, qty, ratio_db};
use crate::{cdh, timeline};
use fune_model::Target;
use fune_model::design::{CommLinkSettings, CommSettings, GroundStationSettings, LinkDirection};
use fune_model::path::SettingPath;
use fune_model::resolve::ResolvedPart;
use sat_analysis::comm::data_throughput::DataThroughputLink;
use sat_analysis::comm::link_budget::{LinkBudgetDirection, LinkBudgetLink};
use sat_analysis::comm::{DataThroughputInput, LinkBudgetInput, RfLinkParams, VisibilityInput};
use sat_analysis::spec::Analysis;
use sat_astro::ephem::CentralBody;
use sat_astro::ground::GroundStation;
use sat_units::typed::{DataRate, Decibel, Frequency, Length, Power, Qty, Temperature};

fn settings<'a>(ctx: &'a Context<'_>, m: &mut MissingInputs) -> Option<&'a CommSettings> {
    let s = ctx.design.analysis.comm.as_ref();
    m.need(s, comm(&[]), "通信の解析の設定（analysis/comm.toml）")
}

/// `analysis.comm.<…>`（`analysis/comm.toml` のキー）。
fn comm(keys: &[&str]) -> SettingPath {
    SettingPath::analysis(&["comm"]).child(keys)
}

/// 地上局のある帯域の性能（`mission.ground.<局>.link.<帯域>`）。
fn ground_band(station: &str, band: &str) -> SettingPath {
    SettingPath::mission(&["ground", station, "link", band])
}

/// `comm.link_budget`: 設定の回線 × 使う地上局の先頭。
pub fn link_budget(ctx: &Context<'_>) -> Result<LinkBudgetInput, MissingInputs> {
    let mut m = MissingInputs::new();
    let s = settings(ctx, &mut m);
    let stations = ctx.stations(s.map(|s| s.stations.as_slice()).unwrap_or(&[]), &mut m);
    let height = orbit_height(ctx, &mut m);
    let mut links = Vec::new();
    if let (Some(s), Some(station)) = (s, stations.first()) {
        if s.links.is_empty() {
            m.push(comm(&["link"]), "回線の設定（[link.<名前>]）");
        }
        for (name, link) in &s.links {
            if let Some(l) = budget_link(ctx, name, link, station, height, &mut m) {
                links.push(l);
            }
        }
    }
    let required_margin_db = s.and_then(|s| db(s.min_margin.as_ref()));
    m.finish(|| {
        Some(LinkBudgetInput {
            links,
            required_margin_db,
        })
    })
}

fn budget_link(
    ctx: &Context<'_>,
    name: &str,
    link: &CommLinkSettings,
    station: &GroundStation,
    height: Option<(Qty<Length>, CentralBody)>,
    m: &mut MissingInputs,
) -> Option<LinkBudgetLink> {
    let (rf, data_rate) = rf_params(ctx, name, link, &station.id, m)?;
    let required = m.need(
        db(link.ebn0_required.as_ref()),
        comm(&["link", name, "ebn0_required"]),
        "所要 Eb/N0（dB）",
    );
    Some(LinkBudgetLink {
        name: name.to_owned(),
        direction: match link.direction {
            LinkDirection::Down => LinkBudgetDirection::Downlink,
            LinkDirection::Up => LinkBudgetDirection::Uplink,
        },
        rf,
        data_rate,
        required_eb_n0_db: required?,
        altitude: height?.0,
        central: height?.1,
        elevation: station.min_elevation,
    })
}

/// 回線 1 本の RF パラメーターとデータレート（下りは衛星 → 地上局、上りは地上局 → 衛星）。
fn rf_params(
    ctx: &Context<'_>,
    name: &str,
    link: &CommLinkSettings,
    station_id: &str,
    m: &mut MissingInputs,
) -> Option<(RfLinkParams, Qty<DataRate>)> {
    let part = ctx.resolved.parts.get(link.part.as_str());
    let part = m.need(
        part,
        comm(&["link", name, "part"]),
        "回線の送受信機（部品 ID）",
    )?;
    let ground = ground_link(ctx, station_id, &link.band, m)?;
    let gp = ground_band(station_id, &link.band);
    let down = link.direction == LinkDirection::Down;
    let (freq_attr, rate_attr) = if down {
        ("frequency_down", "data_rate_down")
    } else {
        ("frequency_up", "data_rate_up")
    };
    let why = "送受信機の値";
    let frequency = crate::parts::part_qty::<Frequency>(part, freq_attr, why, m);
    let rate = crate::parts::part_qty::<DataRate>(part, rate_attr, why, m);
    let sat_gain = sat_gain_db(part, m);
    let ground_gain = m.need(
        db(ground.antenna_gain.as_ref()),
        gp.child(&["antenna_gain"]),
        "地上局のアンテナ利得（dBi）",
    );
    let sat_loss = db(link.line_loss.as_ref());
    let ground_loss = db(ground.line_loss.as_ref());
    let (tx_power, noise) = power_and_noise(part, ground, &gp, down, m);
    let (sat_gain, ground_gain) = (sat_gain?, ground_gain?);
    let (tx_gain_db, rx_gain_db, tx_loss_db, rx_loss_db) = if down {
        (sat_gain, ground_gain, sat_loss, ground_loss)
    } else {
        (ground_gain, sat_gain, ground_loss, sat_loss)
    };
    let rf = RfLinkParams {
        frequency: frequency?,
        tx_power: tx_power?,
        tx_gain_db,
        tx_loss_db,
        rx_gain_db,
        rx_loss_db,
        system_noise_temperature: noise?,
        atmospheric_loss_db: db(link.atmospheric_loss.as_ref()),
        polarization_loss_db: db(link.polarization_loss.as_ref()),
        pointing_loss_db: db(link.pointing_loss.as_ref()),
        other_loss_db: db(link.implementation_loss.as_ref()),
    };
    Some((rf, rate?))
}

/// 送信電力（送信側）と雑音温度（受信側）。下りは衛星が送信、上りは地上局が送信。
fn power_and_noise(
    part: &ResolvedPart,
    ground: &fune_model::design::GroundLinkSettings,
    gp: &SettingPath,
    down: bool,
    m: &mut MissingInputs,
) -> (Option<Qty<Power>>, Option<Qty<Temperature>>) {
    let why = "送受信機の値";
    if down {
        let noise = ground.noise_temperature.as_ref();
        (
            crate::parts::part_qty::<Power>(part, "tx_power", why, m),
            qty::<Temperature>(
                noise,
                gp.child(&["noise_temperature"]),
                "地上局の雑音温度",
                m,
            ),
        )
    } else {
        let power = ground.tx_power.as_ref();
        (
            qty::<Power>(power, gp.child(&["tx_power"]), "地上局の送信電力", m),
            crate::parts::part_qty::<Temperature>(part, "noise_temperature", why, m),
        )
    }
}

/// 衛星側のアンテナ利得 [dB]（送受信機の `antenna_gain`。解決済みの値は線形の比なので dB に戻す）。
fn sat_gain_db(part: &ResolvedPart, m: &mut MissingInputs) -> Option<Decibel> {
    let ratio = m.need(
        part.scalar("antenna_gain"),
        Target::attr(&part.id, "antenna_gain"),
        "衛星側のアンテナ利得",
    );
    ratio.and_then(ratio_db)
}

fn ground_link<'a>(
    ctx: &'a Context<'_>,
    station: &str,
    band: &str,
    m: &mut MissingInputs,
) -> Option<&'a fune_model::design::GroundLinkSettings> {
    let g: Option<&GroundStationSettings> = ctx.design.mission.ground.get(station);
    let l = g.and_then(|g| g.links.get(band));
    m.need(l, ground_band(station, band), "地上局のこの帯域の性能")
}

/// 円軌道の高度（`mission.orbit.altitude`、無ければ長半径 − 中心天体の半径）と中心天体（斜距離に半径を使う）。
fn orbit_height(ctx: &Context<'_>, m: &mut MissingInputs) -> Option<(Qty<Length>, CentralBody)> {
    let orbit = ctx.orbit(m)?;
    Some((Qty::from_raw(orbit.altitude_m()), orbit.central))
}

/// `comm.visibility`: 1 日分。
pub fn visibility(ctx: &Context<'_>) -> Result<VisibilityInput, MissingInputs> {
    let mut m = MissingInputs::new();
    let ids = ctx
        .design
        .analysis
        .comm
        .as_ref()
        .map(|s| s.stations.clone())
        .unwrap_or_default();
    let stations = ctx.stations(&ids, &mut m);
    let sampling = timeline::day_sampling(ctx, &mut m);
    m.finish(|| {
        Some(VisibilityInput {
            sampling: sampling?,
            stations,
        })
    })
}

/// `comm.data_throughput`: 1 日分。発生量は `cdh.data_budget` を回して求める。
pub fn data_throughput(ctx: &Context<'_>) -> Result<DataThroughputInput, MissingInputs> {
    let mut m = MissingInputs::new();
    let s = settings(ctx, &mut m);
    let ids = s.map(|s| s.stations.clone()).unwrap_or_default();
    let stations = ctx.stations(&ids, &mut m);
    let sampling = timeline::day_sampling(ctx, &mut m);
    let down = s.and_then(|s| {
        s.links
            .iter()
            .find(|(_, l)| l.direction == LinkDirection::Down)
    });
    let down = m.need(down, comm(&["link"]), "下りの回線（direction = \"down\"）");
    let rate = down.and_then(|(_, l)| {
        let part = ctx.resolved.parts.get(l.part.as_str())?;
        crate::parts::part_qty::<DataRate>(part, "data_rate_down", "下りのデータレート", &mut m)
    });
    let link = match (down, stations.first()) {
        (Some((name, l)), Some(st)) => {
            rf_params(ctx, name, l, &st.id, &mut m).and_then(|(rf, _)| {
                Some(DataThroughputLink {
                    rf,
                    required_eb_n0_db: db(l.ebn0_required.as_ref())?,
                })
            })
        }
        _ => None,
    };
    let generated = generated_per_day(ctx, &mut m);
    m.finish(|| {
        Some(DataThroughputInput {
            sampling: sampling?,
            stations,
            downlink_rate: rate?,
            usable_pass_fraction: None,
            generated_per_day: generated?,
            initial_backlog: None,
            storage_capacity: None,
            link,
            required_margin_db: s.and_then(|s| db(s.min_margin.as_ref())),
        })
    })
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
                m.push(target, format!("データ量を求められません（{e}）"));
                None
            }
        },
        Err(more) => {
            m.extend(more);
            None
        }
    }
}
