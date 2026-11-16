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
    ensure_positive("日照時間", td)?;
    ensure_non_negative("蝕時間", te)?;
    let generation = (pe * te / u.eclipse_efficiency + pd * td / u.sunlit_efficiency) / td;
    let area = generation / density;
    let capacity = pe * te / (u.dod_max.value() * u.battery_efficiency);
    ensure_non_negative("必要発電", generation)?;
    ensure_non_negative("必要面積", area)?;
    ensure_non_negative("必要容量", capacity)?;
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
            "設計 {} / 必要 {}（{note}）",
            quantity_text::<K>(design),
            quantity_text::<K>(required)
        ),
    )
}

pub(super) fn used(
    input: &SizingInput,
    battery: &EnergyBalanceBattery,
) -> Result<SizingUsed, AnalysisError> {
    let o = &input.options;
    let central = input.season.orbit.central;
    let distance = o
        .sun_distance
        .or_else(|| central.nominal_sun_distance_m().map(Qty::from_raw))
        .ok_or_else(|| {
            AnalysisError::InvalidInput(format!(
                "中心天体 {central:?} の太陽距離の慣例値はありません。sun_distance を書いてください"
            ))
        })?;
    ensure_positive("太陽距離", distance.value())?;
    let (life, lifetime_source) = match input.design_life {
        Some(life) => (life, SizingLifetimeSource::DesignLife),
        None => (input.season.duration, SizingLifetimeSource::SeasonSpan),
    };
    let lifetime_years = sat_units::parse_unit("yr")
        .map_err(|e| AnalysisError::InvalidInput(e.to_string()))?
        .from_si(life.value());
    let annual = o.annual_degradation.unwrap_or(0.0);
    ensure_fraction("年あたりの劣化率", annual)?;
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

fn validate_used(u: &SizingUsed) -> Result<(), AnalysisError> {
    for (name, v) in [
        ("日照経路の効率", u.sunlit_efficiency),
        ("蝕経路の効率", u.eclipse_efficiency),
        ("電池効率", u.battery_efficiency),
        ("固有の劣化", u.inherent_degradation),
        ("許容 DoD", u.dod_max.value()),
    ] {
        ensure_fraction(name, v)?;
        ensure_positive(name, v)?;
    }
    ensure_positive("寿命年数", u.lifetime_years)?;
    if u.battery_strings == 0 {
        return Err(AnalysisError::InvalidInput(
            "電池の組数は正にしてください".into(),
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

/// 逆算の対象モードの負荷。値が不足するモードがあれば入力エラー（判定は出さない）。
pub(super) fn loads(input: &SizingInput) -> Result<Vec<ModeLoads>, AnalysisError> {
    if input.loads.parts.is_empty() {
        return Err(AnalysisError::InvalidInput("負荷の部品がありません".into()));
    }
    let modes = ModeLoad::run(&input.loads)?.modes;
    let mut loads = Vec::new();
    for m in modes
        .iter()
        .filter(|m| input.mode.as_ref().is_none_or(|id| *id == m.id))
    {
        // 値不足の合計を逆算に使うと要求を過小評価するため止める。
        if !m.checks.is_empty() {
            return Err(AnalysisError::InvalidInput(format!(
                "モード {} の負荷入力が不足しています",
                m.id
            )));
        }
        ensure_non_negative("日照負荷", m.total_with_growth.value())?;
        ensure_non_negative("蝕負荷", m.total_eclipse_with_growth.value())?;
        loads.push(ModeLoads {
            id: m.id.clone(),
            sunlit: m.total_with_growth.value(),
            eclipse: m.total_eclipse_with_growth.value(),
        });
    }
    if loads.is_empty() {
        return Err(AnalysisError::InvalidInput("対象モードがありません".into()));
    }
    Ok(loads)
}

/// 設計の SAP 面積と、逆算に使う SAP。
/// 変換効率は日照経路の効率 Xd で数え、太陽光の強さは太陽距離 `used.sun_distance` に固定する
/// （`sat_astro::illumination::solar_flux_w_m2`。入射係数を幾何だけにする）。
pub(super) fn design(
    input: &SizingInput,
    used: &SizingUsed,
) -> Result<(Qty<Area>, SolarArray), AnalysisError> {
    if input.array.panels.is_empty() {
        return Err(AnalysisError::InvalidInput(
            "太陽電池パネルがありません".into(),
        ));
    }
    input.array.validate()?;
    let mut array = input.array.clone();
    array.solar_flux = Some(input.array.flux(Some(used.sun_distance.value())));
    array.conversion_efficiency = None;
    let area = array.panels.iter().map(|p| p.area.value()).sum::<f64>();
    ensure_positive("設計の SAP 面積", area)?;
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
    ensure_positive("バス電圧", bus.value())?;
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
    ensure_positive("セル電圧", v.value())?;
    ensure_positive("セル電流", i.value())?;
    let eol = first.eol() * u.life_factor * u.inherent_degradation * incidence;
    let current = watts / bus.value();
    let series = (bus.value() / v.value()).ceil();
    let parallel = (current / (i.value() * eol)).ceil();
    if !parallel.is_finite() || series > f64::from(u32::MAX) || parallel > f64::from(u32::MAX) {
        return Err(AnalysisError::InvalidInput(
            "直並列数が上限を超えています".into(),
        ));
    }
    Ok(Some(SizingStrings {
        series_count: series as u32,
        parallel_count: parallel as u32,
        required_current: Qty::from_raw(current),
    }))
}
