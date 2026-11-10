//! `fune attr add`・`fune kind add` の引数 → `CreateAttribute`・`CreateKind`。旗は定義のフィールドと 1 対 1。
//! 引数は GUI の定義の作成と同じ打った引数の型（`AttrAddText`・`KindAddText`）にして、service の
//! `edit::attr_add_op`・`kind_add_op` が読む（表示名の既定・数・一覧・ポートの雛形の読み方も service）。

use super::common::{Built, list};
use crate::args::{AttrAddArgs, AttrCmd, KindAddArgs, KindCmd};
use fune_service::dto::{AttrAddText, KindAddText};
use fune_service::edit;

/// `fune attr add`
pub fn attr(c: &AttrCmd) -> Built<'_> {
    let AttrCmd::Add(a) = c else { return Ok(None) };
    Ok(Some((edit::attr_add_op(&attr_text(a))?, &a.edit)))
}

fn attr_text(a: &AttrAddArgs) -> AttrAddText {
    AttrAddText {
        id: a.id.clone(),
        label: a.label.clone(),
        label_en: a.label_en.clone(),
        description: a.description.clone(),
        value_type: Some(a.value_type.clone()),
        by_state: a.by_state,
        dimension: a.dimension.clone(),
        units: list(&a.units),
        display_unit: a.display_unit.clone(),
        min: a.min.clone(),
        max: a.max.clone(),
        default: a.default.clone(),
        symbols: list(&a.symbols),
        budget: a.budget.clone(),
        unit_vector: a.unit_vector,
        section: a.section.clone(),
    }
}

/// `fune kind add`
pub fn kind(c: &KindCmd) -> Built<'_> {
    let KindCmd::Add(a) = c else { return Ok(None) };
    Ok(Some((edit::kind_add_op(&kind_text(a))?, &a.edit)))
}

fn kind_text(a: &KindAddArgs) -> KindAddText {
    KindAddText {
        id: a.id.clone(),
        label: a.label.clone(),
        label_en: a.label_en.clone(),
        description: a.description.clone(),
        extends: a.extends.clone(),
        role: a.role.clone(),
        states: a.states.clone(),
        required: a.required.clone(),
        optional: a.optional.clone(),
        ports: a.ports.clone(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::args::Cli;
    use clap::Parser;
    use fune_service::dto::Op;

    fn attr_op(args: &[&str]) -> Op {
        let cli = Cli::try_parse_from(args).unwrap();
        let crate::args::Command::Attr(c) = &cli.command else {
            panic!()
        };
        attr(c).unwrap().unwrap().0
    }

    /// 値域・既定値は文字列のまま service に渡し、service が SI の数として読む（負の数も旗の値になる）。
    #[test]
    fn attribute_numbers_are_read_by_the_service() {
        let op = attr_op(&[
            "fune",
            "attr",
            "add",
            "depth",
            "--dimension",
            "Length",
            "--min=-40",
            "--max",
            "1e3",
        ]);
        let Op::CreateAttribute { def } = op else {
            panic!()
        };
        assert_eq!(
            (def.min, def.max, def.label.as_str()),
            (Some(-40.0), Some(1000.0), "depth")
        );
    }
}
