//! 時系列の逆算: 手計算・SMAD との一致と、逆算した設計を電力収支に渡したときの判定。
use super::*;
use crate::power::energy_balance::{
    EnergyBalance, EnergyBalanceCase, EnergyBalanceInput, EnergyBalanceLoad, EnergyBalanceOutput,
    EnergyBalanceSegment, LoadGrowth,
};
use crate::power::solar::SolarLife;
use crate::spec::Verdict;

/// SMAD の経路の効率を電力収支のモデル（変換効率 1・充放電効率 0.9）に揃える: Xd = 1、Xe = 1·0.9·0.9、Xbattery = 0.9。
fn matched(mut i: SizingInput) -> SizingInput {
    i.options.sunlit_efficiency = Some(1.0);
    i.options.eclipse_efficiency = Some(0.81);
    i.options.battery_efficiency = Some(0.9);
    i
}

/// 逆算と同じ時系列（最長蝕の日付と β）・運用で、面積 `area`・容量 `capacity` の設計の電力収支を回す。
fn balance(i: &SizingInput, o: &SizingOutput, area: f64, capacity: f64) -> EnergyBalanceOutput {
    let mut array = i.array.clone();
    array.panels[0].area = Qty::from_raw(area);
    let mut battery = i.batteries[0].clone();
    battery.capacity = Qty::from_raw(capacity);
    let cases = i
        .loads
        .modes
        .iter()
        .map(|m| EnergyBalanceCase {
            id: m.id.clone(),
            label: String::new(),
            load: EnergyBalanceLoad::Mode { mode: m.id.clone() },
            attitude: i.mode_laws.get(&m.id).cloned(),
            max_duration: i.max_durations.get(&m.id).copied(),
            timing: i.mode_timings.get(&m.id).copied().unwrap_or_default(),
        })
        .collect();
    EnergyBalance::run(&EnergyBalanceInput {
        timeline: worst_season(&i.timeline, &o.longest),
        array,
        life: Some(SolarLife::Eol),
        batteries: vec![battery],
        loads: i.loads.clone(),
        load_growth: Some(LoadGrowth::Included),
        cases,
        sequence: i.sequence.clone(),
        sequence_timing: i.sequence_timing.clone(),
        base_mode: i.base_mode.clone(),
        mode_attitudes: i.mode_laws.clone(),
    })
    .unwrap()
}

/// 電力収支の DoD・周回の収支・電池切れの判定がすべて OK か（ケースと並び）。
fn passes(out: &EnergyBalanceOutput) -> bool {
    let judged = |c: &Check| matches!(c.key.as_str(), "dod" | "energy_balance" | "depleted");
    out.cases
        .iter()
        .chain(out.sequence.as_ref().map(|s| &s.balance))
        .flat_map(|c| &c.checks)
        .filter(|c| judged(c))
        .all(|c| c.verdict == Verdict::Ok)
}

#[test]
fn sun_pointing_matches_hand_calculation_and_smad() {
    // 太陽指向の +Z 面は日照中つねに垂直入射: 1 m² あたり g = 1361·0.3·0.9·0.8 = 293.976 W（EOL、円柱影）。
    // 1 周の収支 0: 0.9·(g·A − 100)·Td = 60·Te / 0.9 → A = (100 + 60·Te/(0.81·Td)) / g。
    // 容量: 蝕で電池から出す 60·Te/0.9 を DoD 0.3 で割る。Te・Td は逆算と同じ時系列の日照率から。
    let i = matched(input());
    let o = Sizing::run(&i).unwrap();
    assert_eq!(o.basis, SizingBasis::TimeSeries);
    let ts = o.time_series.as_ref().unwrap();
    let eb = balance(&i, &o, 1.0, 360_000.0);
    let period = eb.period.value();
    let (td, te) = (
        eb.sunlit_fraction.value() * period,
        (1.0 - eb.sunlit_fraction.value()) * period,
    );
    let g = 1361.0 * 0.3 * 0.9 * 0.8;
    let area = (100.0 + 60.0 * te / (0.81 * td)) / g;
    let capacity = 60.0 * te / (0.9 * 0.3);
    let rel = |a: f64, b: f64| (a / b - 1.0).abs();
    assert!(
        rel(ts.required_area.value(), area) < 2e-3,
        "{} {area}",
        ts.required_area.value()
    );
    assert!(rel(ts.required_capacity.value(), capacity) < 2e-3);
    // SMAD（最長蝕は季節解析の円錐影、日照時間は周期 − 最長蝕）との差は蝕の長さの違いの分だけ: 1 % 以内。
    let m = &o.modes[0];
    assert!(
        (m.area_ratio.unwrap().value() - 1.0).abs() < 0.01,
        "{:?}",
        m.area_ratio
    );
    assert!(
        (m.capacity_ratio.unwrap().value() - 1.0).abs() < 0.01,
        "{:?}",
        m.capacity_ratio
    );
    assert_eq!(
        ts.targets[0].evaluation,
        crate::power::energy_balance::EnergyBalanceEvaluation::Continuous
    );
    assert!(ts.targets[0].energy_margin_per_orbit.value() >= 0.0);
    assert_eq!(o.required_area, ts.required_area);
    assert!(
        o.checks
            .iter()
            .any(|c| c.key == "sap_margin" && c.message.contains("time-series sizing"))
    );
}

#[test]
fn zenith_panel_on_nadir_pointing_where_smad_underestimates() {
    // ナディア指向で天頂を向いた面: 日照中の入射は平均 0.5 前後で、日照中にも発電が負荷を下回る時間がある。
    let mut i = matched(input());
    i.array = sap(1.0, -Vec3::Z);
    i.timeline.attitude = nadir_pointing_z();
    i.mode_laws.insert("NOMINAL".into(), nadir_pointing_z());
    let o = Sizing::run(&i).unwrap();
    let m = &o.modes[0];
    assert!(m.incidence_factor.value() > 0.3 && m.incidence_factor.value() < 0.7);
    // SMAD は面積も容量も少なく見る（この例では面積 ×1.04、容量 ×1.78。日照中にも電池から出す分を数えないため）
    assert!(m.area_ratio.unwrap().value() > 1.02, "{:?}", m.area_ratio);
    assert!(
        m.capacity_ratio.unwrap().value() > 1.5,
        "{:?}",
        m.capacity_ratio
    );
    // SMAD の値の設計は電力収支を満たさない（1 周の収支 −20 kJ、DoD 0.58）。時系列の逆算の値の設計は満たす。
    let smad = balance(&i, &o, m.required_area.value(), m.required_capacity.value());
    assert!(smad.cases[0].energy_margin_per_orbit.unwrap().value() < 0.0);
    assert!(smad.cases[0].max_dod.value() > 0.3);
    assert!(!passes(&smad));
    let ts = balance(&i, &o, o.required_area.value(), o.required_capacity.value());
    assert!(passes(&ts), "{:?}", ts.cases[0].checks);
    // 面積を許容差より大きく減らすと収支が負になる（最小であること）
    let less = balance(
        &i,
        &o,
        o.required_area.value() * (1.0 - 10.0 * AREA_TOLERANCE),
        o.required_capacity.value(),
    );
    assert!(less.cases[0].energy_margin_per_orbit.unwrap().value() < 0.0);
}

/// 2 モード（日照中は太陽指向の NOMINAL、ナディア指向で負荷の大きい MISSION）と並び・最長継続時間・初期 SOC 0.9。
fn operations() -> SizingInput {
    let mut i = matched(input());
    i.timeline = leo_timeline(sun_pointing_z(), 4.0, 20.0);
    i.batteries[0].initial_soc = Some(0.9);
    let mut payload = i.loads.parts[0].clone();
    payload.id = "PAY".into();
    payload.power.by_state = BTreeMap::from([
        ("on".into(), Qty::from_raw(40.0)),
        ("off".into(), Qty::from_raw(0.0)),
    ]);
    payload.states = vec!["on".into(), "off".into()];
    i.loads.parts.push(payload);
    let pay = |state: &str| StateAssignment {
        part: "PAY".into(),
        ..assignment(state)
    };
    i.loads.modes[0].assignments.push(pay("off"));
    i.loads.modes[0].eclipse.push(pay("off"));
    let mut mission = i.loads.modes[0].clone();
    mission.id = "MISSION".into();
    mission.assignments = vec![assignment("on"), pay("on")];
    mission.eclipse = vec![assignment("on"), pay("on")];
    i.loads.modes.push(mission);
    i.mode_laws.insert("MISSION".into(), nadir_pointing_z());
    i.max_durations
        .insert("MISSION".into(), Qty::from_raw(600.0));
    let minutes = |m: f64| Qty::from_raw(m * 60.0);
    i.sequence = vec![
        EnergyBalanceSegment {
            mode: "NOMINAL".into(),
            duration: minutes(80.0),
        },
        EnergyBalanceSegment {
            mode: "MISSION".into(),
            duration: minutes(14.6),
        },
    ];
    i
}

#[test]
fn sized_design_passes_every_judged_operation() {
    let i = operations();
    let o = Sizing::run(&i).unwrap();
    let ts = o.time_series.as_ref().unwrap();
    use crate::power::energy_balance::EnergyBalanceEvaluation as E;
    let evaluation = |id: &str| ts.targets.iter().find(|t| t.id == id).unwrap().evaluation;
    assert_eq!(evaluation("NOMINAL"), E::Continuous);
    assert_eq!(evaluation("MISSION"), E::MaxDuration);
    assert_eq!(evaluation("sequence"), E::Sequence);
    assert!(ts.targets.iter().all(|t| t.evaluations < 40));
    let out = balance(&i, &o, o.required_area.value(), o.required_capacity.value());
    assert!(
        passes(&out),
        "{:?}",
        out.cases.iter().map(|c| &c.checks).collect::<Vec<_>>()
    );
    // 決めた運用の DoD は上限ちょうど（容量は閉じた形）
    let governing = match ts.capacity_target.as_str() {
        "sequence" => out.sequence.as_ref().unwrap().balance.max_dod.value(),
        id => {
            let c = out.cases.iter().find(|c| c.id == id).unwrap();
            c.by_max_duration
                .as_ref()
                .map_or(c.max_dod.value(), |d| d.max_dod.value())
        }
    };
    assert!((governing - 0.3).abs() < 1e-9, "{governing}");
    // 一つのモードだけを逆算するときは並びを使わない
    let mut one = operations();
    one.mode = Some("MISSION".into());
    let so = Sizing::run(&one).unwrap();
    let targets = &so.time_series.as_ref().unwrap().targets;
    assert_eq!(targets.len(), 1);
    assert_eq!(targets[0].evaluation, E::MaxDuration);
}

#[test]
fn short_span_falls_back_to_smad() {
    // 1 周に満たない時系列では 1 周の収支を判定できないので、SMAD で判定してその旨を注意に出す。
    let mut i = input();
    i.timeline = leo_timeline(sun_pointing_z(), 0.5, 10.0);
    let o = Sizing::run(&i).unwrap();
    assert_eq!(o.basis, SizingBasis::Smad);
    assert!(o.time_series.is_none());
    assert_eq!(o.required_area, o.smad.required_area);
    assert!(o.modes[0].series_area.is_none());
    let warning = o.checks.iter().find(|c| c.key == "time_series").unwrap();
    assert_eq!(warning.verdict, Verdict::Warning);
}

#[test]
fn mode_timing_reaches_the_time_series_inverse() {
    use crate::power::energy_balance::{DurationTiming, EclipseEvent, EventStart, ModeTiming};
    // 時刻の指定が無ければ周回内の最悪の開始。MISSION の開始を蝕明けに決めると、窓が日照中に入る分だけ
    // 1 周の収支が軽くなり（電池を通す負荷が減る）、逆算した必要面積は小さくなる
    // （電力収支と同じ運用で評価するので、逆算した設計は電力収支の判定を通り、開始は指定のものになる）
    // （MISSION も太陽指向にして、面積で収支が取れるようにする）
    let free = {
        let mut i = operations();
        i.mode_laws.insert("MISSION".into(), sun_pointing_z());
        i
    };
    let o_free = Sizing::run(&free).unwrap();
    let mut fixed = free.clone();
    fixed.mode_timings.insert(
        "MISSION".into(),
        ModeTiming {
            start: Some(EventStart {
                from: EclipseEvent::EclipseExit,
                offset: Qty::from_raw(0.0),
            }),
            ..Default::default()
        },
    );
    let o_fixed = Sizing::run(&fixed).unwrap();
    let cap = |o: &SizingOutput| {
        o.time_series
            .as_ref()
            .unwrap()
            .targets
            .iter()
            .find(|t| t.id == "MISSION")
            .map(|t| t.required_capacity.value())
            .unwrap()
    };
    assert!(
        cap(&o_fixed) < cap(&o_free),
        "{} {}",
        cap(&o_fixed),
        cap(&o_free)
    );
    let out = balance(
        &fixed,
        &o_fixed,
        o_fixed.required_area.value(),
        o_fixed.required_capacity.value(),
    );
    assert!(passes(&out));
    let d = out
        .cases
        .iter()
        .find(|c| c.id == "MISSION")
        .and_then(|c| c.by_max_duration.as_ref())
        .unwrap();
    assert_eq!(d.timing, DurationTiming::Fixed);
}
