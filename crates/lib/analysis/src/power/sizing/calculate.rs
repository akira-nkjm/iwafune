//! サイジング内部の仮定解決・負荷の選択・SMAD の式・直並列数。
use super::*;

/// SMAD の式。容量は N 組の合計（1 組の容量は呼び出し側で N で割る）。
pub(super) fn smad_requirements(
    pd: f64,
    pe: f64,
    td: f64,
    te: f64,
    u: &SizingUsed,
    density: f64,
) -> Result<(f64, f64, f64), AnalysisError> {
    ensure_positive("Sunlit time", td)?;
    ensure_non_negative("Eclipse time", te)?;
    let generation = (pe * te / u.eclipse_efficiency + pd * td / u.sunlit_efficiency) / td;
    let area = generation / density;
    let capacity = pe * te / (u.dod_max.value() * u.battery_efficiency);
    ensure_non_negative("Required generation", generation)?;
    ensure_non_negative("Required area", area)?;
    ensure_non_negative("Required capacity", capacity)?;
    Ok((generation, area, capacity))
}

/// 設計値と必要量の比の余裕の判定。`note` は必要量の求め方（文の括弧書き）。
pub(super) fn margin_check<K: sat_units::typed::NamedKind>(
    (key, label): (&str, &str),
    margin: Option<Fraction>,
    design: f64,
    required: f64,
    note: &str,
) -> Check {
    Check::new(
        key,
        label,
        ratio_margin_verdict(margin, Fraction::new(0.0)),
        format!(
            "Design {} / required {} ({note})",
            quantity_text::<K>(design),
            quantity_text::<K>(required)
        ),
    )
    .with_ratio_margin(margin)
}

pub(super) fn used(
    input: &SizingInput,
    battery: &EnergyBalanceBattery,
) -> Result<SizingUsed, AnalysisError> {
    let o = &input.options;
    let (distance, sun_distance_source, sun_distance_epoch) = sun_distance_used(input)?;
    ensure_positive("Sun distance", distance.value())?;
    let (life, lifetime_source) = match input.design_life {
        Some(life) => (life, SizingLifetimeSource::DesignLife),
        None => (input.season.duration, SizingLifetimeSource::SeasonSpan),
    };
    let lifetime_years = sat_units::parse_unit("yr")
        .map_err(|e| AnalysisError::InvalidInput(e.to_string()))?
        .from_si(life.value());
    let annual = o.annual_degradation.unwrap_or(0.0);
    ensure_fraction("Annual degradation rate", annual)?;
    let u = SizingUsed {
        sunlit_efficiency: o.sunlit_eff(),
        eclipse_efficiency: o.eclipse_eff(),
        battery_efficiency: o.battery_efficiency.unwrap_or(battery.discharge_eff()),
        annual_degradation: annual,
        inherent_degradation: o.inherent_degradation.unwrap_or(1.0),
        lifetime_years,
        lifetime_source,
        life_factor: (1.0 - annual).powf(lifetime_years),
        dod_max: Fraction::new(battery.dod_limit),
        battery_strings: o.battery_strings.unwrap_or(1),
        sun_distance: distance,
        sun_distance_source,
        sun_distance_epoch,
        panels: input
            .array
            .panels
            .iter()
            .map(|p| SolarPanel {
                packing_factor: Some(p.packing()),
                eol_factor: Some(p.eol()),
                ..p.clone()
            })
            .collect(),
    };
    validate_used(&u)?;
    Ok(u)
}

/// 発電の強さを決める太陽距離: 設定があればそれ、なければ中心天体の慣例値（地球・月は 1 AU）、それも無い中心天体
/// （火星など日心距離が変わる）は、季節の走査の期間（ミッション期間）の中の太陽距離の最大（最悪の日。発電が最小）。
/// 最大は軌道の関数としての太陽距離（ミッション幾何の 1 か所、`sun_distance_extremes_over`）から取る。
fn sun_distance_used(
    input: &SizingInput,
) -> Result<(Qty<Length>, SizingSunDistanceSource, Option<Epoch>), AnalysisError> {
    if let Some(d) = input.options.sun_distance {
        return Ok((d, SizingSunDistanceSource::Setting, None));
    }
    let orbit = &input.season.orbit;
    if let Some(d) = orbit.central.nominal_sun_distance_m() {
        return Ok((Qty::from_raw(d), SizingSunDistanceSource::Nominal, None));
    }
    let x = sun_distance_extremes_over(
        &Trajectory::from(orbit.clone()),
        input.season.duration.value(),
    )?;
    Ok((
        x.max.distance,
        SizingSunDistanceSource::SpanMax,
        Some(x.max.epoch),
    ))
}

fn validate_used(u: &SizingUsed) -> Result<(), AnalysisError> {
    for (name, v) in [
        ("Sunlit path efficiency", u.sunlit_efficiency),
        ("Eclipse path efficiency", u.eclipse_efficiency),
        ("Battery efficiency", u.battery_efficiency),
        ("Inherent degradation", u.inherent_degradation),
        ("Allowed DoD", u.dod_max.value()),
    ] {
        ensure_fraction(name, v)?;
        ensure_positive(name, v)?;
    }
    ensure_positive("Lifetime in years", u.lifetime_years)?;
    if u.battery_strings == 0 {
        return Err(AnalysisError::InvalidInput(
            "Number of battery strings must be positive".into(),
        ));
    }
    Ok(())
}

/// 逆算に使うモードの負荷（成長余裕込み）。
pub(super) struct ModeLoads {
    pub id: String,
    pub sunlit: f64,
    pub eclipse: f64,
}

/// 逆算の対象モードの負荷と、全モードの負荷の内訳（`power.mode_load` の出力。時系列の逆算も使う）。
/// 値が不足するモードがあれば入力エラー（判定は出さない）。
pub(super) fn loads(
    input: &SizingInput,
) -> Result<(Vec<ModeLoads>, ModeLoadOutput), AnalysisError> {
    if input.loads.parts.is_empty() {
        return Err(AnalysisError::InvalidInput("No load parts".into()));
    }
    let output = ModeLoad::run(&input.loads)?;
    let mut loads = Vec::new();
    for m in output
        .modes
        .iter()
        .filter(|m| input.mode.as_ref().is_none_or(|id| *id == m.id))
    {
        // 値不足の合計を逆算に使うと要求を過小評価するため止める。
        if !m.checks.is_empty() {
            return Err(AnalysisError::InvalidInput(format!(
                "Load inputs of mode {} are missing",
                m.id
            )));
        }
        ensure_non_negative("Sunlit load", m.total_with_growth.value())?;
        ensure_non_negative("Eclipse load", m.total_eclipse_with_growth.value())?;
        loads.push(ModeLoads {
            id: m.id.clone(),
            sunlit: m.total_with_growth.value(),
            eclipse: m.total_eclipse_with_growth.value(),
        });
    }
    if loads.is_empty() {
        return Err(AnalysisError::InvalidInput("No target modes".into()));
    }
    Ok((loads, output))
}

/// 設計の SAP 面積と、逆算に使う SAP。
/// 変換効率は日照経路の効率 Xd で数え、太陽光の強さは太陽距離 `used.sun_distance` に固定する
/// （`sat_astro::illumination::solar_flux_w_m2`。入射係数を幾何だけにする）。
pub(super) fn design(
    input: &SizingInput,
    used: &SizingUsed,
) -> Result<(Qty<Area>, SolarArray), AnalysisError> {
    if input.array.panels.is_empty() {
        return Err(AnalysisError::InvalidInput("No solar panels".into()));
    }
    input.array.validate()?;
    let mut array = input.array.clone();
    array.solar_flux = Some(input.array.flux(Some(used.sun_distance.value())));
    array.conversion_efficiency = None;
    let area = array.panels.iter().map(|p| p.area.value()).sum::<f64>();
    ensure_positive("Design SAP area", area)?;
    Ok((Qty::from_raw(area), array))
}

/// 直並列数。`incidence` は面積を決めたモードの入射係数。
pub(super) fn strings(
    input: &SizingInput,
    u: &SizingUsed,
    watts: f64,
    incidence: f64,
) -> Result<Option<SizingStrings>, AnalysisError> {
    let Some(bus) = input.options.bus_voltage else {
        return Ok(None);
    };
    ensure_positive("Bus voltage", bus.value())?;
    let panels = &input.array.panels;
    let cell = |id: &str| input.cells.iter().find(|c| c.panel == id);
    let Some(first) = panels.first() else {
        return Ok(None);
    };
    let Some(c) = cell(&first.id) else {
        return Ok(None);
    };
    let (v, i) = (c.voltage, c.current);
    // 異種セルの混在では単一の直並列を定義できない。
    if panels.iter().any(|p| {
        cell(&p.id).is_none_or(|c| c.voltage != v || c.current != i) || p.eol() != first.eol()
    }) {
        return Ok(None);
    }
    ensure_positive("Cell voltage", v.value())?;
    ensure_positive("Cell current", i.value())?;
    let eol = first.eol() * u.life_factor * u.inherent_degradation * incidence;
    let current = watts / bus.value();
    let series = (bus.value() / v.value()).ceil();
    let parallel = (current / (i.value() * eol)).ceil();
    if !parallel.is_finite() || series > f64::from(u32::MAX) || parallel > f64::from(u32::MAX) {
        return Err(AnalysisError::InvalidInput(
            "The series/parallel count exceeds the limit".into(),
        ));
    }
    Ok(Some(SizingStrings {
        series_count: series as u32,
        parallel_count: parallel as u32,
        required_current: Qty::from_raw(current),
    }))
}
