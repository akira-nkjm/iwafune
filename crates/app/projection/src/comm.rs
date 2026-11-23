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

mod gains;
mod throughput;

pub use throughput::data_throughput;

use crate::context::Context;
use crate::missing::MissingInputs;
use crate::quantity::{db_opt, ratio_db, setting, setting_opt};
use crate::timeline;
use fune_model::design::setting_dims as dims;
use fune_model::design::{
    AnalysisSystem, CommLinkSettings, CommSettings, GroundStationSettings, LinkDirection,
};
use fune_model::path::SettingPath;
use fune_model::resolve::ResolvedPart;
use gains::{given_g_over_t, ground_gain_db, required_eb_n0, sat_gain_db};
use sat_analysis::comm::link_budget::{LinkBudgetDirection, LinkBudgetLink};
use sat_analysis::comm::{LinkBudgetInput, RfLinkParams, VisibilityInput};
use sat_astro::ephem::CentralBody;
use sat_astro::ground::GroundStation;
use sat_units::typed::{DataRate, Decibel, Frequency, Length, Power, Qty, Temperature, Time};

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
    let orbit = ctx.orbit(&mut m);
    let height = orbit
        .as_ref()
        .map(|o| (Qty::from_raw(o.altitude_m()), o.central));
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
    // 中心天体が地球でないときだけ、地球から衛星までの距離を求めるための軌道と期間（ミッション期間）を渡す。
    let far = orbit.filter(|o| o.central != CentralBody::Earth);
    let duration = far.as_ref().and_then(|_| mission_duration(ctx));
    m.finish(|| {
        Some(LinkBudgetInput {
            links,
            required_margin_db: None,
            orbit: far,
            duration,
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
    let required = required_eb_n0(name, link, m);
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
        required_margin_db: margin_criterion(ctx, name, &station.id),
        pfd_limit_db: pfd_criterion(ctx, link.direction, name, &station.id),
        pfd_reference_bandwidth: setting_opt::<Frequency>(
            &dims::COMM_PFD_REFERENCE_BANDWIDTH,
            &[name],
            link.pfd_reference_bandwidth.as_ref(),
            m,
        ),
        occupied_bandwidth: setting_opt::<Frequency>(
            &dims::COMM_OCCUPIED_BANDWIDTH,
            &[name],
            link.occupied_bandwidth.as_ref(),
            m,
        ),
    })
}

/// この回線 × 局の要求する回線余裕（`comm.link_budget` の `margin:<回線>@<局>` の基準。要求が無ければ `None` で、既定値は解析が持つ）。
/// 通信の解析の判定のキーの書式はここと解析が持つ（`margin:{回線}@{局}`）。
pub(crate) fn margin_criterion(ctx: &Context<'_>, link: &str, station: &str) -> Option<Decibel> {
    let c = ctx.criterion(
        "comm.link_budget",
        &sat_checkkey::link_margin(link, station),
    )?;
    ratio_db(c.si)
}

/// 地表の電力束密度の限度 [dBW/m²]（下りの `pfd:<回線>@<局>` の基準。要求が無ければ `None`で判定を作らない）。
fn pfd_criterion(
    ctx: &Context<'_>,
    direction: LinkDirection,
    link: &str,
    station: &str,
) -> Option<Decibel> {
    if direction != LinkDirection::Down {
        return None;
    }
    let c = ctx.criterion("comm.link_budget", &sat_checkkey::link_pfd(link, station))?;
    ratio_db(c.si)
}

/// ミッション期間（`input = "mission_duration"` の要求の値。無ければ解析の既定）。
fn mission_duration(ctx: &Context<'_>) -> Option<Qty<Time>> {
    ctx.mission_duration()
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
    let (frequency, rate) = frequency_and_rate(ctx, part, down, m);
    let sat_gain = sat_gain_db(ctx, name, link, part, m);
    let g_over_t = if down {
        given_g_over_t(ground, &at, m)
    } else {
        None
    };
    // G/T を直接与える下りは、地上局の利得と雑音温度を省略できる（それ以外は必須）。
    let direct = g_over_t.is_some();
    let ground_gain = ground_gain_db(ground, &at, direct, m);
    let sat_loss = db_opt(link.line_loss.as_ref(), dims::COMM_LINE_LOSS.at(&[name]), m);
    let ground_loss = db_opt(ground.line_loss.as_ref(), dims::GROUND_LINE_LOSS.at(&at), m);
    let (tx_power, noise) = power_and_noise(ctx, part, ground, &at, (down, direct), m);
    let (sat_gain, ground_gain, noise) = (sat_gain?, ground_gain?, noise?);
    let (tx_gain_db, rx_gain_db, tx_loss_db, rx_loss_db) = if down {
        (Some(sat_gain), ground_gain, sat_loss, ground_loss)
    } else {
        (ground_gain, Some(sat_gain), ground_loss, sat_loss)
    };
    let rf = RfLinkParams {
        frequency: frequency?,
        tx_power: tx_power?,
        tx_gain_db: tx_gain_db?,
        tx_loss_db,
        rx_gain_db,
        rx_loss_db,
        system_noise_temperature: noise,
        rx_g_over_t_dbk: g_over_t,
        atmospheric_loss_db: db_opt(
            link.atmospheric_loss.as_ref(),
            dims::COMM_ATMOSPHERIC_LOSS.at(&[name]),
            m,
        ),
        polarization_loss_db: db_opt(
            link.polarization_loss.as_ref(),
            dims::COMM_POLARIZATION_LOSS.at(&[name]),
            m,
        ),
        pointing_loss_db: db_opt(
            link.pointing_loss.as_ref(),
            dims::COMM_POINTING_LOSS.at(&[name]),
            m,
        ),
        other_loss_db: db_opt(
            link.implementation_loss.as_ref(),
            dims::COMM_IMPLEMENTATION_LOSS.at(&[name]),
            m,
        ),
    };
    Some((rf, rate?))
}

/// 送受信機の周波数とデータレート（下りは `*_down`、上りは `*_up`）。
fn frequency_and_rate(
    ctx: &Context<'_>,
    part: &ResolvedPart,
    down: bool,
    m: &mut MissingInputs,
) -> (Option<Qty<Frequency>>, Option<Qty<DataRate>>) {
    let (freq_attr, rate_attr) = if down {
        ("frequency_down", "data_rate_down")
    } else {
        ("frequency_up", "data_rate_up")
    };
    let why = "Transceiver values";
    (
        crate::parts::part_qty::<Frequency>(ctx, part, freq_attr, why, m),
        crate::parts::part_qty::<DataRate>(ctx, part, rate_attr, why, m),
    )
}

/// 送信電力（送信側）と雑音温度（受信側）。下りは衛星が送信、上りは地上局が送信。`at` は (地上局, 帯域)、
/// `link` は (下りか, G/T を直接与えているか)。G/T を直接与える下りは地上局の雑音温度を省略できる（`Some(None)`）。
fn power_and_noise(
    ctx: &Context<'_>,
    part: &ResolvedPart,
    ground: &fune_model::design::GroundLinkSettings,
    at: &[&str],
    link: (bool, bool),
    m: &mut MissingInputs,
) -> (Option<Qty<Power>>, Option<Option<Qty<Temperature>>>) {
    let why = "Transceiver values";
    let (down, direct) = link;
    if down {
        let noise = ground.noise_temperature.as_ref();
        let power = crate::parts::part_qty::<Power>(ctx, part, "tx_power", why, m);
        let noise = if direct {
            Some(setting_opt(&dims::GROUND_NOISE_TEMPERATURE, at, noise, m))
        } else {
            let need = "Ground station noise temperature";
            setting(&dims::GROUND_NOISE_TEMPERATURE, at, noise, need, m).map(Some)
        };
        (power, noise)
    } else {
        let power = ground.tx_power.as_ref();
        (
            setting(&dims::GROUND_TX_POWER, at, power, "Ground TX power", m),
            crate::parts::part_qty::<Temperature>(ctx, part, "noise_temperature", why, m).map(Some),
        )
    }
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
