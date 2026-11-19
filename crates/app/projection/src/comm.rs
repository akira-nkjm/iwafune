//! 通信の射影: `comm.link_budget`・`comm.visibility`・`comm.data_throughput`。回線の計算は sat-analysis に任せる。
//!
//! - 回線余裕は「設定の回線 × 設定の地上局」のうち、その回線の帯域の設定（`conditions.ground.<局>.link.<帯域>`）を持つ局との組を渡す
//!   （回線が地上局を指定する設定は無いので、帯域を持つ局すべてに当てる。帯域を持たない局は組にしない。
//!   どの局にもその帯域が無ければ不足入力。帯域の表にあるが必要な値が無いときも不足入力）。
//! - 送信量に数える下り回線は `data_downlink` で選んだ 1 本（無ければ名前順の先頭の 1 本）。ほかの下り回線は名前だけ渡し、
//!   解析が「数えていない」と判定（Ok）で知らせる。局ごとの回線の条件は `station_links` に、下りの帯域を持つ局の分を渡す。
//!   その帯域を持たない局は `stations_without_band` に ID を渡し、数えない（結果の `station_closures` に残る）。
//! - 衛星側のアンテナ利得は、回線の `antenna`（部品 ID）があればその部品の `antenna_gain`、無ければ送受信機の `antenna_gain`。
//! - 地上局の `g_over_t` があれば、下りの受信側の G/T として渡す（どちらを使うかは解析が決める）。

mod throughput;

pub use throughput::data_throughput;

use crate::context::Context;
use crate::missing::MissingInputs;
use crate::quantity::{db, ratio_db, setting, setting_opt};
use crate::timeline;
use fune_model::Target;
use fune_model::design::setting_dims as dims;
use fune_model::design::{
    AnalysisSystem, CommLinkSettings, CommSettings, GroundStationSettings, LinkDirection,
};
use fune_model::path::SettingPath;
use fune_model::resolve::ResolvedPart;
use sat_analysis::comm::link_budget::{LinkBudgetDirection, LinkBudgetLink};
use sat_analysis::comm::{LinkBudgetInput, RfLinkParams, VisibilityInput};
use sat_astro::ephem::CentralBody;
use sat_astro::ground::GroundStation;
use sat_units::typed::{
    DataRate, Decibel, DecibelPerKelvin, Dimensionless, Frequency, Length, Power, Qty, Temperature,
};

fn settings<'a>(ctx: &'a Context<'_>, m: &mut MissingInputs) -> Option<&'a CommSettings> {
    let s = ctx.design.analysis.comm.as_ref();
    m.need(s, comm(&[]), "Comm analysis settings (analysis/comm.toml)")
}

/// `analysis.comm.<…>`（`analysis/comm.toml` のキー）。
fn comm(keys: &[&str]) -> SettingPath {
    AnalysisSystem::Comm.setting_path().child(keys)
}

/// 地上局のある帯域の性能（`conditions.ground.<局>.link.<帯域>`）。
fn ground_band(station: &str, band: &str) -> SettingPath {
    SettingPath::conditions(&["ground", station, "link", band])
}

/// `comm.link_budget`: 設定の回線 × 使う地上局のすべて。
pub fn link_budget(ctx: &Context<'_>) -> Result<LinkBudgetInput, MissingInputs> {
    let mut m = MissingInputs::new();
    let s = settings(ctx, &mut m);
    let stations = ctx.stations(s.map(|s| s.stations.as_slice()).unwrap_or(&[]), &mut m);
    let height = orbit_height(ctx, &mut m);
    let mut links = Vec::new();
    if let Some(s) = s {
        if s.links.is_empty() {
            m.push(comm(&["link"]), "Link settings ([link.<name>])");
        }
        for (name, link) in &s.links {
            for station in with_band(ctx, name, &link.band, &stations, &mut m) {
                if let Some(l) = budget_link(ctx, name, link, station, height, &mut m) {
                    links.push(l);
                }
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

/// 回線の帯域の設定を持つ局（持たない局は組にしない）。どの局にも無ければ不足入力に記録する（局が無いときは局の不足で足りる）。
fn with_band<'a>(
    ctx: &Context<'_>,
    link: &str,
    band: &str,
    stations: &'a [GroundStation],
    m: &mut MissingInputs,
) -> Vec<&'a GroundStation> {
    let (with, _) = split_by_band(ctx, stations, band);
    if with.is_empty() && !stations.is_empty() {
        m.push(
            comm(&["link", link, "band"]),
            format!(
                "No ground station has settings for band {band} (conditions.ground.<station>.link.{band})"
            ),
        );
    }
    with
}

/// 局を、帯域の設定を持つ局と持たない局に分ける（順序は保つ）。
fn split_by_band<'a>(
    ctx: &Context<'_>,
    stations: &'a [GroundStation],
    band: &str,
) -> (Vec<&'a GroundStation>, Vec<&'a GroundStation>) {
    stations.iter().partition(|st| {
        ctx.design
            .conditions
            .ground
            .get(&st.id)
            .is_some_and(|g| g.links.contains_key(band))
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
        dims::COMM_EBN0_REQUIRED.at(&[name]),
        "Required Eb/N0 (dB)",
    );
    Some(LinkBudgetLink {
        name: name.to_owned(),
        station: station.id.clone(),
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
        "Link transceiver (part id)",
    )?;
    let ground = ground_link(ctx, station_id, &link.band, m)?;
    let at = [station_id, link.band.as_str()];
    let down = link.direction == LinkDirection::Down;
    let (freq_attr, rate_attr) = if down {
        ("frequency_down", "data_rate_down")
    } else {
        ("frequency_up", "data_rate_up")
    };
    let why = "Transceiver values";
    let frequency = crate::parts::part_qty::<Frequency>(ctx, part, freq_attr, why, m);
    let rate = crate::parts::part_qty::<DataRate>(ctx, part, rate_attr, why, m);
    let sat_gain = sat_gain_db(ctx, name, link, part, m);
    let ground_gain = m.need(
        db(ground.antenna_gain.as_ref()),
        dims::GROUND_ANTENNA_GAIN.at(&at),
        "Ground station antenna gain (dBi)",
    );
    let sat_loss = db(link.line_loss.as_ref());
    let ground_loss = db(ground.line_loss.as_ref());
    let (tx_power, noise) = power_and_noise(ctx, part, ground, &at, down, m);
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
        rx_g_over_t_dbk: if down {
            given_g_over_t(ground, &at, m)
        } else {
            None
        },
        atmospheric_loss_db: db(link.atmospheric_loss.as_ref()),
        polarization_loss_db: db(link.polarization_loss.as_ref()),
        pointing_loss_db: db(link.pointing_loss.as_ref()),
        other_loss_db: db(link.implementation_loss.as_ref()),
    };
    Some((rf, rate?))
}

/// 地上局が与えた G/T [dB/K]（`g_over_t`。単位は `dB`。あれば解析は利得・雑音温度から求めずにこれを使う）。
fn given_g_over_t(
    ground: &fune_model::design::GroundLinkSettings,
    at: &[&str],
    m: &mut MissingInputs,
) -> Option<DecibelPerKelvin> {
    let ratio =
        setting_opt::<Dimensionless>(&dims::GROUND_G_OVER_T, at, ground.g_over_t.as_ref(), m)?;
    ratio_db(ratio.value()).map(|d| DecibelPerKelvin::new(d.value()))
}

/// 送信電力（送信側）と雑音温度（受信側）。下りは衛星が送信、上りは地上局が送信。`at` は (地上局, 帯域)。
fn power_and_noise(
    ctx: &Context<'_>,
    part: &ResolvedPart,
    ground: &fune_model::design::GroundLinkSettings,
    at: &[&str],
    down: bool,
    m: &mut MissingInputs,
) -> (Option<Qty<Power>>, Option<Qty<Temperature>>) {
    let why = "Transceiver values";
    if down {
        let noise = ground.noise_temperature.as_ref();
        (
            crate::parts::part_qty::<Power>(ctx, part, "tx_power", why, m),
            setting(
                &dims::GROUND_NOISE_TEMPERATURE,
                at,
                noise,
                "Ground station noise temperature",
                m,
            ),
        )
    } else {
        let power = ground.tx_power.as_ref();
        (
            setting(&dims::GROUND_TX_POWER, at, power, "Ground TX power", m),
            crate::parts::part_qty::<Temperature>(ctx, part, "noise_temperature", why, m),
        )
    }
}

/// Spacecraft antenna gain [dB]（回線の `antenna` の部品、無ければ送受信機の `antenna_gain`。
/// 解決済みの値は線形の比なので dB に戻す）。
fn sat_gain_db(
    ctx: &Context<'_>,
    name: &str,
    link: &CommLinkSettings,
    transceiver: &ResolvedPart,
    m: &mut MissingInputs,
) -> Option<Decibel> {
    let part = match &link.antenna {
        Some(id) => m.need(
            ctx.resolved.parts.get(id.as_str()),
            comm(&["link", name, "antenna"]),
            "Link antenna (part id)",
        )?,
        None => transceiver,
    };
    let ratio = m.need(
        ctx.scalar(part, "antenna_gain"),
        Target::attr(&part.id, "antenna_gain"),
        "Spacecraft antenna gain",
    );
    ratio.and_then(ratio_db)
}

fn ground_link<'a>(
    ctx: &'a Context<'_>,
    station: &str,
    band: &str,
    m: &mut MissingInputs,
) -> Option<&'a fune_model::design::GroundLinkSettings> {
    let g: Option<&GroundStationSettings> = ctx.design.conditions.ground.get(station);
    let l = g.and_then(|g| g.links.get(band));
    m.need(l, ground_band(station, band), "Ground band performance")
}

/// 円軌道の高度（`conditions.orbit.altitude`、無ければ長半径 − 中心天体の半径）と中心天体（斜距離に半径を使う）。
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
