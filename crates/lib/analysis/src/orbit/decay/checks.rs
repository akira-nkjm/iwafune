//! 高度低下の判定: 近円軌道の前提・ミッション期間に対する寿命・廃棄の寿命の上限・推薬の寿命。
//! 判定文の数は `spec::quantity_text` で書く。条件は ID（`low`・`moderate`・`high`・`dated`）で名指す。

use super::{DATED, DecayCase, DecayUsed, MODERATE, NEAR_CIRCULAR_ECCENTRICITY, OrbitDecayInput};
use crate::spec::{Check, Verdict, quantity_text};
use sat_units::typed::Time;

/// すべての判定（書かれた要求があるものだけ）。
pub(super) fn checks(input: &OrbitDecayInput, used: &DecayUsed, cases: &[DecayCase]) -> Vec<Check> {
    let mut out = Vec::new();
    let e = input.orbit.elements.eccentricity;
    if e > NEAR_CIRCULAR_ECCENTRICITY {
        out.push(Check::new(
            "near_circular",
            "近円軌道の前提",
            Verdict::Warning,
            format!(
                "離心率 {e} は近円軌道の前提（{NEAR_CIRCULAR_ECCENTRICITY} 以下）を超えます。離心率の変化を追わないので寿命は粗い見積もりです"
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
                "どの条件でもミッション期間 {} のあいだ再突入しません",
                quantity_text::<Time>(mission)
            ),
        ),
        Some(c) if c.lifetime.is_some() => (
            Verdict::Warning,
            format!(
                "条件 {} では {} で再突入し、ミッション期間 {} に届きません（軌道維持が要ります）",
                c.id,
                quantity_text::<Time>(lower_bound(used, c)),
                quantity_text::<Time>(mission)
            ),
        ),
        Some(_) => (
            Verdict::Warning,
            format!(
                "積分の期間 {} がミッション期間 {} より短く、寿命を確かめられません",
                quantity_text::<Time>(used.span.value()),
                quantity_text::<Time>(mission)
            ),
        ),
    };
    Check::new(
        "lifetime_vs_mission",
        "ミッション期間に対する軌道寿命",
        verdict,
        message,
    )
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
                "条件 {} では推薬で高度を保てるのが {} で、ミッション期間 {} に届きません",
                c.id,
                quantity_text::<Time>(life),
                quantity_text::<Time>(mission)
            ),
        ),
        _ => (
            Verdict::Ok,
            format!(
                "どの条件でも推薬でミッション期間 {} のあいだ高度を保てます",
                quantity_text::<Time>(mission)
            ),
        ),
    };
    Check::new("propellant_life", "軌道維持の推薬の寿命", verdict, message)
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
            "寿命の上限",
            Verdict::Ok,
            "条件がありません",
        );
    };
    let life = lower_bound(used, c);
    let max_text = quantity_text::<Time>(max);
    let (verdict, message) = match c.lifetime {
        Some(_) if life <= max => (
            Verdict::Ok,
            format!(
                "条件 {} では {} で再突入し、上限 {max_text} 以内です",
                c.id,
                quantity_text::<Time>(life)
            ),
        ),
        Some(_) => (
            Verdict::Violation,
            format!(
                "条件 {} では再突入まで {} かかり、上限 {max_text} を超えます",
                c.id,
                quantity_text::<Time>(life)
            ),
        ),
        None if life >= max => (
            Verdict::Violation,
            format!(
                "条件 {} では {} たっても再突入せず、上限 {max_text} を超えます",
                c.id,
                quantity_text::<Time>(life)
            ),
        ),
        None => (
            Verdict::Warning,
            format!(
                "条件 {} では積分の期間 {} のうちに再突入せず、上限 {max_text} を満たすか確かめられません（期間を延ばしてください）",
                c.id,
                quantity_text::<Time>(life)
            ),
        ),
    };
    Check::new("max_lifetime", "寿命の上限", verdict, message)
}
