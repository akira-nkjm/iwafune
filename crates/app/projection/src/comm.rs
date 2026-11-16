//! 通信の射影: `comm.link_budget`・`comm.visibility`・`comm.data_throughput`。回線の計算は sat-analysis に任せる。
//!
//! - 回線余裕は「設定の回線 × 設定の地上局」のうち、その回線の帯域の設定（`conditions.ground.<局>.link.<帯域>`）を持つ局との組を渡す
//!   （回線が地上局を指定する設定は無いので、帯域を持つ局すべてに当てる。帯域を持たない局は組にしない。
//!   どの局にもその帯域が無ければ不足入力。帯域の表にあるが必要な値が無いときも不足入力）。
//! - 送信量に数える下り回線は名前順の先頭の 1 本（どの回線で送るかを決める設定が無い）。ほかの下り回線は名前だけ渡し、
//!   解析が「数えていない」と判定（Ok）で知らせる。局ごとの回線の条件は `station_links` に、下りの帯域を持つ局の分を渡す。
//!   その帯域を持たない局は `stations_without_band` に ID を渡し、数えない（結果の `station_closures` に残る）。
//! - 地上局の `g_over_t` があれば、下りの受信側の G/T として渡す（どちらを使うかは解析が決める）。

use crate::context::Context;
use crate::missing::MissingInputs;
use crate::quantity::{db, ratio_db, setting, setting_opt};
use crate::{cdh, timeline};
use fune_model::Target;
use fune_model::design::setting_dims as dims;
use fune_model::design::{
    AnalysisSystem, CommLinkSettings, CommSettings, GroundStationSettings, LinkDirection,
};
use fune_model::path::SettingPath;
use fune_model::resolve::ResolvedPart;
use sat_analysis::comm::data_throughput::{DataThroughputLink, DataThroughputStationLink};
use sat_analysis::comm::link_budget::{LinkBudgetDirection, LinkBudgetLink};
use sat_analysis::comm::{DataThroughputInput, LinkBudgetInput, RfLinkParams, VisibilityInput};
use sat_analysis::spec::Analysis;
use sat_astro::ephem::CentralBody;
use sat_astro::ground::GroundStation;
use sat_units::typed::{
    DataRate, Decibel, DecibelPerKelvin, Dimensionless, Frequency, Length, Power, Qty, Temperature,
};

fn settings<'a>(ctx: &'a Context<'_>, m: &mut MissingInputs) -> Option<&'a CommSettings> {
    let s = ctx.design.analysis.comm.as_ref();
    m.need(s, comm(&[]), "通信の解析の設定（analysis/comm.toml）")
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
            m.push(comm(&["link"]), "回線の設定（[link.<名前>]）");
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
                "どの地上局にも帯域 {band} の設定（conditions.ground.<局>.link.{band}）がありません"
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
        "所要 Eb/N0（dB）",
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
        "回線の送受信機（部品 ID）",
    )?;
    let ground = ground_link(ctx, station_id, &link.band, m)?;
    let at = [station_id, link.band.as_str()];
    let down = link.direction == LinkDirection::Down;
    let (freq_attr, rate_attr) = if down {
        ("frequency_down", "data_rate_down")
    } else {
        ("frequency_up", "data_rate_up")
    };
    let why = "送受信機の値";
    let frequency = crate::parts::part_qty::<Frequency>(ctx, part, freq_attr, why, m);
    let rate = crate::parts::part_qty::<DataRate>(ctx, part, rate_attr, why, m);
    let sat_gain = sat_gain_db(ctx, part, m);
    let ground_gain = m.need(
        db(ground.antenna_gain.as_ref()),
        dims::GROUND_ANTENNA_GAIN.at(&at),
        "地上局のアンテナ利得（dBi）",
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
    let why = "送受信機の値";
    if down {
        let noise = ground.noise_temperature.as_ref();
        (
            crate::parts::part_qty::<Power>(ctx, part, "tx_power", why, m),
            setting(
                &dims::GROUND_NOISE_TEMPERATURE,
                at,
                noise,
                "地上局の雑音温度",
                m,
            ),
        )
    } else {
        let power = ground.tx_power.as_ref();
        (
            setting(&dims::GROUND_TX_POWER, at, power, "地上局の送信電力", m),
            crate::parts::part_qty::<Temperature>(ctx, part, "noise_temperature", why, m),
        )
    }
}

/// 衛星側のアンテナ利得 [dB]（送受信機の `antenna_gain`。解決済みの値は線形の比なので dB に戻す）。
fn sat_gain_db(ctx: &Context<'_>, part: &ResolvedPart, m: &mut MissingInputs) -> Option<Decibel> {
    let ratio = m.need(
        ctx.scalar(part, "antenna_gain"),
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
    let g: Option<&GroundStationSettings> = ctx.design.conditions.ground.get(station);
    let l = g.and_then(|g| g.links.get(band));
    m.need(l, ground_band(station, band), "地上局のこの帯域の性能")
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

/// `comm.data_throughput`: 1 日分。発生量は `cdh.data_budget` を回して求める。
pub fn data_throughput(ctx: &Context<'_>) -> Result<DataThroughputInput, MissingInputs> {
    let mut m = MissingInputs::new();
    let s = settings(ctx, &mut m);
    let ids = s.map(|s| s.stations.clone()).unwrap_or_default();
    let stations = ctx.stations(&ids, &mut m);
    let sampling = timeline::day_sampling(ctx, &mut m);
    let mut downs = s.into_iter().flat_map(|s| {
        s.links
            .iter()
            .filter(|(_, l)| l.direction == LinkDirection::Down)
    });
    let down = m.need(
        downs.next(),
        comm(&["link"]),
        "下りの回線（direction = \"down\"）",
    );
    let other_downlinks: Vec<String> = downs.map(|(name, _)| name.clone()).collect();
    let rate = down.and_then(|(_, l)| {
        let part = ctx.resolved.parts.get(l.part.as_str())?;
        crate::parts::part_qty::<DataRate>(
            ctx,
            part,
            "data_rate_down",
            "下りのデータレート",
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
            required_margin_db: s.and_then(|s| db(s.min_margin.as_ref())),
        })
    })
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
