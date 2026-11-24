//! 解析ケースの文脈のうち、軌道（`conditions/orbit.toml`。ケースがフェーズを指していて、フェーズが `[orbit]` を持てばそれ）の組み立て。
//! 太陽同期の傾斜角と地方時からの RAAN は、sat-astro の換算を呼び、対応しない天体は不足入力にする。
//! 効いている軌道の設定は [`Context::orbit_settings`] だけが決め、不足入力・読んだものの記録の対象は [`Context::orbit_target`] だけが作る
//! （フェーズの軌道ならフェーズのファイルを指す）。ケプラー要素の軌道（`source = "elements"`）の組み立ては [`Context::orbit`]。
//! 解析に渡す軌道は、出どころ（`source`）で分かれる [`Context::trajectory`]（`context/orbit_source.rs`）が作る。

use super::Context;
use crate::missing::MissingInputs;
use crate::orbit_central::{central_body, design_orbit};
use crate::quantity::{qty, qty_opt};
use fune_model::Target;
use fune_model::design::setting_dims::{self as dims, SettingQuantity};
use fune_model::design::{OrbitSettings, OrbitSource, Phase, PropagatorKind};
use fune_model::path::SettingPath;
use sat_astro::ephem::CentralBody;
use sat_astro::orbit::{DEFAULT_ECCENTRICITY, OrbitElements, OrbitSpec, Propagator};
use sat_astro::time::Epoch;
use sat_units::typed::{Angle, Dimension, Length, Qty};

impl<'a> Context<'a> {
    /// 効いているフェーズ（[`Context::in_phase`] の指定、なければケースが指すフェーズ。ケースにフェーズが無い・フェーズが無ければ None。無い ID は検査が `ref.phase` で知らせる）。
    pub fn phase(&self) -> Option<&'a Phase> {
        if let Some(p) = self.phase_override {
            self.note(Target::phase(p.id.as_str(), None));
            return Some(p);
        }
        let case = self.case?;
        case.phase.as_ref()?;
        self.note(Target::case(case.id.as_str(), Some("phase")));
        crate::case_policy::case_phase(self.design, self.case)
    }

    /// 効いている軌道の設定: ケースのフェーズが `[orbit]` を持てばそれ（設計の軌道を丸ごと置き換える。キーごとの合成はしない）、
    /// なければ設計の `conditions/orbit.toml`。**軌道の設定を読むのはここだけ。**
    pub fn orbit_settings(&self) -> Option<&'a OrbitSettings> {
        match self.phase().and_then(|p| p.orbit.as_ref()) {
            Some(o) => Some(o),
            None => design_orbit(self.design),
        }
    }

    /// 軌道の設定のキーの対象（フェーズの軌道なら `phases.<ID>.orbit.<キー>`、設計の軌道なら `conditions.orbit.<キー>`）。
    pub fn orbit_target(&self, keys: &[&str]) -> Target {
        match self.phase().filter(|p| p.orbit.is_some()) {
            Some(p) => {
                let field = ["orbit"].iter().chain(keys).copied().collect::<Vec<_>>();
                Target::phase(p.id.as_str(), Some(&field.join(".")))
            }
            None => orbit(keys).into(),
        }
    }

    /// 軌道の設定の量の欄 → 対象（設定の次元の表の欄 `conditions.orbit.…` の `conditions.orbit` より後ろを `orbit_target` に渡す）。
    fn orbit_field<D: Dimension>(&self, key: &SettingQuantity<D>) -> Target {
        let keys: Vec<&str> = key.pattern.iter().skip(1).copied().collect();
        self.orbit_target(&keys)
    }

    /// 軌道の設定の量を `Qty<D>` にする（`quantity::setting` の軌道版。不足の対象は [`Context::orbit_target`]）。
    pub(crate) fn orbit_setting<D: Dimension>(
        &self,
        key: &SettingQuantity<D>,
        q: Option<&fune_model::value::Quantity>,
        reason: &str,
        m: &mut MissingInputs,
    ) -> Option<Qty<D>> {
        qty(q, self.orbit_field(key), reason, m)
    }

    /// 軌道の設定のベクトルの量を、次元 `D` の SI の `Vec3` にする（`orbit_setting` のベクトル版）。
    pub(crate) fn orbit_vector<D: Dimension>(
        &self,
        key: &SettingQuantity<D>,
        v: Option<&fune_model::value::Vector3>,
        reason: &str,
        m: &mut MissingInputs,
    ) -> Option<sat_astro::math::Vec3> {
        crate::quantity::vec_qty::<D>(v, self.orbit_field(key), reason, m)
    }

    /// 任意の軌道の設定の量（`quantity::setting_opt` の軌道版）。
    pub(crate) fn orbit_setting_opt<D: Dimension>(
        &self,
        key: &SettingQuantity<D>,
        q: Option<&fune_model::value::Quantity>,
        m: &mut MissingInputs,
    ) -> Option<Qty<D>> {
        qty_opt(q, self.orbit_field(key), m)
    }

    /// 元期（書かれていない・読めなければ不足入力に記録して None）。
    pub(super) fn epoch(&self, o: &OrbitSettings, m: &mut MissingInputs) -> Option<Epoch> {
        match o.epoch.as_deref().map(Epoch::parse) {
            Some(Ok(e)) => Some(e),
            Some(Err(e)) => {
                m.push(
                    self.orbit_target(&["epoch"]),
                    format!("Cannot parse the epoch ({e})"),
                );
                None
            }
            None => {
                m.push(self.orbit_target(&["epoch"]), "Orbit epoch (ISO 8601)");
                None
            }
        }
    }

    /// ケプラー要素の軌道（`conditions/orbit.toml`。ケースのフェーズが `[orbit]` を持てばそれ）。ケプラー要素が本当に要る解析
    /// （季節解析・高度低下・要素を替えるマヌーバの始めの軌道）の入口。**出どころが `elements` でない軌道は対象外**で、
    /// 軌道の `source` を対象に理由を不足入力として返す（直し方の候補は付かない）。
    /// 時系列・通信など、状態・中心天体・元期・繰り返しの長さだけを読む解析は [`Context::trajectory`]。
    pub fn orbit(&self, m: &mut MissingInputs) -> Option<OrbitSpec> {
        let Some(o) = self.orbit_settings() else {
            m.push(
                self.orbit_target(&[]),
                "Orbit settings (conditions/orbit.toml)",
            );
            return None;
        };
        self.note(self.orbit_target(&["source"]));
        if o.source_kind() != OrbitSource::Elements {
            m.push(
                self.orbit_target(&["source"]),
                format!(
                    "Not applicable: this analysis needs an orbit given by Kepler elements, but the orbit in effect has source = \"{}\"",
                    o.source_kind()
                ),
            );
            return None;
        }
        for key in [
            "epoch",
            "central",
            "eccentricity",
            "arg_perigee",
            "mean_anomaly",
        ] {
            self.note(self.orbit_target(&[key]));
        }
        let epoch = self.epoch(o, m);
        let central = o.central.map(central_body).unwrap_or_default();
        let a = self.semi_major_axis(o, central, m);
        self.note(self.orbit_target(&[if o.inclination.is_some() {
            "inclination"
        } else {
            "sun_synchronous"
        }]));
        let inc = match (o.inclination.as_ref(), o.sun_synchronous, a) {
            (Some(q), _, _) => self.orbit_setting(&dims::ORBIT_INCLINATION, Some(q), "", m),
            (None, Some(true), Some(a)) => self.sun_sync_inclination(
                central,
                a.value(),
                o.eccentricity.unwrap_or(DEFAULT_ECCENTRICITY),
                m,
            ),
            _ => self.orbit_setting(
                &dims::ORBIT_INCLINATION,
                None,
                "Inclination (or sun_synchronous = true)",
                m,
            ),
        };
        // 書かれていない要素は None のまま渡す（慣例の値は `OrbitElements::with_defaults` が持つ）。
        // RAAN は LTDN からも求める。外側の None は「求められない」（不足として記録済み）。
        self.note(
            self.orbit_target(&[if o.ltdn.is_some() && o.raan.is_none() {
                "ltdn"
            } else {
                "raan"
            }]),
        );
        let raan = match (o.raan.as_ref(), o.ltdn.as_deref(), epoch) {
            (Some(q), _, _) => self
                .orbit_setting(&dims::ORBIT_RAAN, Some(q), "", m)
                .map(Some),
            (None, Some(text), Some(epoch)) => {
                self.raan_from_ltdn(central, epoch, text, m).map(Some)
            }
            (None, Some(_), None) => None,
            (None, None, _) => Some(None),
        };
        let argp = self.orbit_setting_opt(&dims::ORBIT_ARG_PERIGEE, o.arg_perigee.as_ref(), m);
        let ma = self.orbit_setting_opt(&dims::ORBIT_MEAN_ANOMALY, o.mean_anomaly.as_ref(), m);
        self.note(self.orbit_target(&["propagator"]));
        let propagator = match o.propagator {
            Some(PropagatorKind::Kepler) => Propagator::Kepler,
            Some(PropagatorKind::J2) => Propagator::J2,
            // 数値伝搬は力モデルの既定が無いので使わない（検証が `setting.not_applied` で知らせる）
            Some(PropagatorKind::Numerical) | None => Propagator::default(),
        };
        let elements = OrbitElements::with_defaults(a?, inc?, o.eccentricity, raan?, argp, ma);
        let mut spec = OrbitSpec::new(epoch?, elements, propagator);
        spec.central = central;
        self.with_case_sun_distance(spec, m)
    }

    /// ケースの太陽距離（`sun_distance`）があれば、軌道をその半径の円軌道に置き換える（惑星間の巡航の近日点・遠日点のケース）。
    /// 傾斜角・RAAN・元期はそのまま。太陽を中心天体にしない軌道では使えない（不足入力として記録する）。
    fn with_case_sun_distance(
        &self,
        mut spec: OrbitSpec,
        m: &mut MissingInputs,
    ) -> Option<OrbitSpec> {
        let Some(case) = self.case.filter(|c| c.sun_distance.is_some()) else {
            return Some(spec);
        };
        let target = Target::case(case.id.as_str(), Some("sun_distance"));
        self.note(target.clone());
        let d = qty_opt::<Length>(case.sun_distance.as_ref(), target.clone(), m)?;
        if spec.central != CentralBody::Sun {
            m.push(
                target,
                "The case's sun_distance applies only to an orbit about the Sun (set conditions.orbit.central = \"sun\")",
            );
            return None;
        }
        if d.value() <= spec.central.radius_m() {
            m.push(
                target,
                "The case's sun_distance must be larger than the radius of the Sun",
            );
            return None;
        }
        spec.elements.semi_major_axis = d;
        spec.elements.eccentricity = DEFAULT_ECCENTRICITY;
        Some(spec)
    }

    fn semi_major_axis(
        &self,
        o: &OrbitSettings,
        central: CentralBody,
        m: &mut MissingInputs,
    ) -> Option<Qty<Length>> {
        if o.semi_major_axis.is_some() {
            self.note(self.orbit_target(&["semi_major_axis"]));
            return self.orbit_setting(
                &dims::ORBIT_SEMI_MAJOR_AXIS,
                o.semi_major_axis.as_ref(),
                "",
                m,
            );
        }
        self.note(self.orbit_target(&["altitude"]));
        let alt = self.orbit_setting(
            &dims::ORBIT_ALTITUDE,
            o.altitude.as_ref(),
            "Orbit altitude (or semi_major_axis)",
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

impl Context<'_> {
    /// 太陽同期軌道の傾斜角（換算は sat-astro）。成立しない長半径は不足入力として記録する。
    ///
    /// 地球以外は、sat-astro が返す傾斜角が天体の赤道基準で、`OrbitSpec` の傾斜角（ECI の赤道基準）と
    /// 合わないので使わず、地球の値を黙って入れる代わりに `inclination` を書くよう求める。
    fn sun_sync_inclination(
        &self,
        central: CentralBody,
        a_m: f64,
        e: f64,
        m: &mut MissingInputs,
    ) -> Option<Qty<Angle>> {
        if central != CentralBody::Earth {
            m.push(
            self.orbit_target(&["sun_synchronous"]),
            "Sun-synchronous is supported only for orbits around the Earth (the inclination is relative to the ECI equator). Write inclination directly",
        );
            return None;
        }
        match sat_astro::orbit::sso_inclination_about_rad(central, a_m, e) {
            Ok(rad) => Some(Qty::from_raw(rad)),
            Err(e) => {
                m.push(
                    self.orbit_target(&["sun_synchronous"]),
                    format!("Cannot compute the sun-synchronous inclination ({e})"),
                );
                None
            }
        }
    }

    /// 降交点地方時（`"10:30"`）→ RAAN（読み方は `fune_model::value::parse_clock_hours`、換算は sat-astro）。
    fn raan_from_ltdn(
        &self,
        central: CentralBody,
        epoch: Epoch,
        text: &str,
        m: &mut MissingInputs,
    ) -> Option<Qty<Angle>> {
        let Some(hours) = fune_model::value::parse_clock_hours(text) else {
            m.push(
                self.orbit_target(&["ltdn"]),
                "The local time of the descending node must be written as \"HH:MM\"",
            );
            return None;
        };
        match sat_astro::orbit::raan_from_ltdn_about(central, epoch, hours) {
            Ok(rad) => Some(Qty::from_raw(rad)),
            Err(e) => {
                m.push(
                    self.orbit_target(&["ltdn"]),
                    format!("Cannot convert to RAAN ({e})"),
                );
                None
            }
        }
    }
}
