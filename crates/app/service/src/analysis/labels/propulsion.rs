//! 推進系の解析の出力の列挙の表示名。

use super::label;
use crate::dto::AnalysisLabel;
use fune_projection::propulsion::{model_category, model_method, model_steering};
use sat_analysis::power::thrust_balance::ThrustBalanceNote;
use sat_analysis::propulsion::{
    DvCategory, LaunchWindowNote, LaunchWindowPointKind, LaunchWindowQuantity, ManeuverNote,
    TransferCheckNote, TransferMethod, UnloadingNote,
};
use sat_astro::ephem::EphemerisSource;
use sat_astro::force::thrust::ThrustSteering;

/// 設計の語彙（`maneuver_category`・`maneuver_method`・`maneuver_steering`）の表示名。**区分と計算の段階の表示名は labels.toml だけ**。
fn vocab(ty: &str, value: &str) -> AnalysisLabel {
    let l = fune_model::builtin::enum_label(ty, value);
    label(
        l.map_or(value, |l| l.label.as_str()),
        l.and_then(|l| l.label_en.as_deref()).unwrap_or(value),
    )
}

pub(super) fn category(c: DvCategory) -> AnalysisLabel {
    vocab("maneuver_category", model_category(c).as_str())
}

pub(super) fn method(m: TransferMethod) -> AnalysisLabel {
    vocab("maneuver_method", model_method(m).as_str())
}

pub(super) fn steering(s: ThrustSteering) -> AnalysisLabel {
    vocab("maneuver_steering", model_steering(s).as_str())
}

pub(super) fn thrust_balance_note(n: ThrustBalanceNote) -> AnalysisLabel {
    match n {
        ThrustBalanceNote::DutyAsFullSunlit => label(
            "日照中だけ噴くデューティを、日照中ずっと噴くとみなした（保守側）",
            "The duty while sunlit is treated as firing through the whole sunlit arc (conservative)",
        ),
        ThrustBalanceNote::DutyAsBlockPerOrbit => label(
            "デューティの分を 1 周の中でまとめて噴くとみなした（保守側）",
            "The duty is treated as one firing block per orbit (conservative)",
        ),
    }
}

pub(super) fn transfer_check_note(n: TransferCheckNote) -> AnalysisLabel {
    match n {
        TransferCheckNote::PointMassCruise => label(
            "出発天体の中心から Lambert の速度で始め、出発・到着の天体の重力は入れず、ほかの惑星を点質量の第三体として伝播した",
            "Propagated from the departure body's centre with the Lambert velocity, without the gravity of the departure and arrival bodies, and with the other planets as point-mass third bodies",
        ),
        TransferCheckNote::AnalyticEphemerisNotJudged => label(
            "天体の位置が解析式（DE440 でない）なので、ずれは暦の誤差と同程度で意味を持たず、判定しない",
            "Body positions are analytic (not DE440), so the miss is of the order of the ephemeris error and is not judged",
        ),
        TransferCheckNote::LegsNotChained => label(
            "スイングバイのある遷移なので、脚ごとに伝播した（各脚は自分の Lambert の速度から始まる。通過の天体の近くの双曲線の弧と、通過の前後をつなげた伝播は検証しない）",
            "The transfer has swing-bys, so each leg was propagated separately from its own Lambert velocity (the hyperbolic arc near the flyby body and a propagation chained through the flyby are not verified)",
        ),
        TransferCheckNote::ClosestApproachAtEnd => label(
            "最接近が探した期間の端で、まだ近づいている（極小は期間の外）",
            "The closest approach is at the end of the searched span: still approaching (the minimum lies outside it)",
        ),
    }
}

pub(super) fn launch_window_note(n: LaunchWindowNote) -> AnalysisLabel {
    match n {
        LaunchWindowNote::AnalyticEphemeris => label(
            "天体の位置が解析式（DE440 でない）なので、火星で 10⁵ km 級の誤差があり、最小の日付も数日ずれうる",
            "Body positions are analytic (not DE440): errors of the order of 10^5 km for Mars, so the best dates may shift by days",
        ),
        LaunchWindowNote::DeclinationAboveInclination => label(
            "ΔV が最小の点の出発の漸近線の赤緯がパーキング軌道の傾斜角より大きい（軌道面の変更は数えない）",
            "The departure asymptote declination of the minimum delta-V point is above the parking orbit inclination (the plane change is not counted)",
        ),
        LaunchWindowNote::MinimumAtGridEdge => label(
            "ΔV が最小の点が格子の端（軸の端・飛行時間の範囲の端・解けない点の隣）にある。窓や飛行時間の範囲を広げると下がるかもしれない",
            "The minimum delta-V point is at the edge of the grid (axis end, time-of-flight limit or next to an unsolved point): widening the window or the time-of-flight range may lower it",
        ),
        LaunchWindowNote::WrittenDatesUnsolved => label(
            "マヌーバに書かれた日付では遷移が解けない",
            "The transfer cannot be solved for the dates written in the maneuver",
        ),
        LaunchWindowNote::WrittenFlybyInfeasible => label(
            "マヌーバに書かれた日付では、通過のどれかが下限の高度で曲がりきれない（`propulsion.maneuvers` は足りない偏向の分を噴射に足して警告する）",
            "At the dates written in the maneuver a flyby cannot turn enough at its minimum altitude (propulsion.maneuvers adds the missing turn to the burn and warns)",
        ),
        LaunchWindowNote::UnpoweredFlybyNotBallistic => label(
            "動力なしと書かれた通過の入りと出の超過速度が最小の点で揃っていない。その近点の噴射は ΔV に数えていないので、最小の ΔV はその分だけ小さい",
            "The in and out excess speeds of a flyby declared unpowered differ at the minimum point. That periapsis burn is not counted in delta-V, so the minimum is lower by that amount",
        ),
        LaunchWindowNote::RefinementIncomplete => label(
            "全部の日付の詰めが評価の上限で止まった。最小はまだ下がるかもしれない",
            "The refinement of all dates stopped at its evaluation limit: the minimum may still be lowered",
        ),
    }
}

pub(super) fn launch_window_point_kind(k: LaunchWindowPointKind) -> AnalysisLabel {
    match k {
        LaunchWindowPointKind::MinDeltaV => label("ΔV が最小の点", "Minimum delta-V point"),
        LaunchWindowPointKind::MinC3 => label("C3 が最小の点", "Minimum C3 point"),
        LaunchWindowPointKind::Written => label("書かれた日付", "Written dates"),
    }
}

pub(super) fn launch_window_quantity(q: LaunchWindowQuantity) -> AnalysisLabel {
    match q {
        LaunchWindowQuantity::C3 => label("出発の C3", "Departure C3"),
        LaunchWindowQuantity::VInfDeparture => label("出発の超過速度", "Departure excess speed"),
        LaunchWindowQuantity::VInfArrival => label("到着の超過速度", "Arrival excess speed"),
        LaunchWindowQuantity::DepartureDeclination => {
            label("出発の漸近線の赤緯", "Departure asymptote declination")
        }
        LaunchWindowQuantity::DeltaV => label("マヌーバの ΔV", "Maneuver delta-V"),
    }
}

pub(super) fn ephemeris_source(s: EphemerisSource) -> AnalysisLabel {
    match s {
        EphemerisSource::De440 => label("JPL DE440", "JPL DE440"),
        EphemerisSource::Analytic => label(
            "解析式（VSOP87A・Standish。DE440 より少し粗い）",
            "Analytic elements (slightly less accurate than DE440)",
        ),
    }
}

pub(super) fn note(n: ManeuverNote) -> AnalysisLabel {
    match n {
        ManeuverNote::BiElliptic => label(
            "中間の遠地点を通るバイエリプティック遷移で計算した",
            "Computed as a bi-elliptic transfer through the intermediate apogee",
        ),
        ManeuverNote::ViaAltitudeIgnored => label(
            "低推力では中間の遠地点を使わない",
            "The intermediate apogee is not used for low thrust",
        ),
        ManeuverNote::LowThrustCircularApproximation => label(
            "Edelbaum の式は円軌道どうしの式なので、長半径の円軌道で近似した",
            "Edelbaum's equation is for circular orbits, so the orbits are approximated by circles of the same semi-major axis",
        ),
        ManeuverNote::NoThruster => label(
            "スラスタが無いので ΔV だけを求めた（推薬・噴射時間は求めない）",
            "No thruster, so only the delta-V is computed (no propellant or burn time)",
        ),
        ManeuverNote::WrittenDeltaV => label(
            "ΔV は設計データに書かれた値（余裕）で、軌道は変わらない",
            "The delta-V is the value written in the design (an allowance); the orbit does not change",
        ),
        ManeuverNote::PropagationNotReached => label(
            "数値伝播が上限の時間までに目標に届かなかった（届いたところまでの値）",
            "The propagation did not reach the target within the time limit (values up to where it got)",
        ),
        ManeuverNote::FiniteBurnTooLong => label(
            "噴射が軌道の半周を超えるので、損失を見込まずインパルスの ΔV を使った",
            "The burn exceeds half an orbit, so the impulsive delta-V is used without losses",
        ),
        ManeuverNote::PatchedConic => label(
            "天体間の遷移をパッチドコニック（天体の円軌道どうしのホーマン遷移）で計算した（打ち上げの窓は見ない）",
            "Computed as a patched-conic transfer (Hohmann between circular body orbits; launch windows are not considered)",
        ),
        ManeuverNote::DatedLambert => label(
            "天体間の遷移を、出発の日・到着の日の天体の位置からの Lambert（日付つきのパッチドコニック）で計算した",
            "Computed as a dated patched-conic transfer: a Lambert solution between the body positions on the departure and arrival dates",
        ),
        ManeuverNote::SwingBy => label(
            "途中の惑星のスイングバイをパッチドコニックで計算した（通過は一瞬で、影響圏の中の時間・双曲線の弧は扱わない。近点の噴射は ΔV に数える）",
            "The swing-bys were computed by patched conics (each flyby is instantaneous; the time inside the sphere of influence and the hyperbolic arc are not modelled; the periapsis burns are counted in the delta-V)",
        ),
        ManeuverNote::DepartureByLauncher => label(
            "出発の噴射は打ち上げ機が行うので、ΔV・推薬に数えない（C3・超過速度は打ち上げの要求）",
            "The departure burn is provided by the launcher, so it is not counted in delta-V or propellant (C3 and excess speed are launch requirements)",
        ),
        ManeuverNote::PlaneChangeIgnored => label(
            "天体間の遷移では傾斜角の変更の ΔV を数えない",
            "Inclination change delta-V is not counted for transfers between bodies",
        ),
    }
}

pub(super) fn unloading(n: UnloadingNote) -> AnalysisLabel {
    match n {
        UnloadingNote::NoRcs => label(
            "RCS が無いので、アンロードの ΔV は含めない",
            "No RCS, so unloading delta-V is not included",
        ),
        UnloadingNote::UnloadingInputsMissing => label(
            "aocs.rw_unloading の入力が足りないので、アンロードの ΔV は含めない",
            "aocs.rw_unloading lacks inputs, so unloading delta-V is not included",
        ),
        UnloadingNote::NoMissionDuration => label(
            "フェーズの期間もミッション期間も無いので、アンロードの ΔV は含めない",
            "No phase duration or mission duration, so unloading delta-V is not included",
        ),
        UnloadingNote::NotUnloadable => label(
            "RCS で捨てられない向きがあるので、アンロードの ΔV は含めない",
            "The RCS cannot unload every direction, so unloading delta-V is not included",
        ),
    }
}
