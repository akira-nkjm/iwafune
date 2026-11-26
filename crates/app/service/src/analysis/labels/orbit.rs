//! 軌道の高度低下（`orbit.decay`）・ミッション幾何（`orbit.geometry`）と宇宙天気の列挙の表示名。

use super::label;
use crate::dto::AnalysisLabel;
use sat_analysis::orbit::decay::{DecayCondition, DragAreaSource};
use sat_analysis::orbit::{
    DesignMarginSource, DoseGeometry, DoseInterpolation, DoseLegStatus, DoseNotEvaluated,
    DoseSettingSource, EnvironmentSource, GeometrySpanBasis, NuclearAttenuation, ShieldingSource,
};
use sat_astro::atmosphere::space_weather::{SolarActivity, WeatherSource};
use sat_astro::radiation::{SolarCondition, Species};

pub(super) fn solar_activity(a: SolarActivity) -> AnalysisLabel {
    match a {
        SolarActivity::Low => label("低い太陽活動", "Low solar activity"),
        SolarActivity::Moderate => label("中程度の太陽活動", "Moderate solar activity"),
        SolarActivity::High => label("高い太陽活動", "High solar activity"),
    }
}

pub(super) fn geometry_span_basis(b: GeometrySpanBasis) -> AnalysisLabel {
    match b {
        GeometrySpanBasis::OneOff => label("軌道の 1 回限りの期間", "The orbit's one-off span"),
        GeometrySpanBasis::PhaseDuration => label("フェーズの期間", "The phase duration"),
        GeometrySpanBasis::OneCycle => label("軌道の 1 周", "One orbit"),
    }
}

pub(super) fn species(s: Species) -> AnalysisLabel {
    match s {
        Species::Proton => label("陽子（AP-8）", "Protons (AP-8)"),
        Species::Electron => label("電子（AE-8）", "Electrons (AE-8)"),
    }
}

pub(super) fn solar_condition(c: SolarCondition) -> AnalysisLabel {
    match c {
        SolarCondition::Minimum => label("太陽活動極小（1964）", "Solar minimum (1964)"),
        SolarCondition::Maximum => label("太陽活動極大（1970）", "Solar maximum (1970)"),
    }
}

pub(super) fn weather_source(w: WeatherSource) -> AnalysisLabel {
    match w {
        WeatherSource::Observed => label("観測値", "Observed"),
        WeatherSource::Interpolated => label("補間値", "Interpolated"),
        WeatherSource::DailyPredicted => label("日ごとの予測", "Daily prediction"),
        WeatherSource::MonthlyPredicted => label("月ごとの予測", "Monthly prediction"),
        WeatherSource::Preset => label("太陽活動のプリセット", "Solar-activity preset"),
        WeatherSource::Manual => label("手で与えた値", "Manual value"),
    }
}

pub(super) fn drag_area_source(s: DragAreaSource) -> AnalysisLabel {
    match s {
        DragAreaSource::Given => label("与えた面積", "Given area"),
        DragAreaSource::BodyBoxes => label(
            "部品の箱の和集合の、全方向の平均の投影面積",
            "Mean projected area over all directions of the union of the part boxes",
        ),
    }
}

pub(super) fn decay_condition(kind: &str) -> Option<AnalysisLabel> {
    // 内部タグの列挙はタグの値だけを見る（中身の太陽活動は別の欄の値として引く）
    let probe = match kind {
        "preset" => DecayCondition::Preset {
            activity: SolarActivity::Moderate,
        },
        "dated" => DecayCondition::Dated {
            fallback: SolarActivity::Moderate,
        },
        _ => return None,
    };
    Some(match probe {
        DecayCondition::Preset { .. } => label(
            "太陽活動のプリセットで一定",
            "Constant solar-activity preset",
        ),
        DecayCondition::Dated { .. } => label(
            "宇宙天気の表から日付で引いた値",
            "Dated values from the space-weather table",
        ),
    })
}

pub(super) fn design_margin_source(s: DesignMarginSource) -> AnalysisLabel {
    match s {
        DesignMarginSource::Given => label("設定の値", "Set in the design"),
        DesignMarginSource::Default => label("解析の既定", "Analysis default"),
    }
}

pub(super) fn shielding_source(s: ShieldingSource) -> AnalysisLabel {
    match s {
        ShieldingSource::Part => label("部品の値", "Part value"),
        ShieldingSource::Default => label("設計の既定", "Design default"),
    }
}

pub(super) fn dose_leg_status(s: DoseLegStatus) -> AnalysisLabel {
    match s {
        DoseLegStatus::Evaluated => label("評価した", "Evaluated"),
        DoseLegStatus::NoDuration => label("期間が無いので除いた", "Left out: no duration"),
        DoseLegStatus::NoEnvironment => {
            label("環境の表が無いので除いた", "Left out: no environment table")
        }
    }
}

pub(super) fn dose_not_evaluated(r: DoseNotEvaluated) -> AnalysisLabel {
    match r {
        DoseNotEvaluated::NoShielding => label("遮蔽厚さが無い", "No shielding thickness"),
        DoseNotEvaluated::OutsideTable => {
            label("環境の表の範囲の外", "Outside the environment table")
        }
    }
}

pub(super) fn dose_interpolation(i: DoseInterpolation) -> AnalysisLabel {
    match i {
        DoseInterpolation::LogLinear => label(
            "厚さについて線形、線量率は対数",
            "Linear in thickness, logarithmic in dose rate",
        ),
    }
}

pub(super) fn dose_geometry(g: DoseGeometry) -> AnalysisLabel {
    match g {
        DoseGeometry::SphereCentre => label(
            "アルミの中実の球の中心",
            "Centre of a solid aluminium sphere",
        ),
        DoseGeometry::SemiInfiniteSlab => {
            label("半無限のアルミの媒質", "Semi-infinite aluminium medium")
        }
        DoseGeometry::FiniteSlab => label(
            "有限のアルミの板の透過面",
            "Transmission surface of a finite aluminium slab",
        ),
    }
}

pub(super) fn nuclear_attenuation(n: NuclearAttenuation) -> AnalysisLabel {
    match n {
        NuclearAttenuation::None => label("核相互作用を見ない", "No nuclear attenuation"),
        NuclearAttenuation::Local => label(
            "核相互作用あり（二次粒子はその場に付与）",
            "Nuclear attenuation, secondary energy deposited locally",
        ),
        NuclearAttenuation::LocalWithNeutrons => label(
            "核相互作用あり（中性子の寄与を含む）",
            "Nuclear attenuation with neutron dose",
        ),
    }
}

pub(super) fn dose_setting_source(s: DoseSettingSource) -> AnalysisLabel {
    match s {
        DoseSettingSource::Given => label("指定した値", "Given"),
        DoseSettingSource::Default => label("解析の既定", "Analysis default"),
    }
}

pub(super) fn environment_source(s: EnvironmentSource) -> AnalysisLabel {
    match s {
        EnvironmentSource::UserTable => label("利用者の環境の表", "User-supplied table"),
        EnvironmentSource::ComputedTrapped => label(
            "捕捉粒子のモデルで計算（太陽陽子・宇宙線を含まない）",
            "Computed trapped-particle model (no solar protons or cosmic rays)",
        ),
    }
}
