//! 軌道条件の解析（季節解析・高度低下・ミッション幾何）。既定値や物理計算を持たず、条件ファイル・部品から写す。
use crate::{Context, MissingInputs};
use fune_model::Target;
use fune_model::design::OrbitDecaySettings;
use fune_model::design::setting_dims as dims;
use fune_model::value::Quantity;
use sat_analysis::orbit::decay::OrbitDecayInput;
use sat_analysis::orbit::{EclipseSeasonInput, GeometryInput, GeometryLegInput, SunDistanceInput};
use sat_analysis::structure::interference::InterferenceBox;
use sat_units::typed::{Area, Length, Qty, Time};

/// 軌道・Mission duration・任意の季節刻み。
pub fn eclipse_season(ctx: &Context<'_>) -> Result<EclipseSeasonInput, MissingInputs> {
    let mut m = MissingInputs::new();
    let orbit = ctx.orbit(&mut m);
    let o = ctx.orbit_settings();
    ctx.note(ctx.orbit_target(&["eclipse_season", "step"]));
    // ミッション期間は要求が与える（`input = "mission_duration"`）。無ければ入力不足
    let duration = m.need(
        ctx.mission_duration(),
        Context::mission_duration_target(),
        "Mission duration (an at_least requirement with input = \"mission_duration\")",
    );
    let step = ctx.orbit_setting_opt(
        &dims::ORBIT_ECLIPSE_SEASON_STEP,
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

/// `orbit.geometry`: フェーズごとの区間（並びの順。フェーズが無い設計は設計の軌道 1 つ）。区間の軌道はそのフェーズで効いている軌道（出どころは問わない。遷移から取る巡航の区間も入る）
/// （フェーズが `[orbit]` を持たなければ設計の軌道）、期間はフェーズの `duration`（書かれていれば）。ケースは選ばなくてよく、
/// ケースが指すフェーズは使わず全フェーズを並べる。
pub fn geometry(ctx: &Context<'_>) -> Result<GeometryInput, MissingInputs> {
    let mut m = MissingInputs::new();
    let phases = ctx.design.ordered_phases();
    let legs: Vec<Option<GeometryLegInput>> = if phases.is_empty() {
        vec![ctx.trajectory(&mut m).map(|orbit| GeometryLegInput {
            phase: None,
            orbit,
            duration: None,
        })]
    } else {
        phases
            .into_iter()
            .map(|p| {
                let c = ctx.in_phase(p);
                let target = Target::phase(p.id.as_str(), Some("duration"));
                ctx.note(target.clone());
                let duration =
                    crate::quantity::qty_opt::<Time>(p.duration.as_ref(), target, &mut m);
                c.trajectory(&mut m).map(|orbit| GeometryLegInput {
                    phase: Some(p.id.as_str().to_owned()),
                    orbit,
                    duration,
                })
            })
            .collect()
    };
    m.finish(|| {
        legs.into_iter()
            .collect::<Option<Vec<_>>>()
            .map(|legs| GeometryInput { legs })
    })
}

/// 太陽距離の極値（`sat_analysis::orbit::sun_distance_extremes`。解析ケースを近日点・遠日点で作るのに使う）の入力:
/// 軌道（ケースを指定しない）とミッション期間。
pub fn sun_distance(ctx: &Context<'_>) -> Result<SunDistanceInput, MissingInputs> {
    let mut m = MissingInputs::new();
    let orbit = ctx.orbit(&mut m);
    // ミッション期間は要求が与える（`input = "mission_duration"`）。無ければ入力不足
    let duration = m.need(
        ctx.mission_duration(),
        Context::mission_duration_target(),
        "Mission duration (an at_least requirement with input = \"mission_duration\")",
    );
    m.finish(|| {
        Some(SunDistanceInput {
            orbit: orbit?,
            duration: duration?,
        })
    })
}

/// `orbit.decay`: 軌道・機体の質量（部品）・部品の箱（抗力の面積。書かれていなければ解析が全方向の平均の投影面積を出す）・抗力係数（`analysis.aocs.drag_coefficient`）・
/// Mission durationと `[decay]` の任意の設定。軌道維持の推進系は推進系の解析と同じ入力（`propulsion::maneuvers`。
/// 推進系が無ければ付けず、有れば足りない入力も並べる）。宇宙天気の表から引いた列は service が足す（ここは表を読まない）。
pub fn decay(ctx: &Context<'_>) -> Result<OrbitDecayInput, MissingInputs> {
    let mut m = MissingInputs::new();
    let orbit = ctx.orbit(&mut m);
    let o = ctx.orbit_settings();
    let mission_duration = ctx.mission_duration();
    let s = DecaySettings::read(ctx, o.and_then(|o| o.decay.as_ref()), &mut m);
    // 面積が書かれていなければ、部品の箱（有効な形態の配置）から解析が出す
    let body_boxes: Vec<InterferenceBox> = match s.drag_area {
        Some(_) => Vec::new(),
        None => {
            let body = crate::structure::boxes(ctx);
            if body.boxes.is_empty() {
                m.push(
                    ctx.orbit_target(&["decay", "drag_area"]),
                    "Drag area (or a part with a size)",
                );
            }
            body.boxes.into_iter().map(InterferenceBox::from).collect()
        }
    };
    let mass = crate::structure::summary(ctx, &mut m).map(|s| s.total_mass);
    let propulsion = if crate::propulsion::present(ctx) {
        crate::propulsion::maneuvers(ctx)
            .map_err(|e| m.extend(e))
            .ok()
    } else {
        None
    };
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
            body_boxes,
            drag_coefficient,
            mission_duration,
            span: s.span,
            reentry_altitude: s.reentry_altitude,
            band: s.band,
            max_lifetime: ctx
                .criterion("orbit.decay", "max_lifetime")
                .map(|c| Qty::from_raw(c.si)),
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
}

impl DecaySettings {
    fn read(ctx: &Context<'_>, s: Option<&OrbitDecaySettings>, m: &mut MissingInputs) -> Self {
        let q = |f: fn(&OrbitDecaySettings) -> Option<&Quantity>| s.and_then(f);
        Self {
            span: ctx.orbit_setting_opt(&dims::ORBIT_DECAY_SPAN, q(|s| s.span.as_ref()), m),
            reentry_altitude: ctx.orbit_setting_opt(
                &dims::ORBIT_DECAY_REENTRY_ALTITUDE,
                q(|s| s.reentry_altitude.as_ref()),
                m,
            ),
            drag_area: ctx.orbit_setting_opt(
                &dims::ORBIT_DECAY_DRAG_AREA,
                q(|s| s.drag_area.as_ref()),
                m,
            ),
            band: ctx.orbit_setting_opt(&dims::ORBIT_DECAY_BAND, q(|s| s.band.as_ref()), m),
        }
    }
}
