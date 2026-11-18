//! 単位の一覧・入力文字列の解析・量の表示。**単位の解析と次元の判定と整形は `sat_units`（整形は `sat_units::format`）だけが行い、
//! 「数値＋単位」の文字列の分割は `fune_model::value::parse_number_unit` が行う。ここはそれらを呼んで表示用の型にする。**

use crate::dto::{
    DimensionInfo, FormatRequest, FormatResponse, ParseQuantityRequest, ParsedQuantity,
    QuantityView, UnitInfo, UnitMapping, UnitsCatalog, UnitsQuery,
};
use crate::error::{Result, ServiceError};
use fune_model::value::parse_number_unit;
use sat_units::format::{
    FormattedQuantity, SIGNIFICANT_DIGITS, canonical_dimension_name, dimension_label_preferring,
    display_units, format_dimension, format_in_unit, format_number, format_quantity,
    si_unit_expression,
};
use sat_units::typed::{NamedKind, TemperatureDifferenceKind, TorqueKind};
use sat_units::{Dimension, Unit, named_dimensions, parse_dimension, parse_unit};

/// SI 値の一括整形（`POST /units/format`）。単位の指定が無ければ次元名から読みやすい単位を選ぶ（`sat_units::format`）。
/// 結果はほかの API と同じ `QuantityView`。どれか 1 つでも次元名・単位が読めなければ、何番目かを添えてエラーにする。
pub fn format_quantities(req: &FormatRequest) -> Result<FormatResponse> {
    let items = req
        .items
        .iter()
        .enumerate()
        .map(|(i, item)| {
            let formatted = match item.unit.as_deref() {
                Some(unit) => {
                    let u = parse_unit(unit).map_err(|e| unit_error(unit, e))?;
                    if u.dimension() != dimension_of(&item.dimension)? {
                        return Err(ServiceError::Unit {
                            input: unit.to_owned(),
                            reason: format!("#{}: not a unit of dimension {}", i, item.dimension),
                        });
                    }
                    let name = dimension_label_preferring(u.dimension(), Some(&item.dimension));
                    format_in_unit(item.si, unit).map(|f| (f, name))
                }
                None => format_quantity(item.si, &item.dimension).map(|f| {
                    let name = canonical_dimension_name(&item.dimension).unwrap_or(&item.dimension);
                    (f, name.to_owned())
                }),
            };
            formatted
                .map(|(f, dimension)| view(item.si, f, dimension))
                .map_err(|e| ServiceError::Unit {
                    input: item.dimension.clone(),
                    reason: format!("#{i}: {e}"),
                })
        })
        .collect::<Result<Vec<_>>>()?;
    Ok(FormatResponse { items })
}

/// 単位の一覧（`GET /units`）。`query.dimension` を指定するとその次元の単位だけを返す。
pub fn units_catalog(query: &UnitsQuery) -> Result<UnitsCatalog> {
    let wanted = query.dimension.as_deref().map(dimension_of).transpose()?;
    // 同次元の別名（Torque は Energy、TemperatureDifference は Temperature と同次元）は、その名前と表示単位だけを返す。
    let alias = query
        .dimension
        .as_deref()
        .and_then(canonical_dimension_name)
        .filter(|n| [TorqueKind::name(), TemperatureDifferenceKind::name()].contains(n));
    let dimensions = named_dimensions()
        .iter()
        .filter(|(name, d)| wanted.is_none_or(|w| w == *d) && alias.is_none_or(|a| *name == a))
        .map(|(name, d)| DimensionInfo {
            name: (*name).to_owned(),
            si_unit: si_unit_expression(*d),
            display_units: display_units(name)
                .map(|u| u.map(str::to_owned).collect())
                .unwrap_or_default(),
        })
        .collect();
    let symbols: Vec<&str> = match alias {
        Some(a) => display_units(a).into_iter().flatten().collect(),
        None => sat_units::NAMED_UNIT_SYMBOLS.to_vec(),
    };
    let units = symbols
        .iter()
        .filter_map(|symbol| parse_unit(symbol).ok().map(|u| (*symbol, u)))
        .filter(|(_, u)| wanted.is_none_or(|w| w == u.dimension()))
        .map(|(symbol, u)| unit_info(symbol, &u, query.dimension.as_deref()))
        .collect();
    Ok(UnitsCatalog { dimensions, units })
}

/// 入力文字列（`"18 mN*m*s"`）を解析する（`POST /units/parse`）。
pub fn parse_quantity(req: &ParseQuantityRequest) -> Result<ParsedQuantity> {
    let (number, unit_text) = parse_number_unit(&req.input).map_err(|e| ServiceError::Unit {
        input: req.input.clone(),
        reason: e.to_string(),
    })?;
    let unit = parse_unit(&unit_text).map_err(|e| unit_error(&req.input, e))?;
    if let Some(expected) = req.dimension.as_deref() {
        let want = dimension_of(expected)?;
        if want != unit.dimension() {
            return Err(ServiceError::Unit {
                input: req.input.clone(),
                reason: format!(
                    "Dimension is {} but {expected} is required",
                    sat_units::format::dimension_label(unit.dimension())
                ),
            });
        }
    }
    let quantity = quantity_from_authored(number, &unit_text, req.dimension.as_deref())?;
    // 同じ量を SI の組立単位で（`sat_units::format::si_unit_expression`。無次元は単位なし）
    let si_unit = si_unit_expression(unit.dimension());
    let formatted = format_in_unit(quantity.si, &si_unit).unwrap_or_else(|_| FormattedQuantity {
        value: quantity.si,
        unit: si_unit.clone(),
        text: format_number(quantity.si, SIGNIFICANT_DIGITS),
    });
    let si = view(quantity.si, formatted, quantity.dimension.clone());
    Ok(ParsedQuantity { quantity, si })
}

/// SI 値から表示用の量を作る。`display_unit` があればその単位で、無ければ次元名から単位を選んで整形する。
pub fn quantity_from_si(
    si: f64,
    display_unit: Option<&str>,
    dimension: Option<&str>,
) -> QuantityView {
    let unit_dim = display_unit
        .filter(|u| !u.is_empty())
        .and_then(|u| parse_unit(u).ok().map(|unit| (u, unit.dimension())));
    let formatted = match (unit_dim, dimension) {
        (Some((u, _)), _) => format_in_unit(si, u).ok(),
        (None, Some(d)) => format_quantity(si, d).ok(),
        (None, None) => None,
    };
    let formatted = formatted.unwrap_or_else(|| FormattedQuantity {
        value: si,
        unit: display_unit.unwrap_or_default().to_owned(),
        text: format_number(si, SIGNIFICANT_DIGITS),
    });
    let dim_name = match unit_dim {
        Some((_, d)) => dimension_label_preferring(d, dimension),
        None => dimension
            .and_then(canonical_dimension_name)
            .unwrap_or_default()
            .to_owned(),
    };
    view(si, formatted, dim_name)
}

/// 割合（0〜1 の比）を `%` で表示する（例: 0.1 → `"10.0 %"`）。
pub fn percent(ratio: f64) -> QuantityView {
    quantity_from_si(ratio, Some("%"), Some("Dimensionless"))
}

/// `unit` で表した値を SI に直し、次元名から読みやすい単位で表示する（例: 20715072 `B` → `"20.7 MB"`）。
pub fn auto_from(value: f64, unit: &str, dimension: &str) -> Option<QuantityView> {
    let si = parse_unit(unit).ok()?.to_si(value);
    Some(quantity_auto(si, dimension))
}

/// SI 値と次元から表示用の量を作る（表示単位は自動で選ぶ）。
pub fn quantity_auto(si: f64, dimension: &str) -> QuantityView {
    quantity_from_si(si, None, Some(dimension))
}

/// 書かれたままの数値と単位から表示用の量を作る（表示は書かれた単位のまま）。
pub fn quantity_from_authored(
    number: f64,
    unit_text: &str,
    dimension: Option<&str>,
) -> Result<QuantityView> {
    let unit = parse_unit(unit_text).map_err(|e| unit_error(unit_text, e))?;
    let si = unit.to_si(number);
    let formatted = format_in_unit(si, unit_text).unwrap_or_else(|_| FormattedQuantity {
        value: number,
        unit: unit_text.to_owned(),
        text: format_number(number, SIGNIFICANT_DIGITS),
    });
    Ok(view(
        si,
        formatted,
        dimension_label_preferring(unit.dimension(), dimension),
    ))
}

/// 数値だけの表示（有効数字 3 桁）。
pub fn number_text(value: f64) -> String {
    format_number(value, SIGNIFICANT_DIGITS)
}

/// 次元だけから整形した表示文字列（次元名が分からないとき）。
pub fn display_dimension(si: f64, dim: Dimension) -> String {
    format_dimension(si, dim).text
}

fn view(si: f64, f: FormattedQuantity, dimension: String) -> QuantityView {
    QuantityView {
        value: f.value,
        unit: f.unit,
        si,
        dimension,
        display: f.text,
    }
}

fn dimension_of(name: &str) -> Result<Dimension> {
    parse_dimension(name)
        .map_err(|_| ServiceError::InvalidInput(format!("Unknown dimension: {name}")))
}

fn unit_error(input: &str, e: sat_units::UnitError) -> ServiceError {
    ServiceError::Unit {
        input: input.to_owned(),
        reason: e.to_string(),
    }
}

fn unit_info(symbol: &str, u: &Unit, preferred: Option<&str>) -> UnitInfo {
    let mapping = if u.is_logarithmic() {
        UnitMapping::Logarithmic
    } else if u.is_affine() {
        UnitMapping::Affine
    } else {
        UnitMapping::Linear
    };
    UnitInfo {
        symbol: symbol.to_owned(),
        dimension: dimension_label_preferring(u.dimension(), preferred),
        scale: u.scale(),
        offset: u.offset(),
        scale_text: number_text(u.scale()),
        mapping,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn torque_input_candidates_exclude_energy_units() {
        let attr = &fune_model::builtin::attributes()["max_torque"];
        assert_eq!(attr.dimension.as_deref(), Some(TorqueKind::name()));
        assert_eq!(attr.units, ["N*m", "mN*m"]);
        let catalog = units_catalog(&UnitsQuery {
            dimension: attr.dimension.clone(),
        })
        .unwrap();
        assert_eq!(catalog.dimensions.len(), 1);
        assert_eq!(catalog.dimensions[0].name, "Torque");
        let preferred: Vec<_> = display_units(TorqueKind::name()).unwrap().collect();
        let symbols: Vec<_> = catalog.units.iter().map(|u| u.symbol.as_str()).collect();
        assert_eq!(symbols, preferred);
        assert!(symbols.contains(&"N*m") && symbols.contains(&"mN*m"));
        assert!(!symbols.contains(&"Wh") && !symbols.contains(&"J"));
        // 次元の検査は従来どおり。入力の綴りだけでは量の種類を拒否しない。
        let parsed = parse_quantity(&ParseQuantityRequest {
            input: "1 J".into(),
            dimension: Some("Torque".into()),
        })
        .unwrap();
        assert_eq!(parsed.quantity.dimension, "Torque");
        let energy = units_catalog(&UnitsQuery {
            dimension: Some("Energy".into()),
        })
        .unwrap();
        assert!(energy.units.iter().any(|u| u.symbol == "J"));
        assert!(energy.units.iter().any(|u| u.symbol == "Wh"));
    }

    /// 温度差の入力候補は K だけ（絶対温度の degC・degF を混ぜない）。Temperature は従来どおり。
    #[test]
    fn temperature_difference_input_candidates_are_kelvin_only() {
        let delta = units_catalog(&UnitsQuery {
            dimension: Some("TemperatureDifference".into()),
        })
        .unwrap();
        assert_eq!(delta.dimensions.len(), 1);
        assert_eq!(delta.dimensions[0].name, "TemperatureDifference");
        let symbols: Vec<_> = delta.units.iter().map(|u| u.symbol.as_str()).collect();
        assert_eq!(symbols, ["K"]);
        let parsed = parse_quantity(&ParseQuantityRequest {
            input: "5 K".into(),
            dimension: Some("TemperatureDifference".into()),
        })
        .unwrap();
        assert_eq!(parsed.quantity.si, 5.0);
        assert_eq!(parsed.quantity.dimension, "TemperatureDifference");
        let absolute = units_catalog(&UnitsQuery {
            dimension: Some("Temperature".into()),
        })
        .unwrap();
        assert!(absolute.units.iter().any(|u| u.symbol == "degC"));
    }

    #[test]
    fn parses_value_and_unit() {
        let r = parse_quantity(&ParseQuantityRequest {
            input: "18 mN*m*s".into(),
            dimension: Some("AngularMomentum".into()),
        })
        .unwrap();
        assert_eq!(r.quantity.value, 18.0);
        assert_eq!(r.quantity.unit, "mN*m*s");
        assert!((r.quantity.si - 0.018).abs() < 1e-12);
        assert_eq!(r.quantity.dimension, "AngularMomentum");
        assert_eq!(r.quantity.display, "18.0 mN·m·s");
        // 同じ量の SI の組立単位での表示も、同じ `QuantityView` の形
        assert_eq!(r.si.display, "0.0180 m²·kg/s");
        assert_eq!(r.si.unit, "m^2*kg/s");
        assert_eq!(r.si.si, r.quantity.si);
        assert_eq!(r.si.dimension, "AngularMomentum");
    }

    #[test]
    fn rejects_wrong_dimension() {
        let e = parse_quantity(&ParseQuantityRequest {
            input: "1 kg".into(),
            dimension: Some("Power".into()),
        })
        .unwrap_err();
        assert_eq!(e.code(), "unit_error");
    }

    #[test]
    fn dimensionless_and_exponent() {
        let r = parse_quantity(&ParseQuantityRequest {
            input: "1.5e-3".into(),
            dimension: None,
        })
        .unwrap();
        assert_eq!(r.quantity.si, 1.5e-3);
        assert_eq!(r.quantity.dimension, "Dimensionless");
        assert_eq!(r.si.display, "0.00150");
        assert_eq!(r.si.unit, "");
    }

    #[test]
    fn catalog_filters_by_dimension() {
        let c = units_catalog(&UnitsQuery {
            dimension: Some("Power".into()),
        })
        .unwrap();
        assert!(c.units.iter().any(|u| u.symbol == "W"));
        assert!(c.units.iter().all(|u| u.dimension == "Power"));
        assert!(c.dimensions[0].display_units.iter().any(|u| u == "mW"));
    }

    #[test]
    fn batch_format() {
        use crate::dto::{FormatItem, FormatRequest};
        let r = format_quantities(&FormatRequest {
            items: vec![
                FormatItem {
                    si: 0.0123,
                    dimension: "Power".into(),
                    unit: None,
                },
                FormatItem {
                    si: 2.0e6,
                    dimension: "Information".into(),
                    unit: Some("Mbit".into()),
                },
            ],
        })
        .unwrap();
        // ほかの API と同じ `QuantityView`（SI 値・表示単位での値・次元名つき）
        assert_eq!(r.items[0].display, "12.3 mW");
        assert_eq!(r.items[0].unit, "mW");
        assert_eq!(r.items[0].si, 0.0123);
        assert_eq!(r.items[0].dimension, "Power");
        assert_eq!(r.items[1].display, "2.00 Mbit");
        assert_eq!(r.items[1].dimension, "Information");
        // 比は種類名 `Fraction` で % に（次元名の欄も種類名のまま）
        let fraction = format_quantities(&FormatRequest {
            items: vec![FormatItem {
                si: 0.25,
                dimension: "Fraction".into(),
                unit: None,
            }],
        })
        .unwrap();
        assert_eq!(fraction.items[0].display, "25.0 %");
        assert_eq!(fraction.items[0].dimension, "Fraction");
        let bad = FormatRequest {
            items: vec![FormatItem {
                si: 1.0,
                dimension: "Mass".into(),
                unit: Some("W".into()),
            }],
        };
        assert!(format_quantities(&bad).is_err());
    }

    #[test]
    fn auto_display_picks_prefix() {
        assert_eq!(quantity_auto(0.0123, "Power").display, "12.3 mW");
        assert_eq!(
            quantity_from_si(0.62, Some("g"), Some("Mass")).display,
            "620 g"
        );
    }
}
