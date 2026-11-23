//! `fune project set|unset` に打った引数 → `SetProjectField`（`POST /ops/from_command` の `project_set`）。
//! **`[project]` のフィールド名の読み方はここだけ**（名前の一覧は `ProjectField::NAMES`）。

use super::text::{given, invalid};
use crate::dto::ProjectSetText;
use crate::error::Result;
use fune_model::design::ProjectField;
use fune_ops::Op;

/// 打った引数 → `SetProjectField`（前後の空白を除き、空の値は書かない）。`name` は消せない。
pub fn project_field_op(t: &ProjectSetText) -> Result<Op> {
    let value = given(&t.value).map(str::to_owned);
    let field = match t.field.trim() {
        "name" => ProjectField::Name(
            value.ok_or_else(|| invalid("The project name cannot be removed (change it with `fune project set name <value>`)"))?,
        ),
        "label" => ProjectField::Label(value),
        other => {
            return Err(invalid(format!(
                "Unknown project field `{other}` (one of: {})",
                ProjectField::NAMES.join(", ")
            )));
        }
    };
    Ok(Op::SetProjectField { field })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn text(field: &str, value: Option<&str>) -> ProjectSetText {
        ProjectSetText {
            field: field.into(),
            value: value.map(str::to_owned),
        }
    }

    #[test]
    fn typed_text_becomes_a_project_field() {
        let op = |f| Op::SetProjectField { field: f };
        assert_eq!(
            project_field_op(&text(" name", Some(" x "))).unwrap(),
            op(ProjectField::Name("x".into()))
        );
        assert_eq!(
            project_field_op(&text("label", Some("  "))).unwrap(),
            op(ProjectField::Label(None))
        );
        assert_eq!(
            project_field_op(&text("label", None)).unwrap(),
            op(ProjectField::Label(None))
        );
        assert!(project_field_op(&text("name", None)).is_err());
        assert!(project_field_op(&text("schema", Some("3"))).is_err());
        // 名前の一覧のどれも読める
        for name in ProjectField::NAMES {
            assert!(project_field_op(&text(name, Some("v"))).is_ok());
        }
    }
}
