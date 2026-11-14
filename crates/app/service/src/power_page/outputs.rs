//! 電源のフロー図の解析のカードの主な出力（2〜3 行。名前付き）。値は解析の出力と KPI（`AnalysisRunResult.kpis`。代表値と
//! 判定の選び方は `analysis::kpi`）から読み、表示の文字列は `units` で作る。
use crate::dto::{AnalysisRunResult, FlowRow, KpiKey, KpiView};
use crate::units::{percent, quantity_auto};
use sat_analysis::power::energy_balance::EnergyBalanceOutput;
use sat_analysis::power::mode_load::ModeLoadOutput;
use sat_analysis::power::orbit_generation::OrbitGenerationOutput;
use sat_analysis::power::sizing::SizingOutput;
use sat_analysis::spec::Verdict;

/// 解析の主な出力の行（電源の 4 つの解析だけ。出力が無ければ None で、KPI の行のまま）。
pub(super) fn rows(r: &AnalysisRunResult) -> Option<Vec<FlowRow>> {
    let out = r.output.clone()?;
    let kpi = |key: KpiKey| r.kpis.iter().find(|k| k.key == key);
    Some(match r.id.as_str() {
        "power.mode_load" => mode_load(serde_json::from_value(out).ok()?),
        "power.orbit_generation" => {
            let o: OrbitGenerationOutput = serde_json::from_value(out).ok()?;
            vec![
                row(
                    "orbit_average_eol",
                    ("軌道平均の発電（EOL）", "Orbit-average generation (EOL)"),
                    quantity_auto(o.orbit_average_eol.value(), "Power").display,
                    None,
                ),
                row(
                    "sunlit_fraction",
                    ("日照率", "Sunlit fraction"),
                    percent(o.sunlit_fraction.value()).display,
                    None,
                ),
            ]
        }
        "power.energy_balance" => {
            let o: EnergyBalanceOutput = serde_json::from_value(out).ok()?;
            let dod = kpi(KpiKey::MaxDod);
            let balance = kpi(KpiKey::OrbitBalance);
            vec![
                row(
                    "max_dod",
                    ("最大 DoD / 許容", "Max DoD / allowed"),
                    format!("{} / {}", shown(dod), percent(o.dod_limit.value()).display),
                    dod.and_then(|k| k.verdict),
                ),
                row(
                    "orbit_balance",
                    ("1 周の収支", "Energy margin per orbit"),
                    shown(balance),
                    balance.and_then(|k| k.verdict),
                ),
            ]
        }
        "power.sizing" => {
            let o: SizingOutput = serde_json::from_value(out).ok()?;
            let pair = |need: f64, have: f64, dim: &str| {
                format!(
                    "{} / {}",
                    quantity_auto(need, dim).display,
                    quantity_auto(have, dim).display
                )
            };
            vec![
                row(
                    "area",
                    (
                        "太陽電池の面積 必要 / 設計",
                        "Solar array area required / design",
                    ),
                    pair(o.required_area.value(), o.design_area.value(), "Area"),
                    kpi(KpiKey::SapMargin).and_then(|k| k.verdict),
                ),
                row(
                    "capacity",
                    ("電池容量 必要 / 設計", "Battery capacity required / design"),
                    pair(
                        o.required_capacity.value(),
                        o.design_capacity.value(),
                        "Energy",
                    ),
                    kpi(KpiKey::BatteryMargin).and_then(|k| k.verdict),
                ),
            ]
        }
        _ => return None,
    })
}

/// モードの消費の最大と最小（成長余裕込み。どのモードか添える。モードの名前は日本語は表示名、英語は ID）。
fn mode_load(o: ModeLoadOutput) -> Vec<FlowRow> {
    let named =
        |id: &str, title: (&str, &str), m: &sat_analysis::power::mode_load::ModeLoadMode| {
            let power = quantity_auto(m.total_with_growth.value(), "Power").display;
            let ja = if m.label.is_empty() { &m.id } else { &m.label };
            let mut r = row(id, title, format!("{power} · {ja}"), None);
            r.display_en = Some(format!("{power} · {}", m.id));
            r
        };
    let by = |a: &&sat_analysis::power::mode_load::ModeLoadMode,
              b: &&sat_analysis::power::mode_load::ModeLoadMode| {
        a.total_with_growth
            .value()
            .total_cmp(&b.total_with_growth.value())
    };
    let mut rows = Vec::new();
    if let Some(m) = o.modes.iter().max_by(by) {
        rows.push(named("max_mode", ("最大のモード", "Highest mode"), m));
    }
    if let Some(m) = o.modes.iter().min_by(by) {
        rows.push(named("min_mode", ("最小のモード", "Lowest mode"), m));
    }
    rows
}

fn shown(k: Option<&KpiView>) -> String {
    k.and_then(|k| k.value.as_ref())
        .map_or_else(|| "—".to_owned(), |v| v.display.clone())
}

fn row(id: &str, (ja, en): (&str, &str), display: String, verdict: Option<Verdict>) -> FlowRow {
    FlowRow {
        id: id.to_owned(),
        label: ja.to_owned(),
        label_en: en.to_owned(),
        display: Some(display),
        set: true,
        derived: false,
        display_en: None,
        varies: None,
        verdict,
        target: None,
    }
}
