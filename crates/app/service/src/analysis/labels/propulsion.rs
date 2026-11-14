//! 推進系の解析の出力の列挙の表示名。

use super::label;
use crate::dto::AnalysisLabel;
use fune_projection::propulsion::{model_category, model_method};
use sat_analysis::propulsion::{DvCategory, ManeuverNote, TransferMethod, UnloadingNote};

/// 設計の語彙（`maneuver_category`・`maneuver_method`）の表示名。**区分と計算の段階の表示名は labels.toml だけ**。
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
        ManeuverNote::FiniteBurnTooLong => label(
            "噴射が軌道の半周を超えるので、損失を見込まずインパルスの ΔV を使った",
            "The burn exceeds half an orbit, so the impulsive delta-V is used without losses",
        ),
        ManeuverNote::PatchedConic => label(
            "天体間の遷移をパッチドコニック（天体の円軌道どうしのホーマン遷移）で計算した（打ち上げの窓は見ない）",
            "Computed as a patched-conic transfer (Hohmann between circular body orbits; launch windows are not considered)",
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
