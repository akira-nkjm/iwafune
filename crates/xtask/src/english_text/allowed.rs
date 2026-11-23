//! 日本語の文字列リテラルを残してよいファイルの表（データ）。

/// 各解析が持つ日本語と英語の題・説明の対（`TITLE` / `TITLE_EN`、`DESCRIPTION` / `DESCRIPTION_EN`）。
const ANALYSIS_TEXT: &str =
    "bilingual analysis title and description (TITLE / TITLE_EN, DESCRIPTION / DESCRIPTION_EN)";

/// 日本語のリテラルを残してよいファイル（日本語と英語の表示名の対を持つもの）と、その理由。ディレクトリ単位では許さない。
pub(super) const ALLOWED: &[(&str, &str)] = &[
    (
        "crates/app/model/src/builtin/labels.rs",
        "Japanese and English display name of the fallback role group (Label / label_en)",
    ),
    (
        "crates/app/model/src/design/conditions.rs",
        "Japanese display name of a condition group (label / label_en)",
    ),
    (
        "crates/app/report/src/labels.rs",
        "bilingual report headings, captions and notes, pair(ja, en)",
    ),
    (
        "crates/app/service/src/analysis/labels.rs",
        "bilingual analysis-view labels, label(ja, en) tables",
    ),
    (
        "crates/app/service/src/analysis/labels/orbit.rs",
        "bilingual analysis-view labels, label(ja, en) tables",
    ),
    (
        "crates/app/service/src/analysis/labels/propulsion.rs",
        "bilingual analysis-view labels, label(ja, en) tables",
    ),
    (
        "crates/app/service/src/analysis/overview/power.rs",
        "bilingual power overview chart labels, label(ja, en)",
    ),
    (
        "crates/app/service/src/analysis/overview/power/meter.rs",
        "bilingual power meter labels, label(ja, en)",
    ),
    (
        "crates/app/service/src/system_modes/power/budget.rs",
        "bilingual power budget labels, label(ja, en)",
    ),
    (
        "crates/app/service/src/system_modes/power/charts.rs",
        "bilingual power chart labels, label(ja, en)",
    ),
    (
        "crates/app/service/src/attitude_view.rs",
        "bilingual attitude axis names (ja, en)",
    ),
    (
        "crates/lib/analysis/src/aocs/rw_unloading.rs",
        ANALYSIS_TEXT,
    ),
    ("crates/lib/analysis/src/aocs/sensor_fov.rs", ANALYSIS_TEXT),
    ("crates/lib/analysis/src/aocs/slew_time.rs", ANALYSIS_TEXT),
    ("crates/lib/analysis/src/aocs/unload_time.rs", ANALYSIS_TEXT),
    ("crates/lib/analysis/src/cdh/data_budget.rs", ANALYSIS_TEXT),
    (
        "crates/lib/analysis/src/comm/data_throughput.rs",
        ANALYSIS_TEXT,
    ),
    ("crates/lib/analysis/src/comm/link_budget.rs", ANALYSIS_TEXT),
    ("crates/lib/analysis/src/comm/visibility.rs", ANALYSIS_TEXT),
    ("crates/lib/analysis/src/orbit/decay.rs", ANALYSIS_TEXT),
    (
        "crates/lib/analysis/src/orbit/eclipse_season.rs",
        ANALYSIS_TEXT,
    ),
    (
        "crates/lib/analysis/src/power/energy_balance.rs",
        ANALYSIS_TEXT,
    ),
    ("crates/lib/analysis/src/power/mode_load.rs", ANALYSIS_TEXT),
    (
        "crates/lib/analysis/src/power/orbit_generation.rs",
        ANALYSIS_TEXT,
    ),
    ("crates/lib/analysis/src/power/sizing.rs", ANALYSIS_TEXT),
    (
        "crates/lib/analysis/src/power/solar_sunlit.rs",
        ANALYSIS_TEXT,
    ),
    (
        "crates/lib/analysis/src/power/thrust_balance.rs",
        ANALYSIS_TEXT,
    ),
    (
        "crates/lib/analysis/src/propulsion/dv_budget.rs",
        ANALYSIS_TEXT,
    ),
    (
        "crates/lib/analysis/src/propulsion/maneuvers.rs",
        ANALYSIS_TEXT,
    ),
    (
        "crates/lib/analysis/src/propulsion/mass_properties.rs",
        ANALYSIS_TEXT,
    ),
    (
        "crates/lib/analysis/src/propulsion/orbit_history.rs",
        ANALYSIS_TEXT,
    ),
    (
        "crates/lib/analysis/src/propulsion/propellant.rs",
        ANALYSIS_TEXT,
    ),
    (
        "crates/lib/analysis/src/propulsion/thruster_usage.rs",
        ANALYSIS_TEXT,
    ),
    (
        "crates/lib/analysis/src/structure/launch_fit.rs",
        ANALYSIS_TEXT,
    ),
    (
        "crates/lib/analysis/src/structure/mass_properties.rs",
        ANALYSIS_TEXT,
    ),
    (
        "crates/lib/analysis/src/structure/modal_estimate.rs",
        ANALYSIS_TEXT,
    ),
    (
        "crates/lib/analysis/src/structure/modal_fem.rs",
        ANALYSIS_TEXT,
    ),
    (
        "crates/lib/analysis/src/structure/static_load.rs",
        ANALYSIS_TEXT,
    ),
    ("crates/lib/analysis/src/thermal/steady.rs", ANALYSIS_TEXT),
    (
        "crates/lib/analysis/src/thermal/transient.rs",
        ANALYSIS_TEXT,
    ),
    (
        "crates/lib/units/src/typed/decibel.rs",
        "Japanese rustdoc text passed through a macro (documentation, not output)",
    ),
];
