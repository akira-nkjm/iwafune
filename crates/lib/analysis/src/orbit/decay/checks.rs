//! 高度低下の判定: 近円軌道の前提・ミッション期間に対する寿命・廃棄の寿命の上限・推薬の寿命。
//! 判定文の数は `spec::quantity_text` で書く。条件は ID（`low`・`moderate`・`high`・`dated`）で名指す。

use super::{DATED, DecayCase, DecayUsed, MODERATE, NEAR_CIRCULAR_ECCENTRICITY, OrbitDecayInput};
use crate::spec::{Check, Verdict, quantity_text, ratio_margin};
use sat_units::typed::{Dimensionless, Time};

/// すべての判定（書かれた要求があるものだけ）。
pub(super) fn checks(
    input: &OrbitDecayInput,
    used: &DecayUsed,
    cases: &[DecayCase],
    undetermined: Option<&[String]>,
) -> Vec<Check> {
    let mut out = Vec::new();
    if let Some(names) = undetermined {
        out.push(thruster_check(names));
    }
    let e = input.orbit.elements.eccentricity;
    if e > NEAR_CIRCULAR_ECCENTRICITY {
        out.push(Check::new(
            "near_circular",
            "Near-circular orbit assumption",
            Verdict::Warning,
            format!(
                "Eccentricity {} exceeds the near-circular assumption ({} or less). Eccentricity changes are not tracked, so the lifetime is a rough estimate",
                quantity_text::<Dimensionless>(e),
                quantity_text::<Dimensionless>(NEAR_CIRCULAR_ECCENTRICITY)
            ),
        ));
    }
    if let Some(mission) = input.mission_duration {
        out.push(mission_check(used, cases, mission.value()));
        if used.propulsion_delta_v.is_some() {
            out.push(propellant_check(cases, mission.value()));
        }
    }
    if let Some(max) = input.max_lifetime {
        out.push(disposal_check(used, cases, max.value()));
    }
    out
}

/// 推薬が複数あり、軌道維持に使うスラスタを決められないという注意。
fn thruster_check(names: &[String]) -> Check {
    Check::new(
        "station_keeping_thruster",
        "Station-keeping thruster",
        Verdict::Warning,
        format!(
            "There are several propellants ({}), so the station-keeping thruster cannot be chosen and the ΔV and duration the propellant can sustain are not computed. \
             Add a maneuver with category station_keeping and a thruster",
            names.join(", ")
        ),
    )
}

/// 寿命の下限 [s]（再突入しなければ期間の上限）。
fn lower_bound(used: &DecayUsed, c: &DecayCase) -> f64 {
    c.lifetime.map_or(used.span.value(), |l| l.value())
}

/// ミッション期間より早く再突入する条件があれば注意（軌道維持が要る）。
fn mission_check(used: &DecayUsed, cases: &[DecayCase], mission: f64) -> Check {
    let short = cases
        .iter()
        .filter(|c| lower_bound(used, c) < mission)
        .min_by(|a, b| lower_bound(used, a).total_cmp(&lower_bound(used, b)));
    let (verdict, message) = match short {
        None => (
            Verdict::Ok,
            format!(
                "No condition re-enters within the mission duration of {}",
                quantity_text::<Time>(mission)
            ),
        ),
        Some(c) if c.lifetime.is_some() => (
            Verdict::Warning,
            format!(
                "Condition {} re-enters after {} and does not reach the mission duration of {} (orbit maintenance is required)",
                c.id,
                quantity_text::<Time>(lower_bound(used, c)),
                quantity_text::<Time>(mission)
            ),
        ),
        Some(_) => (
            Verdict::Warning,
            format!(
                "The integration span {} is shorter than the mission duration {}, so the lifetime cannot be confirmed",
                quantity_text::<Time>(used.span.value()),
                quantity_text::<Time>(mission)
            ),
        ),
    };
    // 余裕 = 最も短い寿命（の下限）/ ミッション期間 − 1。ミッション期間より早い条件があれば負
    let shortest = cases
        .iter()
        .map(|c| lower_bound(used, c))
        .min_by(f64::total_cmp);
    Check::new(
        "lifetime_vs_mission",
        "Orbit lifetime against mission duration",
        verdict,
        message,
    )
    .with_ratio_margin(shortest.and_then(|l| ratio_margin(l, mission)))
}

/// 推薬で高度を保てる期間がミッション期間に届かない条件があれば注意。
fn propellant_check(cases: &[DecayCase], mission: f64) -> Check {
    let worst = cases
        .iter()
        .filter_map(|c| c.station_keeping.propellant_life.map(|l| (c, l.value())))
        .min_by(|a, b| a.1.total_cmp(&b.1));
    let (verdict, message) = match worst {
        Some((c, life)) if life < mission => (
            Verdict::Warning,
            format!(
                "Condition {} keeps altitude on propellant for {}, which does not reach the mission duration of {}",
                c.id,
                quantity_text::<Time>(life),
                quantity_text::<Time>(mission)
            ),
        ),
        _ => (
            Verdict::Ok,
            format!(
                "Altitude can be held on propellant for the mission duration of {} in every condition",
                quantity_text::<Time>(mission)
            ),
        ),
    };
    // 余裕 = 最も短い推薬で保てる期間 / ミッション期間 − 1
    let margin = worst.and_then(|(_, life)| ratio_margin(life, mission));
    Check::new(
        "propellant_life",
        "Station-keeping propellant life",
        verdict,
        message,
    )
    .with_ratio_margin(margin)
}

/// 代表の条件で寿命の上限（廃棄の要求）を満たすか。上限までに積分が届かなければ注意。
///
/// 代表は日付で引いた条件（太陽周期の予測を含む）、無ければ中程度のプリセット。低いプリセットを何十年も保つのは
/// 太陽周期を無視した極端な仮定なので、廃棄の判定には使わない（ISO 27852 も太陽周期を通した活動の予測を使う）。
/// 低い・高いプリセットの寿命は条件ごとの結果に出る。
fn disposal_check(used: &DecayUsed, cases: &[DecayCase], max: f64) -> Check {
    let representative = cases
        .iter()
        .find(|c| c.id == DATED)
        .or_else(|| cases.iter().find(|c| c.id == MODERATE));
    let Some(c) = representative else {
        return Check::new(
            "max_lifetime",
            "Lifetime limit",
            Verdict::Ok,
            "No conditions",
        );
    };
    let life = lower_bound(used, c);
    let max_text = quantity_text::<Time>(max);
    let (verdict, message) = match c.lifetime {
        Some(_) if life <= max => (
            Verdict::Ok,
            format!(
                "Condition {} re-enters after {}, within the limit of {max_text}",
                c.id,
                quantity_text::<Time>(life)
            ),
        ),
        Some(_) => (
            Verdict::Violation,
            format!(
                "Condition {} takes {} to re-enter, exceeding the limit of {max_text}",
                c.id,
                quantity_text::<Time>(life)
            ),
        ),
        None if life >= max => (
            Verdict::Violation,
            format!(
                "Condition {} does not re-enter even after {}, exceeding the limit of {max_text}",
                c.id,
                quantity_text::<Time>(life)
            ),
        ),
        None => (
            Verdict::Warning,
            format!(
                "Condition {} does not re-enter within the integration span of {}, so the limit of {max_text} cannot be confirmed (extend the span)",
                c.id,
                quantity_text::<Time>(life)
            ),
        ),
    };
    // 余裕 = 上限 / 寿命（の下限）− 1。積分が届かず確かめられない（警告）ときも、下限に対する余裕を添える
    Check::new("max_lifetime", "Lifetime limit", verdict, message)
        .with_ratio_margin(ratio_margin(max, life))
}
