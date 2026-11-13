//! `analysis/thermal.toml` の検査: 表面材と面積割合・発熱の場所・解析マージンの温度差の単位。

use fune_model::design::{Part, ThermalCoatingSettings, ThermalMarginSettings, ThermalSettings};
use fune_model::value::Quantity;
use fune_model::{Design, Severity};

fn design(t: ThermalSettings) -> Design {
    let mut d = Design::default();
    let part = Part::new("PANEL");
    d.parts.insert(part.id.clone(), part);
    d.analysis.thermal = Some(t);
    d
}

fn codes(d: &Design) -> Vec<(Severity, String)> {
    fune_model::check(d)
        .into_iter()
        .filter(|x| x.code.starts_with("thermal.") || x.code.starts_with("ref."))
        .map(|x| (x.severity, x.code))
        .collect()
}

fn coating(a: f64, e: f64) -> ThermalCoatingSettings {
    ThermalCoatingSettings {
        label: None,
        absorptivity: Some(a),
        emissivity: Some(e),
    }
}

/// 面積割合の合計が 1 なら問題なし。合計が 1 でなければ警告（1 つずつ書いている途中でも書き込める）、
/// 無い表面材・無い部品は誤り。
#[test]
fn surface_coatings_are_checked() {
    let mut t = ThermalSettings::default();
    t.coating.insert("white".into(), coating(0.2, 0.85));
    t.coating.insert("black".into(), coating(0.92, 0.86));
    t.surface.insert(
        "PANEL".into(),
        [("white".to_owned(), 0.7), ("black".to_owned(), 0.3)].into(),
    );
    assert!(codes(&design(t.clone())).is_empty());
    t.surface
        .get_mut("PANEL")
        .unwrap()
        .insert("black".into(), 0.2);
    assert_eq!(
        codes(&design(t.clone())),
        [(Severity::Warning, "thermal.coating_share".to_owned())]
    );
    t.surface
        .insert("NOPE".into(), [("gold".to_owned(), 1.0)].into());
    let found = codes(&design(t));
    assert!(found.contains(&(Severity::Error, "ref.part".to_owned())));
    assert!(found.contains(&(Severity::Error, "ref.coating".to_owned())));
}

/// 発熱の場所は存在する部品と節点を指す。
#[test]
fn heat_node_refers_to_a_node() {
    let mut t = ThermalSettings::default();
    t.nodes.insert("N".into(), vec!["PANEL".into()]);
    t.heat_node.insert("PANEL".into(), "N".into());
    assert!(codes(&design(t.clone())).is_empty());
    t.heat_node.insert("PANEL".into(), "MISSING".into());
    assert_eq!(
        codes(&design(t)),
        [(Severity::Error, "ref.thermal_node".to_owned())]
    );
}

/// 解析マージンは温度差なので K で書く（degC は 273.15 K のずれになるので誤り）。
#[test]
fn margin_is_a_temperature_difference_in_kelvin() {
    let margin = |unit: &str| ThermalSettings {
        margin: Some(ThermalMarginSettings {
            uncertainty: Some(Quantity::new(5.0, unit)),
            warning_band: None,
        }),
        ..ThermalSettings::default()
    };
    assert!(codes(&design(margin("K"))).is_empty());
    assert_eq!(
        codes(&design(margin("degC"))),
        [(Severity::Error, "thermal.difference_unit".to_owned())]
    );
}
