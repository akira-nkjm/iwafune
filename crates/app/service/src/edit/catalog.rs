//! カタログ品の編集の文字列 → Op（`fune catalog set local/rw_10mNms mass "620 g"` → `SetCatalogValue` など）。
//! 入口は `edit::{set_op, unset_op}`（`SetText.item`・`UnsetText.item` があるときここに振り分ける）。
//! 属性の値の読み方は部品と同じ（`text::attr_value`）。フィールド名（`fune_model::design::CatalogFieldName`）は `SetCatalogField` にする。

use super::text::{AttrSlot, SetText, attr_value, invalid, list_from_text, maturity};
use crate::error::Result;
use crate::workspace::Workspace;
use fune_model::design::{CatalogField, CatalogFieldName};
use fune_model::path::{AttrPath, parse_catalog_attr};
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

/// フィールド名（`CatalogFieldName`）なら、そのフィールド。状態の付いたパスはフィールドではない。
fn field_name(p: &AttrPath) -> Option<CatalogFieldName> {
    match p.state {
        Some(_) => None,
        None => p.attr.parse().ok(),
    }
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
    let value = attr_value(ws, &attr, AttrSlot::of(state.as_deref()), t)?;
    let from = t.provenance();
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

/// フィールドと打った文字列 → 型付きのフィールド（`text` が None なら消す。`kind` は消せない）。
fn catalog_field(name: CatalogFieldName, text: Option<&str>) -> Result<CatalogField> {
    let owned = text.map(str::to_owned);
    Ok(match name {
        CatalogFieldName::Kind => {
            CatalogField::Kind(owned.ok_or_else(|| invalid("カタログ品の kind は消せません"))?)
        }
        CatalogFieldName::Label => CatalogField::Label(owned),
        CatalogFieldName::LabelEn => CatalogField::LabelEn(owned),
        CatalogFieldName::Manufacturer => CatalogField::Manufacturer(owned),
        CatalogFieldName::PartNumber => CatalogField::PartNumber(owned),
        CatalogFieldName::Tags => CatalogField::Tags(text.map(list_from_text).unwrap_or_default()),
        CatalogFieldName::Datasheet => CatalogField::Datasheet(owned),
        CatalogFieldName::Source => CatalogField::Source(owned),
        CatalogFieldName::Maturity => CatalogField::Maturity(text.map(maturity).transpose()?),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    /// カタログ品のフィールドの名前（`CatalogFieldName::NAMES`）は、どれも読めて同じ名前の型付きのフィールドになる。
    #[test]
    fn every_catalog_field_name_reads_back_to_the_same_name() {
        for &name in CatalogFieldName::ALL {
            let text = if name == CatalogFieldName::Maturity {
                "modified"
            } else {
                "text"
            };
            assert_eq!(catalog_field(name, Some(text)).unwrap().key(), name);
            if name != CatalogFieldName::Kind {
                assert_eq!(catalog_field(name, None).unwrap().key(), name);
            }
        }
        assert!(catalog_field(CatalogFieldName::Kind, None).is_err());
        assert!(catalog_field(CatalogFieldName::Maturity, Some("brand_new")).is_err());
    }

    /// タグはカンマ区切り（空白を除き、空の語は捨てる）。書いた順を保ち、消すと空になる。
    #[test]
    fn tags_are_read_as_a_comma_list_in_written_order() {
        let tags = |text| catalog_field(CatalogFieldName::Tags, text).unwrap();
        assert_eq!(
            tags(Some("flight_proven, 国産 ,,b")),
            CatalogField::Tags(vec!["flight_proven".into(), "国産".into(), "b".into()])
        );
        assert_eq!(tags(None), CatalogField::Tags(vec![]));
    }

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
