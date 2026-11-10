//! 定義の作成の打った引数 → Op（`fune attr add` → `CreateAttribute`、`fune kind add` → `CreateKind`。`POST /ops/from_command`）。
//! 表示名を省くと ID、値の型を省くと scalar（**定義を作るときの既定はここだけ**。GUI の定義の作成も同じ関数を通る）。
//! 列挙の綴り（値の型・バジェット・ネットの種類）は定義の型の serde に任せる。

use super::text::{given, invalid, list_from_text, net_kind, pair_from_text};
use crate::dto::{AttrAddText, KindAddText};
use crate::error::Result;
use fune_model::design::{AttributeDef, KindDef, NetKind};
use fune_ops::Op;
use serde_json::json;
use std::collections::BTreeMap;

/// `fune attr add <ID> …` → `CreateAttribute`（次元と許可単位の検査は適用のときの検証）。
pub fn attr_add_op(t: &AttrAddText) -> Result<Op> {
    let id = t.id.trim();
    let number = |flag: &str, text: &Option<String>| -> Result<Option<f64>> {
        given(text)
            .map(|s| {
                s.parse::<f64>()
                    .map_err(|_| invalid(format!("{flag} は SI の数で書きます: {s}")))
            })
            .transpose()
    };
    let words = |items: &[String]| -> Vec<String> {
        items
            .iter()
            .map(|s| s.trim().to_owned())
            .filter(|s| !s.is_empty())
            .collect()
    };
    let value = json!({
        "label": given(&t.label).unwrap_or(id),
        "label_en": given(&t.label_en),
        "description": given(&t.description),
        "type": given(&t.value_type).unwrap_or("scalar"),
        "by_state": t.by_state,
        "dimension": given(&t.dimension),
        "units": words(&t.units),
        "display_unit": given(&t.display_unit),
        "min": number("--min", &t.min)?,
        "max": number("--max", &t.max)?,
        "default": number("--default", &t.default)?,
        "symbols": words(&t.symbols),
        "budget": given(&t.budget),
        "unit_vector": t.unit_vector,
        "section": given(&t.section),
    });
    let mut def: AttributeDef =
        serde_json::from_value(value).map_err(|e| invalid(format!("属性定義を作れません: {e}")))?;
    def.id = id.into();
    Ok(Op::CreateAttribute { def })
}

/// `fune kind add <ID> …` → `CreateKind`。状態・属性の一覧は `list_from_text`、ポートの雛形は [`port_template_from_text`]。
pub fn kind_add_op(t: &KindAddText) -> Result<Op> {
    let id = t.id.trim();
    let list = |text: &Option<String>| text.as_deref().map(list_from_text).unwrap_or_default();
    let ports = t
        .ports
        .iter()
        .filter(|p| !p.trim().is_empty())
        .map(|p| port_template_from_text(p))
        .collect::<Result<BTreeMap<_, _>>>()?;
    let value = json!({
        "label": given(&t.label).unwrap_or(id),
        "label_en": given(&t.label_en),
        "description": given(&t.description),
        "extends": given(&t.extends),
        "role": given(&t.role),
        "states": list(&t.states),
        "required": list(&t.required),
        "optional": list(&t.optional),
        "ports": ports,
    });
    let mut def: KindDef =
        serde_json::from_value(value).map_err(|e| invalid(format!("種別定義を作れません: {e}")))?;
    def.id = id.into();
    Ok(Op::CreateKind { def })
}

/// ポートの雛形（`"PWR=power"`）→ (ポート名, ネットの種類)。`fune kind add --port`。
pub fn port_template_from_text(text: &str) -> Result<(String, NetKind)> {
    let (name, kind) = pair_from_text(text)
        .ok_or_else(|| invalid(format!("--port は 名前=power|data|rf の形です: {text}")))?;
    Ok((name, net_kind(&kind)?))
}

#[cfg(test)]
mod tests {
    use super::*;

    /// GUI の属性の作成（ID・表示名・次元・許可単位・状態ごと）と CLI の `fune attr add` は同じ定義になる。表示名の既定は ID。
    #[test]
    fn attribute_defaults_label_to_id_and_type_to_scalar() {
        let t = AttrAddText {
            id: "tank_pressure".into(),
            label: Some(" ".into()),
            dimension: Some("Pressure".into()),
            units: vec!["Pa".into(), " kPa ".into()],
            by_state: true,
            min: Some("-1e3".into()),
            ..Default::default()
        };
        let Op::CreateAttribute { def } = attr_add_op(&t).unwrap() else {
            panic!()
        };
        assert_eq!(def.label, "tank_pressure");
        assert_eq!(def.units, ["Pa", "kPa"]);
        assert_eq!(def.min, Some(-1000.0));
        assert!(def.by_state);
        assert_eq!(
            serde_json::to_value(def.value_type).unwrap(),
            json!("scalar")
        );
        let bad = AttrAddText {
            max: Some("many".into()),
            ..t.clone()
        };
        assert!(attr_add_op(&bad).is_err());
        let bad_type = AttrAddText {
            value_type: Some("matrix".into()),
            ..t
        };
        assert!(attr_add_op(&bad_type).is_err());
    }

    /// 種別の状態・属性の一覧は `list_from_text`（カンマで分け、空白を除く）、ポートは `名前=種類`。
    #[test]
    fn kind_lists_and_port_templates_are_read_by_the_service() {
        let t = KindAddText {
            id: "tank".into(),
            role: Some(String::new()),
            states: Some(" off, on ,".into()),
            required: Some("mass,power".into()),
            ports: vec!["PWR = power".into(), "DATA=data".into()],
            ..Default::default()
        };
        let Op::CreateKind { def } = kind_add_op(&t).unwrap() else {
            panic!()
        };
        assert_eq!(def.label, "tank");
        assert_eq!(def.role, None);
        assert_eq!(def.states, ["off", "on"]);
        assert_eq!(def.required.len(), 2);
        assert_eq!(def.ports.get("PWR"), Some(&NetKind::Power));
        assert!(port_template_from_text("PWR").is_err());
        assert!(port_template_from_text("PWR=dc").is_err());
    }
}
