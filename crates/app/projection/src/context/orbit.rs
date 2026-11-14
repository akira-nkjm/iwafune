//! 解析ケースの文脈のうち、軌道（`conditions/orbit.toml`）の組み立て。
//! 太陽同期の傾斜角と地方時からの RAAN は、sat-astro の換算を呼び、対応しない天体は不足入力にする。

use super::{Context, central_body};
use crate::missing::MissingInputs;
use crate::quantity::{setting, setting_opt};
use fune_model::design::PropagatorKind;
use fune_model::design::setting_dims as dims;
use fune_model::path::SettingPath;
use sat_astro::ephem::CentralBody;
use sat_astro::orbit::{DEFAULT_ECCENTRICITY, OrbitElements, OrbitSpec, Propagator};
use sat_astro::time::Epoch;
use sat_units::typed::{Angle, Length, Qty};

impl<'a> Context<'a> {
    /// 軌道（`conditions/orbit.toml`）。
    pub fn orbit(&self, m: &mut MissingInputs) -> Option<OrbitSpec> {
        let Some(o) = self.design.conditions.orbit.as_ref() else {
            m.push(orbit(&[]), "軌道の設定（conditions/orbit.toml）");
            return None;
        };
        for key in [
            "epoch",
            "central",
            "eccentricity",
            "arg_perigee",
            "mean_anomaly",
        ] {
            self.note(orbit(&[key]));
        }
        let epoch = match o.epoch.as_deref().map(Epoch::parse) {
            Some(Ok(e)) => Some(e),
            Some(Err(e)) => {
                m.push(orbit(&["epoch"]), format!("元期を解釈できません（{e}）"));
                None
            }
            None => {
                m.push(orbit(&["epoch"]), "軌道の元期（ISO 8601）");
                None
            }
        };
        let central = o.central.map_or(CentralBody::Earth, central_body);
        let a = self.semi_major_axis(o, central, m);
        self.note(orbit(&[if o.inclination.is_some() {
            "inclination"
        } else {
            "sun_synchronous"
        }]));
        let inc = match (o.inclination.as_ref(), o.sun_synchronous, a) {
            (Some(q), _, _) => setting(&dims::ORBIT_INCLINATION, &[], Some(q), "", m),
            (None, Some(true), Some(a)) => sun_sync_inclination(
                central,
                a.value(),
                o.eccentricity.unwrap_or(DEFAULT_ECCENTRICITY),
                m,
            ),
            _ => setting(
                &dims::ORBIT_INCLINATION,
                &[],
                None,
                "傾斜角（または sun_synchronous = true）",
                m,
            ),
        };
        // 書かれていない要素は None のまま渡す（慣例の値は `OrbitElements::with_defaults` が持つ）。
        // RAAN は LTDN からも求める。外側の None は「求められない」（不足として記録済み）。
        self.note(orbit(&[if o.ltdn.is_some() && o.raan.is_none() {
            "ltdn"
        } else {
            "raan"
        }]));
        let raan = match (o.raan.as_ref(), o.ltdn.as_deref(), epoch) {
            (Some(q), _, _) => setting(&dims::ORBIT_RAAN, &[], Some(q), "", m).map(Some),
            (None, Some(text), Some(epoch)) => raan_from_ltdn(central, epoch, text, m).map(Some),
            (None, Some(_), None) => None,
            (None, None, _) => Some(None),
        };
        let argp = setting_opt(&dims::ORBIT_ARG_PERIGEE, &[], o.arg_perigee.as_ref(), m);
        let ma = setting_opt(&dims::ORBIT_MEAN_ANOMALY, &[], o.mean_anomaly.as_ref(), m);
        self.note(orbit(&["propagator"]));
        let propagator = match o.propagator {
            Some(PropagatorKind::Kepler) => Propagator::Kepler,
            Some(PropagatorKind::J2) => Propagator::J2,
            // 数値伝搬は力モデルの既定が無いので使わない（検証が `setting.not_applied` で知らせる）
            Some(PropagatorKind::Numerical) | None => Propagator::default(),
        };
        let elements = OrbitElements::with_defaults(a?, inc?, o.eccentricity, raan?, argp, ma);
        let mut spec = OrbitSpec::new(epoch?, elements, propagator);
        spec.central = central;
        Some(spec)
    }

    fn semi_major_axis(
        &self,
        o: &fune_model::design::OrbitSettings,
        central: CentralBody,
        m: &mut MissingInputs,
    ) -> Option<Qty<Length>> {
        if o.semi_major_axis.is_some() {
            self.note(orbit(&["semi_major_axis"]));
            return setting(
                &dims::ORBIT_SEMI_MAJOR_AXIS,
                &[],
                o.semi_major_axis.as_ref(),
                "",
                m,
            );
        }
        self.note(orbit(&["altitude"]));
        let alt = setting(
            &dims::ORBIT_ALTITUDE,
            &[],
            o.altitude.as_ref(),
            "軌道の高度（または semi_major_axis）",
            m,
        )?;
        Some(Qty::from_raw(
            sat_astro::orbit::semi_major_axis_from_altitude(central, alt.value()),
        ))
    }
}

/// `conditions.orbit.<…>`（`conditions/orbit.toml` のキー）。
fn orbit(keys: &[&str]) -> SettingPath {
    SettingPath::conditions(&["orbit"]).child(keys)
}

/// 太陽同期軌道の傾斜角（換算は sat-astro）。成立しない長半径は不足入力として記録する。
///
/// 地球以外は、sat-astro が返す傾斜角が天体の赤道基準で、`OrbitSpec` の傾斜角（ECI の赤道基準）と
/// 合わないので使わず、地球の値を黙って入れる代わりに `inclination` を書くよう求める。
fn sun_sync_inclination(
    central: CentralBody,
    a_m: f64,
    e: f64,
    m: &mut MissingInputs,
) -> Option<Qty<Angle>> {
    if central != CentralBody::Earth {
        m.push(
            orbit(&["sun_synchronous"]),
            "太陽同期は地球を回る軌道だけに対応しています（傾斜角が ECI の赤道基準のため）。inclination を直接書いてください",
        );
        return None;
    }
    match sat_astro::orbit::sso_inclination_about_rad(central, a_m, e) {
        Ok(rad) => Some(Qty::from_raw(rad)),
        Err(e) => {
            m.push(
                orbit(&["sun_synchronous"]),
                format!("太陽同期の傾斜角を求められません（{e}）"),
            );
            None
        }
    }
}

/// 降交点地方時（`"10:30"`）→ RAAN（読み方は `fune_model::value::parse_clock_hours`、換算は sat-astro）。
fn raan_from_ltdn(
    central: CentralBody,
    epoch: Epoch,
    text: &str,
    m: &mut MissingInputs,
) -> Option<Qty<Angle>> {
    let Some(hours) = fune_model::value::parse_clock_hours(text) else {
        m.push(
            orbit(&["ltdn"]),
            "降交点地方時は \"HH:MM\" で書いてください",
        );
        return None;
    };
    match sat_astro::orbit::raan_from_ltdn_about(central, epoch, hours) {
        Ok(rad) => Some(Qty::from_raw(rad)),
        Err(e) => {
            m.push(orbit(&["ltdn"]), format!("RAAN に換算できません（{e}）"));
            None
        }
    }
}
