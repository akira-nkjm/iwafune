//! カタログ品の編集の文字列 → Op（`fune catalog set local/rw_10mNms mass "620 g"` → `SetCatalogValue` など）。
//! 入口は `edit::{set_op, unset_op}`（`SetText.item`・`UnsetText.item` があるときここに振り分ける）。
//! 属性の値の読み方は部品と同じ（`text::attr_value`）。フィールド名（`fune_model::path::CATALOG_FIELDS`）は `SetCatalogField` にする。

use super::text::{SetText, attr_value, invalid};
use crate::error::Result;
use crate::workspace::Workspace;
use fune_model::design::{CatalogField, Maturity};
use fune_model::path::{AttrPath, CATALOG_FIELDS, parse_catalog_attr};
use fune_model::value::Provenance;
use fune_ops::Op;

/// 品の中のパス（`mass`・`power.on`・`label`）→ 属性と状態。品のパラメーターパス（`local/rw_10mNms.power.on`。
/// `CatalogItemView.paths` の値で、GUI はこれを送る）も受け、品が `item` と違えば拒む。読み方は `fune_model::path`。
fn item_path(item: &str, path: &str) -> Result<AttrPath> {
    match parse_catalog_attr(path) {
        Ok((owner, p)) if owner == item => Ok(p),
        Ok((owner, _)) => Err(invalid(format!(
            "`{path}` はカタログ品 `{owner}` のパスです（編集する品は `{item}`）"
        ))),
        Err(_) => AttrPath::parse(path).map_err(invalid),
    }
}

/// フィールド名（`fune_model::path::CATALOG_FIELDS`）なら、その名前。状態の付いたパスはフィールドではない。
fn field_name(p: &AttrPath) -> Option<&str> {
    (p.state.is_none() && CATALOG_FIELDS.contains(&p.attr.as_str())).then_some(p.attr.as_str())
}

/// `fune catalog set <品> <属性[.状態] | フィールド> <値>`。`t.path` は品の中のパス。
pub(super) fn catalog_set_op(ws: &Workspace, item: &str, t: &SetText) -> Result<Op> {
    let path = item_path(item, &t.path)?;
    if let Some(name) = field_name(&path) {
        return Ok(Op::SetCatalogField {
            item: item.to_owned(),
            field: catalog_field(name, Some(&t.value))?,
        });
    }
    let AttrPath { attr, state } = path;
    let value = attr_value(ws, &attr, state.is_some(), t)?;
    let from = (t.source.is_some() || t.note.is_some()).then(|| Provenance {
        source: t.source.clone(),
        note: t.note.clone(),
    });
    Ok(Op::SetCatalogValue {
        item: item.to_owned(),
        attr,
        state,
        value,
        from,
    })
}

/// `fune catalog unset <品> <属性[.状態] | フィールド>`。
pub(super) fn catalog_unset_op(item: &str, path: &str) -> Result<Op> {
    let path = item_path(item, path)?;
    if let Some(name) = field_name(&path) {
        return Ok(Op::SetCatalogField {
            item: item.to_owned(),
            field: catalog_field(name, None)?,
        });
    }
    let AttrPath { attr, state } = path;
    Ok(Op::UnsetCatalogValue {
        item: item.to_owned(),
        attr,
        state,
    })
}

fn catalog_field(name: &str, text: Option<&str>) -> Result<CatalogField> {
    let owned = text.map(str::to_owned);
    Ok(match name {
        "kind" => {
            CatalogField::Kind(owned.ok_or_else(|| invalid("カタログ品の kind は消せません"))?)
        }
        "label" => CatalogField::Label(owned),
        "label_en" => CatalogField::LabelEn(owned),
        "manufacturer" => CatalogField::Manufacturer(owned),
        "part_number" => CatalogField::PartNumber(owned),
        "datasheet" => CatalogField::Datasheet(owned),
        "source" => CatalogField::Source(owned),
        "maturity" => CatalogField::Maturity(match text {
            None => None,
            Some(t) => Some(maturity(t)?),
        }),
        other => {
            return Err(invalid(format!(
                "カタログ品のフィールドではありません: {other}"
            )));
        }
    })
}

/// `off_the_shelf` / `modified` / `new`。
pub fn maturity(text: &str) -> Result<Maturity> {
    text.parse().map_err(|_| {
        invalid(format!(
            "成熟度は off_the_shelf / modified / new です: {text}"
        ))
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn item_paths_and_parameter_paths_give_the_same_op() {
        let item = "local/rw_10mNms";
        for (short, full) in [
            ("power.peak", "local/rw_10mNms.power.peak"),
            ("hk_rate", "local/rw_10mNms.hk_rate"),
            ("label_en", "local/rw_10mNms.label_en"),
        ] {
            assert_eq!(
                catalog_unset_op(item, short).unwrap(),
                catalog_unset_op(item, full).unwrap()
            );
        }
        assert!(matches!(
            catalog_unset_op(item, "label_en").unwrap(),
            Op::SetCatalogField {
                field: CatalogField::LabelEn(None),
                ..
            }
        ));
        // 状態の付いたパスはフィールドではない（属性 `label` の状態として読む）
        assert!(matches!(
            catalog_unset_op(item, "label.on").unwrap(),
            Op::UnsetCatalogValue { .. }
        ));
        // 別の品のパスは拒む
        assert!(catalog_unset_op(item, "local/obc_m7.mass").is_err());
    }
}
