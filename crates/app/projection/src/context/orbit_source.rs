//! 軌道の出どころ（`source`）ごとの組み立て。**効いている軌道を解析に渡す `Trajectory` にするのは [`Context::trajectory`] だけ**
//! （出どころの分岐はここだけ。解析の入力を作る側は軌道の出どころを知らない）。
//!
//! - `elements`: ケプラー要素の軌道（[`Context::orbit`]）。数値は出どころを足す前と同じ。
//! - `transfer`: 日付つきの遷移のマヌーバ（`maneuver`）の巡航の軌道。元期は出発の天体の影響圏を出る時刻、中心は共通の親、期間は到着の天体の影響圏に入るまで（影響圏の中の双曲線の弧は解析しない）。
//!   日付の読み方は `maneuver_dates::parse`、巡航の区間は `sat_astro::transfer::cruise_leg`（Lambert の弧 `cruise_orbit` を影響圏の外に切り出す）（`propulsion.maneuvers` の
//!   日付つきの遷移と同じ `lambert_arc`）。親のまわりの二体の軌道（接触要素）で、期間（`Trajectory::with_span`）が 1 回限りにする。
//!   遷移の軌道が楕円でないときは不足入力として理由を返す（状態ベクトルの出どころが受け持つ）。
//!
//! - `state`: 状態ベクトル（`epoch`・`central`・`position`・`velocity`・`span`）。`sat_astro::orbit::StateSpec`（既存の数値伝搬。
//!   力の設定は書かせず、原点の天体で決まる慣例の既定 `ForceConfig::deep_space_defaults`）。向きは ECI。期間は 1 回限り（`Cycle::OneOff`）。
//!
//! ケプラー要素が本当に要る解析は [`Context::orbit`] を使い、出どころが `elements` でなければ対象外になる。

use super::Context;
use crate::maneuver_dates;
use crate::missing::MissingInputs;
use crate::orbit_central::central_body;
use fune_model::Target;
use fune_model::design::setting_dims as dims;
use fune_model::design::{Maneuver, OrbitSettings, OrbitSource};
use sat_astro::ephem::CentralBody;
use sat_astro::orbit::{OrbitState, StateSpec, Trajectory};
use sat_astro::time::Epoch;
use sat_units::typed::Qty;

impl Context<'_> {
    /// 効いている軌道の出どころ（軌道の設定が無ければ `elements`）。
    pub fn orbit_source(&self) -> OrbitSource {
        self.orbit_settings()
            .map_or(OrbitSource::Elements, OrbitSettings::source_kind)
    }

    /// 解析に渡す軌道（時刻での状態・中心天体・元期・繰り返しの長さ）。出どころごとに組み立てる。
    /// 作れなければ不足入力に記録して `None`。ケプラー要素が要る解析は [`Context::orbit`]。
    pub fn trajectory(&self, m: &mut MissingInputs) -> Option<Trajectory> {
        match self.orbit_settings() {
            Some(o) if o.source_kind() == OrbitSource::Transfer => self.transfer_trajectory(o, m),
            Some(o) if o.source_kind() == OrbitSource::State => self.state_trajectory(o, m),
            _ => self.orbit(m).map(Trajectory::Elements),
        }
    }

    /// `source = "state"`: 元期・原点の天体・位置・速度・期間から、既存の数値伝搬で伝搬する軌道。足りない欄・読めない欄は不足入力に記録する。
    fn state_trajectory(&self, o: &OrbitSettings, m: &mut MissingInputs) -> Option<Trajectory> {
        for key in ["source", "epoch", "central", "position", "velocity", "span"] {
            self.note(self.orbit_target(&[key]));
        }
        let overrides = self.reject_case_overrides(m);
        let epoch = self.epoch(o, m);
        let central = o.central.map(central_body).unwrap_or_default();
        let r = self.orbit_vector::<sat_units::typed::Length>(
            &dims::ORBIT_POSITION,
            o.position.as_ref(),
            "Position of the state orbit (x, y, z with a length unit; from the centre of the origin body, in the J2000 mean-equator inertial frame)",
            m,
        );
        let v = self.orbit_vector::<sat_units::typed::Velocity>(
            &dims::ORBIT_VELOCITY,
            o.velocity.as_ref(),
            "Velocity of the state orbit (x, y, z with a velocity unit; same frame as the position)",
            m,
        );
        let span: Option<Qty<sat_units::typed::Time>> = self.orbit_setting(
            &dims::ORBIT_SPAN,
            o.span.as_ref(),
            "Span of the state orbit (the duration from the epoch that the analyses evaluate)",
            m,
        );
        let state = OrbitState {
            r_eci: r?,
            v_eci: v?,
        };
        let spec = StateSpec::with_defaults(epoch?, central, state, span?.value(), None);
        if let Err(e) = spec.validate() {
            m.push(
                self.orbit_target(&["position"]),
                format!("Cannot use the state orbit ({e})"),
            );
            return None;
        }
        overrides.then_some(Trajectory::State(spec))
    }

    /// `source = "transfer"`: マヌーバの日付つきの遷移の巡航の軌道。
    fn transfer_trajectory(&self, o: &OrbitSettings, m: &mut MissingInputs) -> Option<Trajectory> {
        self.note(self.orbit_target(&["source"]));
        self.note(self.orbit_target(&["maneuver"]));
        let overrides = self.reject_case_overrides(m);
        let maneuver = self.transfer_maneuver(o, m)?;
        let id = maneuver.id.as_str();
        let dates = match maneuver_dates::parse(maneuver) {
            Some(Ok(dates)) => dates,
            Some(Err((key, why))) => {
                m.push(Target::maneuver(id, Some(&format!("transfer.{key}"))), why);
                return None;
            }
            None => {
                m.push(
                    self.orbit_target(&["maneuver"]),
                    format!("Maneuver {id} has no dated [transfer] (departure and arrival) to take an orbit from"),
                );
                return None;
            }
        };
        let to = self.maneuver_target_body(maneuver, m)?;
        let from = maneuver_dates::central_before(self.design, maneuver);
        let orbit = self.cruise_orbit(maneuver, (from, to), dates, m)?;
        overrides.then_some(orbit)
    }

    /// `maneuver` が指すマヌーバ（書かれていない・無ければ不足入力に記録して `None`）。
    fn transfer_maneuver(&self, o: &OrbitSettings, m: &mut MissingInputs) -> Option<&Maneuver> {
        let target = self.orbit_target(&["maneuver"]);
        let Some(id) = o.maneuver.as_deref() else {
            m.push(
                target,
                "Maneuver with a dated [transfer] to take the cruise orbit from",
            );
            return None;
        };
        let found = self.design.maneuvers.get(id);
        if found.is_none() {
            m.push(target, format!("Maneuver {id} not found"));
        }
        found
    }

    /// 遷移の到着の天体（マヌーバの `target.body`）。
    fn maneuver_target_body(&self, x: &Maneuver, m: &mut MissingInputs) -> Option<CentralBody> {
        let target = Target::maneuver(x.id.as_str(), Some("target.body"));
        self.note(target.clone());
        m.need(
            x.target.body.map(crate::context::central_body),
            target,
            "Target body of the transfer maneuver",
        )
    }

    /// 巡航の軌道（Lambert の弧を親のまわりの二体の軌道にし、両端の天体の影響圏の外の区間に切り出す。期間はその長さ）。
    fn cruise_orbit(
        &self,
        x: &Maneuver,
        (from, to): (CentralBody, CentralBody),
        (departure, arrival): (Epoch, Epoch),
        m: &mut MissingInputs,
    ) -> Option<Trajectory> {
        for key in ["departure", "arrival"] {
            self.note(Target::maneuver(
                x.id.as_str(),
                Some(&format!("transfer.{key}")),
            ));
        }
        match sat_astro::transfer::cruise_leg(from, to, departure, arrival) {
            Ok(leg) => Some(Trajectory::Elements(leg.orbit).with_span(leg.span_s)),
            Err(e) => {
                m.push(
                    self.orbit_target(&["maneuver"]),
                    format!("Cannot take the cruise orbit from maneuver {} ({e})", x.id),
                );
                None
            }
        }
    }

    /// 巡航の軌道に効かないケースの上書き（`sun_distance`・`beta`）が書かれていれば不足入力に記録する（`true` なら無い）。
    /// どちらも要素の軌道の値を置き換える指定で、日付から決まる軌道には当てられない。
    fn reject_case_overrides(&self, m: &mut MissingInputs) -> bool {
        let Some(case) = self.case else {
            return true;
        };
        let mut clean = true;
        for (key, written) in [
            ("sun_distance", case.sun_distance.is_some()),
            ("beta", case.beta.is_some()),
        ] {
            if written {
                clean = false;
                self.note(Target::case(case.id.as_str(), Some(key)));
                m.push(
                    Target::case(case.id.as_str(), Some(key)),
                    self.not_applicable_override(&format!("The case's {key}")),
                );
            }
        }
        clean
    }

    /// β 角の指定（熱の条件の `beta`）を当てられるか。ケプラー要素の軌道だけが当てられる（RAAN を置き換える）。
    /// 当てられない軌道なら、`target` を対象に理由を不足入力として記録して `false`。
    pub(crate) fn beta_applies(&self, target: impl Into<Target>, m: &mut MissingInputs) -> bool {
        if self.orbit_source() == OrbitSource::Elements {
            return true;
        }
        m.push(target, self.not_applicable_override("The beta angle"));
        false
    }

    /// 要素の軌道に限る指定（`what`）が、要素でない軌道に当たったときの理由。
    fn not_applicable_override(&self, what: &str) -> String {
        format!(
            "Not applicable: {what} applies only to an orbit given by Kepler elements, but the orbit in effect has source = \"{}\"",
            self.orbit_source()
        )
    }
}
