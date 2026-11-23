//! 推進系の解析の出力の列挙の表示名。

use super::label;
use crate::dto::AnalysisLabel;
use fune_projection::propulsion::{model_category, model_method, model_steering};
use sat_analysis::power::thrust_balance::ThrustBalanceNote;
use sat_analysis::propulsion::{DvCategory, ManeuverNote, TransferMethod, UnloadingNote};
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
        ManeuverNote::PropagationNotReached => label(
            "数値伝搬が上限の時間までに目標に届かなかった（届いたところまでの値）",
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
            "ミッション期間が無いので、アンロードの ΔV は含めない",
            "No mission duration, so unloading delta-V is not included",
        ),
        UnloadingNote::NotUnloadable => label(
            "RCS で捨てられない向きがあるので、アンロードの ΔV は含めない",
            "The RCS cannot unload every direction, so unloading delta-V is not included",
        ),
    }
}
