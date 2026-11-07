//! `fune attr add`・`fune kind add` の引数 → `CreateAttribute`・`CreateKind`。旗は定義のフィールドと 1 対 1。

use super::common::{Built, list};
use crate::args::{AttrAddArgs, AttrCmd, KindAddArgs, KindCmd};
use fune_service::dto::{AttributeDef, KindDef, Op};
use serde_json::json;
use std::collections::BTreeMap;

/// `fune attr add`
pub fn attr(c: &AttrCmd) -> Built<'_> {
    let AttrCmd::Add(a) = c else { return Ok(None) };
    Ok(Some((
        Op::CreateAttribute {
            def: attribute_def(a)?,
        },
        &a.edit,
    )))
}

fn attribute_def(a: &AttrAddArgs) -> anyhow::Result<AttributeDef> {
    let value = json!({
        "label": a.label.clone().unwrap_or_else(|| a.id.clone()),
        "label_en": a.label_en,
        "description": a.description,
        "type": a.value_type,
        "by_state": a.by_state,
        "dimension": a.dimension,
        "units": list(&a.units),
        "display_unit": a.display_unit,
        "min": a.min,
        "max": a.max,
        "symbols": list(&a.symbols),
        "budget": a.budget,
        "default": a.default,
        "unit_vector": a.unit_vector,
        "section": a.section,
    });
    let mut def: AttributeDef =
        serde_json::from_value(value).map_err(|e| anyhow::anyhow!("属性定義を作れません: {e}"))?;
    def.id = a.id.as_str().into();
    Ok(def)
}

/// `fune kind add`
pub fn kind(c: &KindCmd) -> Built<'_> {
    let KindCmd::Add(a) = c else { return Ok(None) };
    Ok(Some((Op::CreateKind { def: kind_def(a)? }, &a.edit)))
}

fn kind_def(a: &KindAddArgs) -> anyhow::Result<KindDef> {
    let mut ports = BTreeMap::new();
    for p in &a.ports {
        let (name, kind) = p
            .split_once('=')
            .ok_or_else(|| anyhow::anyhow!("--port は 名前=power|data|rf の形です: {p}"))?;
        ports.insert(name.trim().to_owned(), kind.trim().to_owned());
    }
    let value = json!({
        "label": a.label.clone().unwrap_or_else(|| a.id.clone()),
        "label_en": a.label_en,
        "description": a.description,
        "extends": a.extends,
        "role": a.role,
        "states": list(&a.states),
        "required": list(&a.required),
        "optional": list(&a.optional),
        "ports": ports,
    });
    let mut def: KindDef =
        serde_json::from_value(value).map_err(|e| anyhow::anyhow!("種別定義を作れません: {e}"))?;
    def.id = a.id.as_str().into();
    Ok(def)
}
