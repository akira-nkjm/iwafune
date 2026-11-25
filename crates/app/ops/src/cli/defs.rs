//! 定義の Op のコマンド（`fune attr add|replace|rm`・`fune kind add|replace|rm`。`rm` は ID だけなので commands.rs）。構造体のフィールドと旗は 1 対 1。
//!
//! | フィールド | 旗 |
//! |---|---|
//! | AttributeDef `label` / `label_en` / `description` | `--label` / `--label-en` / `--description` |
//! | `type` / `by_state` | `--type <scalar…>` / `--by-state` |
//! | `dimension` / `units` / `display_unit` | `--dimension` / `--units a,b` / `--display-unit` |
//! | `min` / `max` / `default`（SI の数値） | `--min` / `--max` / `--default` |
//! | `symbols` / `budget` / `unit_vector` / `fov_axis` / `references` / `reference_roles` / `section` | `--symbols a,b` / `--budget mass` / `--unit-vector` / `--fov-axis <属性>` / `--references part` / `--reference-roles a,b` / `--section` |
//! | KindDef `label` / `label_en` / `description` / `extends` / `role` | `--label` / `--label-en` / `--description` / `--extends` / `--role` |
//! | `states` / `required` / `optional` | `--states off,on` / `--required a,b` / `--optional c,d` |
//! | `ports`（ポート名 → ネットの種類） | `--port PWR=power --port DATA=data`（繰り返し。ポート名の順） |

use super::text::Cmd;
use fune_model::design::{AttributeDef, KindDef};
use fune_model::value::{list_to_text, number_to_text};

/// 空でなければ一覧の文字列（`a,b`）。
fn list<S: AsRef<str>>(items: &[S]) -> Option<String> {
    (!items.is_empty()).then(|| list_to_text(items))
}

/// `fune attr add|replace <ID> …`（`replace` は丸ごとの置き換え。旗は `add` と同じ）
pub(super) fn attr_cmd(verb: &str, def: &AttributeDef) -> String {
    let num = number_to_text;
    let (min, max, default) = (def.min.map(num), def.max.map(num), def.default.map(num));
    Cmd::new(&["fune", "attr", verb, def.id.as_str()])
        .flag("label", Some(&def.label))
        .flag("label-en", def.label_en.as_deref())
        .flag("description", def.description.as_deref())
        .flag("type", Some(def.value_type.as_str()))
        .switch("by-state", def.by_state)
        .flag("dimension", def.dimension.as_deref())
        .flag("units", list(&def.units).as_deref())
        .flag("display-unit", def.display_unit.as_deref())
        .flag("min", min.as_deref())
        .flag("max", max.as_deref())
        .flag("default", default.as_deref())
        .flag("symbols", list(&def.symbols).as_deref())
        .flag("budget", def.budget.map(|b| b.as_str()))
        .switch("unit-vector", def.unit_vector)
        .flag("fov-axis", def.fov_axis.as_ref().map(|a| a.as_str()))
        .flag("references", def.references.map(|r| r.as_str()))
        .flag(
            "reference-roles",
            list(
                &def.reference_roles
                    .iter()
                    .map(|r| r.as_str())
                    .collect::<Vec<_>>(),
            )
            .as_deref(),
        )
        .flag("section", def.section.as_deref())
        .done()
}

/// `fune kind add|replace <ID> …`（`replace` は丸ごとの置き換え。旗は `add` と同じ）
pub(super) fn kind_cmd(verb: &str, def: &KindDef) -> String {
    let ids = |v: &[fune_model::Id<AttributeDef>]| {
        list(&v.iter().map(|a| a.as_str()).collect::<Vec<_>>())
    };
    let ports: Vec<String> = def
        .ports
        .iter()
        .map(|(name, kind)| format!("{name}={}", kind.as_str()))
        .collect();
    Cmd::new(&["fune", "kind", verb, def.id.as_str()])
        .flag("label", Some(&def.label))
        .flag("label-en", def.label_en.as_deref())
        .flag("description", def.description.as_deref())
        .flag("extends", def.extends.as_ref().map(|e| e.as_str()))
        .flag("role", def.role.map(|r| r.as_str()))
        .flag("states", list(&def.states).as_deref())
        .flag("required", ids(&def.required).as_deref())
        .flag("optional", ids(&def.optional).as_deref())
        .flags("port", ports.iter().map(String::as_str))
        .done()
}

#[cfg(test)]
mod tests {
    use super::*;
    use fune_model::builtin;
    use fune_model::design::{BudgetKind, NetKind};

    #[test]
    fn every_attribute_field_has_a_flag() {
        let mut def = builtin::attributes()["power"].clone();
        def.description = Some("消費電力".into());
        def.min = Some(-40.0);
        def.max = Some(1e3);
        def.default = Some(0.5);
        def.symbols = vec!["a".into(), "b".into()];
        def.budget = Some(BudgetKind::Power);
        def.unit_vector = true;
        def.fov_axis = Some("boresight".into());
        def.references = Some(fune_model::design::AttrReference::Part);
        def.reference_roles = vec![
            fune_model::role::Role::PropellantTank,
            fune_model::role::Role::Thruster,
        ];
        assert_eq!(
            attr_cmd("add", &def),
            "fune attr add power --label 消費電力 --label-en \"Power consumption\" --description 消費電力 \
             --type scalar --by-state --dimension Power --units W,mW --display-unit W --min=-40 --max 1000 \
             --default 0.5 --symbols a,b --budget power --unit-vector --fov-axis boresight --references part --reference-roles propellant_tank,thruster --section power"
                .replace("消費電力", "\"消費電力\"")
        );
    }

    #[test]
    fn every_kind_field_has_a_flag() {
        let mut def = builtin::kinds()["reaction_wheel"].clone();
        def.description = Some("RW".into());
        def.optional = vec!["size".into()];
        def.ports.insert("PWR".into(), NetKind::Power);
        def.ports.insert("DATA".into(), NetKind::Data);
        assert_eq!(
            kind_cmd("add", &def),
            "fune kind add reaction_wheel --label \"リアクションホイール\" --label-en \"Reaction wheel\" \
             --description RW --extends component --role reaction_wheel --states off,idle,on,peak \
             --required power,max_momentum,max_torque,spin_axis --optional size --port DATA=data --port PWR=power"
        );
    }
}
