//! カタログ品の表示（`GET /design` の `catalog`）。値は部品の値と同じ組み立て（`values::value_view`）で作る。

use crate::dto::{CatalogItemView, CatalogSourceView};
use crate::query::values::value_view;
use crate::query::views::parameter_paths;
use fune_model::design::{CatalogItem, Design};
use fune_model::path::split_catalog_ref;
use fune_model::resolve::resolve_catalog_item;
use fune_model::value::list_to_text;
use fune_store::layout::{CatalogLocation, catalog_location};

/// カタログの出所の一覧（fune.toml の `[[catalog]]` の順）。書き込めるかは品の `editable` と同じ判定。
/// 共有カタログには lock の commit と読み込みの状態（`Design::shared_catalog_statuses`）を付ける。
pub(crate) fn sources(design: &Design) -> Vec<CatalogSourceView> {
    let mut statuses = design.shared_catalog_statuses();
    design
        .project
        .catalogs
        .iter()
        .map(|c| CatalogSourceView {
            name: c.name.clone(),
            editable: c.local_dir().is_some(),
            shared: statuses
                .iter()
                .position(|s| s.name == c.name)
                .map(|i| statuses.remove(i)),
        })
        .collect()
}

/// カタログ品の一覧（ID 順）。
pub(crate) fn list(design: &Design) -> Vec<CatalogItemView> {
    design
        .catalog
        .values()
        .map(|item| item_view(design, item))
        .collect()
}

/// カタログ品 1 つ。書き換えられるか（`editable`）は fune-ops と同じ `catalog_location` で決める。
pub(crate) fn item_view(design: &Design, item: &CatalogItem) -> CatalogItemView {
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
        system: design
            .kind_system(item.kind.as_str())
            .map(fune_projection::system::to_analysis),
        label: item.label.clone(),
        label_en: item.label_en.clone(),
        manufacturer: item.manufacturer.clone(),
        part_number: item.part_number.clone(),
        maturity: item.maturity,
        tags: item.tags.clone(),
        tags_text: list_to_text(&item.tags),
        datasheet: item.datasheet.clone(),
        datasheet_link: super::sources::datasheet_link(design, item.datasheet.as_deref()),
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

#[cfg(test)]
mod tests {
    use super::*;
    use fune_model::design::CatalogSource;
    use sat_analysis::spec::System;

    /// 出所の一覧の「書き込めるか」は、品の `editable`（`catalog_location`）と同じ判定で、fune.toml の順に並ぶ。
    #[test]
    fn sources_mark_only_project_catalogs_as_editable() {
        let mut design = Design::default();
        design.project.catalogs = vec![
            CatalogSource {
                name: "shared".into(),
                path: None,
                git: Some("https://git.example.com/parts-catalog".into()),
                rev: Some("v1".into()),
            },
            CatalogSource {
                name: "local".into(),
                path: Some("catalog".into()),
                git: None,
                rev: None,
            },
        ];
        let listed: Vec<(String, bool)> = sources(&design)
            .into_iter()
            .map(|s| (s.name, s.editable))
            .collect();
        assert_eq!(
            listed,
            [("shared".to_owned(), false), ("local".to_owned(), true)]
        );
        for (name, editable) in listed {
            let local = matches!(
                catalog_location(&design, &format!("{name}/rw")),
                CatalogLocation::Local { .. }
            );
            assert_eq!(local, editable, "{name}");
        }
    }

    /// 品の系は種別の役割の系。継承した役割も辿り、役割の無い種別（`component`）は系を持たない。
    #[test]
    fn item_system_follows_the_kind_role_through_inheritance() {
        let mut design = Design::default();
        let custom: fune_model::design::KindDef = serde_json::from_value(serde_json::json!({
            "id": "big_wheel", "label": "Big wheel", "extends": "reaction_wheel"
        }))
        .expect("kind");
        design.kinds.insert(custom.id.clone(), custom);
        let system_of = |kind: &str| {
            let item: CatalogItem = serde_json::from_value(serde_json::json!({
                "id": format!("local/{kind}"), "kind": kind, "attr": {}, "ports": {}
            }))
            .expect("item");
            item_view(&design, &item).system
        };
        assert_eq!(system_of("reaction_wheel"), Some(System::Aocs));
        assert_eq!(system_of("battery"), Some(System::Power));
        assert_eq!(system_of("big_wheel"), Some(System::Aocs));
        assert_eq!(system_of("component"), None);
    }
}
