//! カタログ品の Op のコマンド（`fune catalog add|set|unset|rm|rename`）。

use super::commands::value_cmds;
use super::text::Cmd;
use crate::op::Op;
use fune_model::design::CatalogField;

/// カタログ品の Op のコマンド。該当しなければ None。
pub(super) fn catalog_op(op: &Op) -> Option<Vec<String>> {
    Some(match op {
        Op::CreateCatalogItem {
            id,
            kind,
            label,
            label_en,
            maturity,
            copy_from_part,
        } => vec![
            Cmd::new(&["fune", "catalog", "add", id])
                .flag("kind", Some(kind))
                .flag("label", label.as_deref())
                .flag("label-en", label_en.as_deref())
                .flag("maturity", maturity.map(|m| m.as_str()))
                .flag("from-part", copy_from_part.as_deref())
                .done(),
        ],
        Op::SetCatalogValue {
            item,
            attr,
            state,
            value,
            from,
        } => value_cmds(
            &["fune", "catalog", "set", item],
            attr,
            state.as_deref(),
            value,
            from.as_ref(),
        ),
        Op::UnsetCatalogValue { item, attr, state } => {
            let path = match state {
                Some(s) => format!("{attr}.{s}"),
                None => attr.clone(),
            };
            vec![Cmd::new(&["fune", "catalog", "unset", item, &path]).done()]
        }
        Op::SetCatalogField { item, field } => vec![field_cmd(item, field)],
        Op::RemoveCatalogItem { id } => vec![Cmd::new(&["fune", "catalog", "rm", id]).done()],
        Op::RenameCatalogItem { from, to } => {
            vec![Cmd::new(&["fune", "catalog", "rename", from, to]).done()]
        }
        _ => return None,
    })
}

fn field_cmd(item: &str, field: &CatalogField) -> String {
    let text: Option<String> = match field {
        CatalogField::Kind(k) => Some(k.clone()),
        CatalogField::Maturity(m) => m.map(|m| m.as_str().to_string()),
        CatalogField::Label(v)
        | CatalogField::LabelEn(v)
        | CatalogField::Manufacturer(v)
        | CatalogField::PartNumber(v)
        | CatalogField::Datasheet(v)
        | CatalogField::Source(v) => v.clone(),
    };
    match text {
        Some(t) => Cmd::new(&["fune", "catalog", "set", item, field.name()])
            .arg(&t)
            .done(),
        None => Cmd::new(&["fune", "catalog", "unset", item, field.name()]).done(),
    }
}
