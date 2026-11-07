//! 解析ケースの文脈: 設計・解決済みの値・ケースと、どの解析でも使う入力（軌道・地上局・姿勢則・モード）の組み立て。

use crate::missing::MissingInputs;
use crate::quantity::{qty, qty_opt};
use fune_model::design::{AttitudeLawKind, AttitudeLawSetting, Case, Design, Mode};
use fune_model::resolve::Resolved;
use fune_model::value::{Quaternion, Vector3};
use sat_analysis::modes::{ModeDef, StateAssignment};
use sat_astro::attitude::{AttitudeLaw, DEFAULT_PRIMARY_AXIS, DEFAULT_SECONDARY_AXIS};
use sat_astro::ground::GroundStation;
use sat_astro::math::{Quat, Vec3};
use sat_astro::orbit::{DEFAULT_ECCENTRICITY, OrbitElements, OrbitSpec, Propagator};
use sat_astro::time::Epoch;
use sat_units::typed::{Angle, Length, Qty};

/// 射影の文脈。
pub struct Context<'a> {
    pub design: &'a Design,
    pub resolved: &'a Resolved,
    /// 選んだ解析ケース（無ければ既定）
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
                    m.push(format!("cases.{id}"), "指定した解析ケースがありません");
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
        self.design.modes.values().map(mode_def).collect()
    }

    /// 軌道（`mission/orbit.toml`）。
    pub fn orbit(&self, m: &mut MissingInputs) -> Option<OrbitSpec> {
        let Some(o) = self.design.mission.orbit.as_ref() else {
            m.push("mission.orbit", "軌道の設定（mission/orbit.toml）");
            return None;
        };
        let epoch = match o.epoch.as_deref().map(Epoch::parse) {
            Some(Ok(e)) => Some(e),
            Some(Err(e)) => {
                m.push(
                    "mission.orbit.epoch",
                    format!("元期を解釈できません（{e}）"),
                );
                None
            }
            None => {
                m.push("mission.orbit.epoch", "軌道の元期（ISO 8601）");
                None
            }
        };
        let a = self.semi_major_axis(o, m);
        let inc = match (o.inclination.as_ref(), o.sun_synchronous, a) {
            (Some(q), _, _) => qty::<Angle>(Some(q), "mission.orbit.inclination", "", m),
            (None, Some(true), Some(a)) => {
                Some(Qty::from_raw(sat_astro::orbit::sso_inclination_rad(
                    a.value(),
                    o.eccentricity.unwrap_or(DEFAULT_ECCENTRICITY),
                )))
            }
            _ => qty::<Angle>(
                None,
                "mission.orbit.inclination",
                "傾斜角（または sun_synchronous = true）",
                m,
            ),
        };
        // 書かれていない要素は None のまま渡す（慣例の値は `OrbitElements::with_defaults` が持つ）。
        // RAAN は LTDN からも求める。外側の None は「求められない」（不足として記録済み）。
        let raan = match (o.raan.as_ref(), o.ltdn.as_deref(), epoch) {
            (Some(q), _, _) => qty::<Angle>(Some(q), "mission.orbit.raan", "", m).map(Some),
            (None, Some(text), Some(epoch)) => raan_from_ltdn(epoch, text, m).map(Some),
            (None, Some(_), None) => None,
            (None, None, _) => Some(None),
        };
        let argp = qty_opt::<Angle>(o.arg_perigee.as_ref(), "mission.orbit.arg_perigee", m);
        let ma = qty_opt::<Angle>(o.mean_anomaly.as_ref(), "mission.orbit.mean_anomaly", m);
        let propagator = match o.propagator {
            Some(fune_model::design::PropagatorKind::Kepler) => Propagator::Kepler,
            _ => Propagator::default(),
        };
        let elements = OrbitElements::with_defaults(a?, inc?, o.eccentricity, raan?, argp, ma);
        Some(OrbitSpec::new(epoch?, elements, propagator))
    }

    fn semi_major_axis(
        &self,
        o: &fune_model::design::OrbitSettings,
        m: &mut MissingInputs,
    ) -> Option<Qty<Length>> {
        if o.semi_major_axis.is_some() {
            return qty::<Length>(
                o.semi_major_axis.as_ref(),
                "mission.orbit.semi_major_axis",
                "",
                m,
            );
        }
        let alt = qty::<Length>(
            o.altitude.as_ref(),
            "mission.orbit.altitude",
            "軌道の高度（または semi_major_axis）",
            m,
        )?;
        Some(Qty::from_raw(
            sat_astro::orbit::semi_major_axis_from_altitude(
                sat_astro::ephem::CentralBody::Earth,
                alt.value(),
            ),
        ))
    }

    /// 地上局（`ids` が空なら全局）。
    pub fn stations(&self, ids: &[String], m: &mut MissingInputs) -> Vec<GroundStation> {
        let ground = &self.design.mission.ground;
        if ground.is_empty() {
            m.push("mission.ground", "地上局（mission/ground/<ID>.toml）");
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
                    m.push(format!("mission.ground.{id}"), "地上局の設定がありません");
                    return None;
                };
                let p = |k: &str| format!("mission.ground.{id}.{k}");
                // 標高と最低仰角は書かれていなければ None（慣例の値は `GroundStation::with_defaults`）。
                Some(GroundStation::with_defaults(
                    id.clone(),
                    qty::<Angle>(g.latitude.as_ref(), &p("latitude"), "緯度", m)?,
                    qty::<Angle>(g.longitude.as_ref(), &p("longitude"), "経度", m)?,
                    qty_opt::<Length>(g.altitude.as_ref(), &p("altitude"), m),
                    qty_opt::<Angle>(g.min_elevation.as_ref(), &p("min_elevation"), m),
                ))
            })
            .collect()
    }

    /// モードの姿勢則（`analysis/attitude.toml` の `[law.<モード>]`）。無ければ不足入力。
    pub fn attitude_law(&self, mode: Option<&str>, m: &mut MissingInputs) -> Option<AttitudeLaw> {
        let laws = self.design.analysis.attitude.as_ref().map(|a| &a.law);
        let found = laws.and_then(|l| mode.and_then(|id| l.get(id)).or_else(|| l.values().next()));
        let Some(setting) = found else {
            let key = mode.unwrap_or("<モード>");
            m.push(format!("analysis.attitude.law.{key}"), "モードの姿勢則");
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
    ModeDef {
        id: mode.id.to_string(),
        label: mode.label.clone().unwrap_or_else(|| mode.id.to_string()),
        assignments: mode
            .states
            .iter()
            .map(|(part, a)| StateAssignment {
                part: part.to_string(),
                state: a.state.clone(),
                duty: a.duty.clone(),
            })
            .collect(),
    }
}

/// 降交点地方時（`"10:30"`）→ RAAN（読み方は `fune_model::value::parse_clock_hours`、換算は sat-astro）。
fn raan_from_ltdn(epoch: Epoch, text: &str, m: &mut MissingInputs) -> Option<Qty<Angle>> {
    let Some(hours) = fune_model::value::parse_clock_hours(text) else {
        m.push(
            "mission.orbit.ltdn",
            "降交点地方時は \"HH:MM\" で書いてください",
        );
        return None;
    };
    match sat_astro::orbit::raan_from_ltdn(epoch, hours) {
        Ok(rad) => Some(Qty::from_raw(rad)),
        Err(e) => {
            m.push(
                "mission.orbit.ltdn",
                format!("RAAN に換算できません（{e}）"),
            );
            None
        }
    }
}
