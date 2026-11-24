//! 通信の射影の、回線の利得と所要 Eb/N0 の読み出し。書かれていて読めない値は、黙って無いことにせず、対象を名指しした不足入力にする
//! （任意の値が書かれていなければ、これまでどおり無いまま）。

use super::comm;
use crate::context::Context;
use crate::missing::MissingInputs;
use crate::quantity::{db_need, ratio_db_need, setting_opt};
use fune_model::Target;
use fune_model::design::CommLinkSettings;
use fune_model::design::setting_dims as dims;
use fune_model::resolve::ResolvedPart;
use sat_units::typed::{Decibel, DecibelPerKelvin, Dimensionless};

/// 回線の所要 Eb/N0 [dB]（必須。書かれていて読めないときも、この回線の対象を名指しして不足入力にする）。
pub(super) fn required_eb_n0(
    name: &str,
    link: &CommLinkSettings,
    m: &mut MissingInputs,
) -> Option<Decibel> {
    db_need(
        link.ebn0_required.as_ref(),
        dims::COMM_EBN0_REQUIRED.at(&[name]),
        "Required Eb/N0 (dB)",
        m,
    )
}

/// 地上局のアンテナ利得 [dBi]。G/T を直接与える下り（`direct`）では、書かれていなければ省略できる（`Some(None)`）。
/// 書かれていて読めないときは省略せず、不足入力に記録する。
pub(super) fn ground_gain_db(
    ground: &fune_model::design::GroundLinkSettings,
    at: &[&str],
    direct: bool,
    m: &mut MissingInputs,
) -> Option<Option<Decibel>> {
    let target = dims::GROUND_ANTENNA_GAIN.at(at);
    match (ground.antenna_gain.as_ref(), direct) {
        (None, true) => Some(None),
        (None, false) => {
            m.push(target, "Ground station antenna gain (dBi)");
            None
        }
        (Some(_), _) => db_need(
            ground.antenna_gain.as_ref(),
            target,
            "Ground station antenna gain (dBi)",
            m,
        )
        .map(Some),
    }
}

/// 地上局が与えた G/T [dB/K]（`g_over_t`。単位は `dB`。あれば解析は利得・雑音温度から求めずにこれを使う）。
pub(super) fn given_g_over_t(
    ground: &fune_model::design::GroundLinkSettings,
    at: &[&str],
    m: &mut MissingInputs,
) -> Option<DecibelPerKelvin> {
    let ratio =
        setting_opt::<Dimensionless>(&dims::GROUND_G_OVER_T, at, ground.g_over_t.as_ref(), m)?;
    ratio_db_need(
        ratio.value(),
        dims::GROUND_G_OVER_T.at(at),
        "Ground station G/T",
        m,
    )
    .map(|d| DecibelPerKelvin::new(d.value()))
}

/// Spacecraft antenna gain [dB]（回線の `antenna` の部品、無ければ送受信機の `antenna_gain`。
/// 解決済みの値は線形の比なので dB に戻す）。
pub(super) fn sat_gain_db(
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
    let target = Target::attr(&part.id, "antenna_gain");
    let ratio = m.need(
        ctx.scalar(part, "antenna_gain"),
        target.clone(),
        "Spacecraft antenna gain",
    )?;
    ratio_db_need(ratio, target, "Spacecraft antenna gain", m)
}
