//! 解析ケースの文脈: 設計・解決済みの値・ケースと、どの解析でも使う入力（軌道・地上局・姿勢則・モード）の組み立て。

mod orbit;

use crate::missing::MissingInputs;
use crate::quantity::{setting, setting_opt};
use crate::reads::ReadLog;
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

/// 射影の文脈。部品の属性・設定・モード・ケースの読み出しは、読んだものを記録できるよう文脈の入口（部品の属性は
/// `parts` の `Context::attr`・`scalar`・`vec3`、設定は `setting`、ほかは `orbit`・`modes` など。記録は `note`）を通す。
pub struct Context<'a> {
    pub design: &'a Design,
    pub resolved: &'a Resolved,
    /// 選んだ解析ケース（無ければケース未選択）
    pub case: Option<&'a Case>,
    /// 読んだものの記録先（無ければ記録しない）
    reads: Option<&'a ReadLog>,
}

impl<'a> Context<'a> {
    /// ケースを選ばない文脈。
    pub fn bare(design: &'a Design, resolved: &'a Resolved) -> Self {
        Self {
            design,
            resolved,
            case: None,
            reads: None,
        }
    }

    /// 読んだものを `reads` に記録する文脈にする。
    pub fn with_reads(mut self, reads: Option<&'a ReadLog>) -> Self {
        self.reads = reads;
        self
    }

    /// 記録先（同じ記録先で別のケースの文脈を作るとき）。
    pub fn reads(&self) -> Option<&'a ReadLog> {
        self.reads
    }

    /// 読んだ対象を記録する。**読み出しの記録はすべてここを通る。**
    pub fn note(&self, target: impl Into<Target>) {
        if let Some(log) = self.reads {
            log.push(target.into());
        }
    }

    /// 設定の値を読む（`path` を記録して `value` をそのまま返す）。
    pub fn setting<T>(&self, path: SettingPath, value: T) -> T {
        self.note(path);
        value
    }

    /// 表示順のモード（読んだものとして各モードを記録する）。
    pub fn ordered_modes(&self) -> Vec<&'a Mode> {
        let modes = self.design.ordered_modes();
        for m in &modes {
            self.note(Target::mode(m.id.as_str(), None));
        }
        modes
    }

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
            reads: None,
        })
    }

    /// ケースが指すモード（無ければ None）。
    pub fn case_mode(&self) -> Option<&'a Mode> {
        let case = self.case?;
        let id = case.mode.as_ref()?;
        self.note(Target::case(case.id.as_str(), Some("mode")));
        self.design.modes.get(id.as_str())
    }

    /// 全モードを解析の形に。
    pub fn modes(&self) -> Vec<ModeDef> {
        self.ordered_modes().into_iter().map(mode_def).collect()
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
                self.note(SettingPath::conditions(&["ground", id]));
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
        let law = AnalysisSystem::Aocs.setting_path().child(&["law"]);
        let key = match (mode, found.is_some()) {
            (Some(id), _) => Some(id),
            (None, true) => laws.and_then(|l| l.keys().next()).map(String::as_str),
            (None, false) => None,
        };
        self.note(key.map_or_else(|| law.clone(), |id| law.child(&[id])));
        let Some(setting) = found else {
            // モード未指定なら姿勢則の表を、指定済みならそのモードの欄を示す。
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
