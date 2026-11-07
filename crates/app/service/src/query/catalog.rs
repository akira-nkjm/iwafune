//! カタログ品の表示（`GET /design` の `catalog`）。値は部品の値と同じ組み立て（`values::value_view`）で作る。

use crate::dto::CatalogItemView;
use crate::query::values::value_view;
use crate::query::views::parameter_paths;
use fune_model::design::{CatalogItem, Design};
use fune_model::path::split_catalog_ref;
use fune_model::resolve::resolve_catalog_item;
use fune_store::layout::{CatalogLocation, catalog_location};

/// カタログ品の一覧（ID 順）。
pub(crate) fn list(design: &Design) -> Vec<CatalogItemView> {
    design
        .catalog
        .values()
        .map(|item| item_view(design, item))
        .collect()
}

/// カタログ品 1 つ。書き換えられるか（`editable`）は fune-ops と同じ `catalog_location` で決める。
fn item_view(design: &Design, item: &CatalogItem) -> CatalogItemView {
    let (scope, name) = split_catalog_ref(&item.id).unwrap_or(("", item.id.as_str()));
    let states = design.kind_states(item.kind.as_str()).to_vec();
    CatalogItemView {
        id: item.id.clone(),
        scope: scope.to_owned(),
        name: name.to_owned(),
        editable: matches!(
            catalog_location(design, &item.id),
            CatalogLocation::Local { .. }
        ),
        kind: item.kind.to_string(),
        label: item.label.clone(),
        label_en: item.label_en.clone(),
        manufacturer: item.manufacturer.clone(),
        part_number: item.part_number.clone(),
        maturity: item.maturity,
        datasheet: item.datasheet.clone(),
        source: item.source.clone(),
        ports: item.ports.clone(),
        paths: parameter_paths(design, &item.id, &states),
        states,
        values: resolve_catalog_item(design, item)
            .iter()
            .map(|(attr, v)| (attr.clone(), value_view(attr, v)))
            .collect(),
        users: design.catalog_part_users(&item.id),
    }
}
