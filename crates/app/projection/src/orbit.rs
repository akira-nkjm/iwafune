//! 軌道条件の解析（季節解析・高度低下）。既定値や物理計算を持たず、条件ファイル・部品から写す。
use crate::parts::{by_role, part_qty};
use crate::quantity::{setting, setting_opt};
use crate::{Context, MissingInputs};
use fune_model::Target;
use fune_model::builtin::Role;
use fune_model::design::OrbitDecaySettings;
use fune_model::design::setting_dims as dims;
use fune_model::value::Quantity;
use sat_analysis::orbit::EclipseSeasonInput;
use sat_analysis::orbit::decay::{DecayPropulsion, OrbitDecayInput};
use sat_units::typed::{Area, Length, Mass, Qty, Time};

/// 軌道・ミッション期間・任意の季節刻み。
pub fn eclipse_season(ctx: &Context<'_>) -> Result<EclipseSeasonInput, MissingInputs> {
    let mut m = MissingInputs::new();
    let orbit = ctx.orbit(&mut m);
    let o = ctx.design.conditions.orbit.as_ref();
    let duration = setting(
        &dims::ORBIT_DURATION,
        &[],
        o.and_then(|o| o.duration.as_ref()),
        "ミッション期間",
        &mut m,
    );
    let step = setting_opt(
        &dims::ORBIT_ECLIPSE_SEASON_STEP,
        &[],
        o.and_then(|o| o.eclipse_season.as_ref())
            .and_then(|s| s.step.as_ref()),
        &mut m,
    );
    // 数値伝搬のマヌーバがあるときだけ、標本日の軌道をマヌーバの順の推移から引く（無ければ結果を変えない）
    let maneuvers = if crate::propulsion::has_propagated(ctx) {
        crate::propulsion::maneuvers(ctx)
            .map_err(|e| m.extend(e))
            .ok()
    } else {
        None
    };
    m.finish(|| {
        Some(EclipseSeasonInput {
            orbit: orbit?,
            duration: duration?,
            step,
            maneuvers,
        })
    })
}

/// `orbit.decay`: 軌道・機体の質量（部品）・外形（構体の `size`）・抗力係数（`analysis.aocs.drag_coefficient`）・
/// ミッション期間と `[decay]` の任意の設定。宇宙天気の表から引いた列は service が足す（ここは表を読まない）。
pub fn decay(ctx: &Context<'_>) -> Result<OrbitDecayInput, MissingInputs> {
    let mut m = MissingInputs::new();
    let orbit = ctx.orbit(&mut m);
    let o = ctx.design.conditions.orbit.as_ref();
    let mission_duration = setting_opt(
        &dims::ORBIT_DURATION,
        &[],
        o.and_then(|o| o.duration.as_ref()),
        &mut m,
    );
    let s = DecaySettings::read(o.and_then(|o| o.decay.as_ref()), &mut m);
    let body_size = match s.drag_area {
        Some(_) => None,
        None => {
            let size = crate::aocs::reference_box(ctx, &mut m);
            if size.is_none() {
                m.push(
                    dims::ORBIT_DECAY_DRAG_AREA.at(&[]),
                    "抗力の面積（または構体の外形 size）",
                );
            }
            size
        }
    };
    let mass = crate::structure::summary(ctx, &mut m).map(|s| s.total_mass);
    let propulsion = s.propellant.and_then(|p| propulsion(ctx, p, &mut m));
    let drag_coefficient = ctx
        .design
        .analysis
        .aocs
        .as_ref()
        .and_then(|a| a.drag_coefficient);
    m.finish(|| {
        Some(OrbitDecayInput {
            orbit: orbit?,
            mass: mass?,
            drag_area: s.drag_area,
            body_size,
            drag_coefficient,
            mission_duration,
            span: s.span,
            reentry_altitude: s.reentry_altitude,
            band: s.band,
            max_lifetime: s.max_lifetime,
            propulsion,
            dated_weather: None,
        })
    })
}

/// `[decay]` の設定を型付きの量に読んだもの（書かれていなければ `None`）。
struct DecaySettings {
    span: Option<Qty<Time>>,
    reentry_altitude: Option<Qty<Length>>,
    drag_area: Option<Qty<Area>>,
    band: Option<Qty<Length>>,
    max_lifetime: Option<Qty<Time>>,
    propellant: Option<Qty<Mass>>,
}

impl DecaySettings {
    fn read(s: Option<&OrbitDecaySettings>, m: &mut MissingInputs) -> Self {
        let q = |f: fn(&OrbitDecaySettings) -> Option<&Quantity>| s.and_then(f);
        Self {
            span: setting_opt(&dims::ORBIT_DECAY_SPAN, &[], q(|s| s.span.as_ref()), m),
            reentry_altitude: setting_opt(
                &dims::ORBIT_DECAY_REENTRY_ALTITUDE,
                &[],
                q(|s| s.reentry_altitude.as_ref()),
                m,
            ),
            drag_area: setting_opt(
                &dims::ORBIT_DECAY_DRAG_AREA,
                &[],
                q(|s| s.drag_area.as_ref()),
                m,
            ),
            band: setting_opt(&dims::ORBIT_DECAY_BAND, &[], q(|s| s.band.as_ref()), m),
            max_lifetime: setting_opt(
                &dims::ORBIT_DECAY_MAX_LIFETIME,
                &[],
                q(|s| s.max_lifetime.as_ref()),
                m,
            ),
            propellant: setting_opt(
                &dims::ORBIT_DECAY_PROPELLANT,
                &[],
                q(|s| s.propellant.as_ref()),
                m,
            ),
        }
    }
}

/// 推薬を書いたときの推進系: 比推力はスラスタ（role = thruster）のうち最も低いもの（保守側）。
fn propulsion(
    ctx: &Context<'_>,
    propellant: Qty<Mass>,
    m: &mut MissingInputs,
) -> Option<DecayPropulsion> {
    let thrusters: Vec<_> = by_role(ctx, Role::Thruster)
        .filter_map(|p| {
            Some((
                p.id.to_string(),
                part_qty::<Time>(p, "isp", "スラスタの比推力", m)?,
            ))
        })
        .collect();
    let lowest = thrusters
        .into_iter()
        .min_by(|a, b| a.1.value().total_cmp(&b.1.value()));
    if lowest.is_none() {
        m.push(
            Target::any_of_roles(&[Role::Thruster], None),
            "軌道維持の推薬を書いたときはスラスタ（role = thruster）の比推力が要ります",
        );
    }
    lowest.map(|(thruster, isp)| DecayPropulsion {
        thruster,
        isp,
        propellant,
    })
}
