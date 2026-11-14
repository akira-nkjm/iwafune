//! 解析ケースの文脈: 設計・解決済みの値・ケースと、どの解析でも使う入力（軌道・地上局・姿勢則・モード）の組み立て。

use crate::missing::MissingInputs;
use crate::quantity::{setting, setting_opt};
use fune_model::Target;
use fune_model::design::CentralBodyKind;
use fune_model::design::setting_dims as dims;
use fune_model::design::{AnalysisSystem, AttitudeLawKind, AttitudeLawSetting, Case, Design, Mode};
use fune_model::path::SettingPath;
use fune_model::resolve::Resolved;
use fune_model::value::{Quaternion, Vector3};
use sat_analysis::modes::{ModeDef, StateAssignment};
use sat_astro::attitude::{AttitudeLaw, DEFAULT_PRIMARY_AXIS, DEFAULT_SECONDARY_AXIS};
use sat_astro::ephem::CentralBody;
use sat_astro::ground::GroundStation;
use sat_astro::math::{Quat, Vec3};
use sat_astro::orbit::{DEFAULT_ECCENTRICITY, OrbitElements, OrbitSpec, Propagator};
use sat_astro::time::Epoch;
use sat_units::typed::{Angle, Length, Qty};

/// 設計の中心天体 → sat-astro の中心天体（μ・半径・暦・環境の値は sat-astro と解析が持つ）。
pub fn central_body(c: CentralBodyKind) -> CentralBody {
    match c {
        CentralBodyKind::Earth => CentralBody::Earth,
        CentralBodyKind::Moon => CentralBody::Moon,
        CentralBodyKind::Sun => CentralBody::Sun,
        CentralBodyKind::Mercury => CentralBody::Mercury,
        CentralBodyKind::Venus => CentralBody::Venus,
        CentralBodyKind::Mars => CentralBody::Mars,
        CentralBodyKind::Jupiter => CentralBody::Jupiter,
        CentralBodyKind::Saturn => CentralBody::Saturn,
        CentralBodyKind::Uranus => CentralBody::Uranus,
        CentralBodyKind::Neptune => CentralBody::Neptune,
    }
}

/// 射影の文脈。
pub struct Context<'a> {
    pub design: &'a Design,
    pub resolved: &'a Resolved,
    /// 選んだ解析ケース（無ければケース未選択）
    pub case: Option<&'a Case>,
}

impl<'a> Context<'a> {
    /// ケースの ID から文脈を作る。ケースが無ければ不足入力。
    pub fn new(
        design: &'a Design,
        resolved: &'a Resolved,
        case: Option<&str>,
    ) -> Result<Self, MissingInputs> {
        let case = match case {
            None => None,
            Some(id) => match design.cases.get(id) {
                Some(c) => Some(c),
                None => {
                    let mut m = MissingInputs::new();
                    m.push(Target::case(id, None), "指定した解析ケースがありません");
                    return Err(m);
                }
            },
        };
        Ok(Self {
            design,
            resolved,
            case,
        })
    }

    /// ケースが指すモード（無ければ None）。
    pub fn case_mode(&self) -> Option<&'a Mode> {
        let id = self.case?.mode.as_ref()?;
        self.design.modes.get(id.as_str())
    }

    /// 全モードを解析の形に。
    pub fn modes(&self) -> Vec<ModeDef> {
        self.design
            .ordered_modes()
            .into_iter()
            .map(mode_def)
            .collect()
    }

    /// 軌道（`conditions/orbit.toml`）。
    pub fn orbit(&self, m: &mut MissingInputs) -> Option<OrbitSpec> {
        let Some(o) = self.design.conditions.orbit.as_ref() else {
            m.push(orbit(&[]), "軌道の設定（conditions/orbit.toml）");
            return None;
        };
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
        let inc = match (o.inclination.as_ref(), o.sun_synchronous, a) {
            (Some(q), _, _) => setting(&dims::ORBIT_INCLINATION, &[], Some(q), "", m),
            (None, Some(true), Some(a)) => {
                Some(Qty::from_raw(sat_astro::orbit::sso_inclination_rad(
                    a.value(),
                    o.eccentricity.unwrap_or(DEFAULT_ECCENTRICITY),
                )))
            }
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
        let raan = match (o.raan.as_ref(), o.ltdn.as_deref(), epoch) {
            (Some(q), _, _) => setting(&dims::ORBIT_RAAN, &[], Some(q), "", m).map(Some),
            (None, Some(text), Some(epoch)) => raan_from_ltdn(epoch, text, m).map(Some),
            (None, Some(_), None) => None,
            (None, None, _) => Some(None),
        };
        let argp = setting_opt(&dims::ORBIT_ARG_PERIGEE, &[], o.arg_perigee.as_ref(), m);
        let ma = setting_opt(&dims::ORBIT_MEAN_ANOMALY, &[], o.mean_anomaly.as_ref(), m);
        let propagator = match o.propagator {
            Some(fune_model::design::PropagatorKind::Kepler) => Propagator::Kepler,
            _ => Propagator::default(),
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
            return setting(
                &dims::ORBIT_SEMI_MAJOR_AXIS,
                &[],
                o.semi_major_axis.as_ref(),
                "",
                m,
            );
        }
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

    /// 地上局（`ids` が空なら全局）。
    pub fn stations(&self, ids: &[String], m: &mut MissingInputs) -> Vec<GroundStation> {
        let ground = &self.design.conditions.ground;
        if ground.is_empty() {
            m.push(
                SettingPath::conditions(&["ground"]),
                "地上局（conditions/ground/<ID>.toml）",
            );
        }
        let chosen: Vec<&String> = if ids.is_empty() {
            ground.keys().collect()
        } else {
            ids.iter().collect()
        };
        chosen
            .into_iter()
            .filter_map(|id| {
                let Some(g) = ground.get(id) else {
                    let target = SettingPath::conditions(&["ground", id]);
                    m.push(target, "地上局の設定がありません");
                    return None;
                };
                let at = [id.as_str()];
                // 標高と最低仰角は書かれていなければ None（慣例の値は `GroundStation::with_defaults`）。
                Some(GroundStation::with_defaults(
                    id.clone(),
                    setting(&dims::GROUND_LATITUDE, &at, g.latitude.as_ref(), "緯度", m)?,
                    setting(
                        &dims::GROUND_LONGITUDE,
                        &at,
                        g.longitude.as_ref(),
                        "経度",
                        m,
                    )?,
                    setting_opt(&dims::GROUND_ALTITUDE, &at, g.altitude.as_ref(), m),
                    setting_opt(
                        &dims::GROUND_MIN_ELEVATION,
                        &at,
                        g.min_elevation.as_ref(),
                        m,
                    ),
                ))
            })
            .collect()
    }

    /// 指定モードの姿勢則。モード未指定なら唯一の姿勢則だけを使い、選べなければ不足入力。
    pub fn attitude_law(&self, mode: Option<&str>, m: &mut MissingInputs) -> Option<AttitudeLaw> {
        let laws = self.design.analysis.aocs.as_ref().map(|a| &a.law);
        let found = laws.and_then(|l| match mode {
            Some(id) => l.get(id),
            None if l.len() == 1 => l.values().next(),
            None => None,
        });
        let Some(setting) = found else {
            // モード未指定なら姿勢則の表を、指定済みならそのモードの欄を示す。
            let law = AnalysisSystem::Aocs.setting_path().child(&["law"]);
            let target = mode.map_or_else(|| law.clone(), |id| law.child(&[id]));
            m.push(
                target,
                "モードの姿勢則を一意に選べません。対象モードの姿勢則を指定してください",
            );
            return None;
        };
        Some(law_of(setting))
    }
}

pub(crate) fn law_of(s: &AttitudeLawSetting) -> AttitudeLaw {
    let v = |x: &Option<Vector3>, d: Vec3| x.as_ref().map_or(d, vec3);
    match s.law {
        AttitudeLawKind::SunPointing => AttitudeLaw::SunPointing {
            axis: v(&s.axis, DEFAULT_PRIMARY_AXIS),
            secondary: v(&s.secondary, DEFAULT_SECONDARY_AXIS),
        },
        AttitudeLawKind::NadirPointing => AttitudeLaw::NadirPointing {
            axis: v(&s.axis, DEFAULT_PRIMARY_AXIS),
            along_track: v(&s.along_track, DEFAULT_SECONDARY_AXIS),
        },
        AttitudeLawKind::Inertial => AttitudeLaw::Inertial {
            q_body_to_eci: s.orientation.as_ref().map_or(Quat::IDENTITY, quat),
        },
    }
}

/// 設計のベクトル（数値そのまま）→ `Vec3`。単位付きは呼び出し側で SI にする。
pub fn vec3(v: &Vector3) -> Vec3 {
    Vec3::new(v.x, v.y, v.z)
}

/// 設計のクォータニオン → `Quat`。
pub fn quat(q: &Quaternion) -> Quat {
    Quat {
        w: q.w,
        x: q.x,
        y: q.y,
        z: q.z,
    }
}

/// 設計のモード → 解析のモード。
pub fn mode_def(mode: &Mode) -> ModeDef {
    let assignments = |rows: &std::collections::BTreeMap<_, fune_model::design::StateAssign>| {
        rows.iter()
            .map(
                |(part, a): (&fune_model::id::Id<fune_model::design::Part>, _)| StateAssignment {
                    part: part.to_string(),
                    state: a.state.clone(),
                    duty: a.duty.clone(),
                    active: a.active,
                    spare_state: a.spare_state.clone(),
                },
            )
            .collect()
    };
    ModeDef {
        id: mode.id.to_string(),
        label: mode.label.clone().unwrap_or_else(|| mode.id.to_string()),
        assignments: assignments(&mode.states),
        eclipse: assignments(&mode.eclipse),
    }
}

/// `conditions.orbit.<…>`（`conditions/orbit.toml` のキー）。
fn orbit(keys: &[&str]) -> SettingPath {
    SettingPath::conditions(&["orbit"]).child(keys)
}

/// 降交点地方時（`"10:30"`）→ RAAN（読み方は `fune_model::value::parse_clock_hours`、換算は sat-astro）。
fn raan_from_ltdn(epoch: Epoch, text: &str, m: &mut MissingInputs) -> Option<Qty<Angle>> {
    let Some(hours) = fune_model::value::parse_clock_hours(text) else {
        m.push(
            orbit(&["ltdn"]),
            "降交点地方時は \"HH:MM\" で書いてください",
        );
        return None;
    };
    match sat_astro::orbit::raan_from_ltdn(epoch, hours) {
        Ok(rad) => Some(Qty::from_raw(rad)),
        Err(e) => {
            m.push(orbit(&["ltdn"]), format!("RAAN に換算できません（{e}）"));
            None
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 中心天体の綴り（fune-model）と sat-astro の綴りが一致する。
    #[test]
    fn central_body_spellings_match_sat_astro() {
        for &kind in CentralBodyKind::ALL {
            let model = serde_json::to_value(kind).unwrap();
            assert_eq!(model, serde_json::to_value(central_body(kind)).unwrap());
        }
    }
}
