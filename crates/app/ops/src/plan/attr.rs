//! 属性の値の書き込み（部品とカタログ品で共有）。状態ごとのセルを書くときの広げ方もここだけ。

use crate::error::OpError;
use fune_model::Design;
use fune_model::value::{Authored, Provenance, Scalar, Value};
use fune_store::Edits;
use fune_store::write::part as wpart;
use std::collections::BTreeMap;

/// 書き込み先（ファイル・今の値・状態の語彙・表示名）。
pub(super) struct AttrTarget<'a> {
    pub file: &'a str,
    /// 今書かれている値（部品なら上書き、カタログ品ならその値）
    pub current: Option<&'a Value>,
    /// 種別の状態の語彙
    pub states: &'a [String],
    /// 診断の本文に出す名前（`RW_X`・`local/rw_10mNms`）
    pub owner: &'a str,
}

/// 属性の値を書く（`state` があれば状態ごとの値の 1 セル）。
pub(super) fn write_attr(
    edits: &mut Edits,
    design: &Design,
    target: AttrTarget,
    attr: &str,
    state: Option<&str>,
    value: &Value,
    from: Option<Provenance>,
) -> Result<(), OpError> {
    if design.attribute(attr).is_none() {
        return Err(OpError::not_found(format!(
            "属性 `{attr}` の定義（先に attr add で足してください）"
        )));
    }
    let Some(state) = state else {
        let authored = Authored {
            value: value.clone(),
            from,
        };
        return Ok(wpart::set_attr(edits, target.file, attr, &authored)?);
    };
    let Value::Scalar(scalar) = value else {
        return Err(OpError::invalid("状態ごとのセルにはスカラーを書きます"));
    };
    let cell = Authored {
        value: scalar.clone(),
        from,
    };
    match target.current {
        None | Some(Value::ByState(_)) => Ok(wpart::set_attr_state(
            edits,
            target.file,
            attr,
            state,
            &cell,
        )?),
        Some(Value::Scalar(constant)) => {
            // どの状態でも同じ値 → 状態ごとの表に広げてから 1 セルを書き換える。
            let mut cells: BTreeMap<String, Authored<Scalar>> = target
                .states
                .iter()
                .map(|s| (s.clone(), Authored::bare(constant.clone())))
                .collect();
            cells.insert(state.to_string(), cell);
            let value = Authored::bare(Value::ByState(cells));
            Ok(wpart::set_attr(edits, target.file, attr, &value)?)
        }
        Some(_) => Err(OpError::invalid(format!(
            "`{}.{attr}` は状態ごとの値にできない型です",
            target.owner
        ))),
    }
}

/// 属性の値を消す（`state` があればそのセルだけ）。書かれていなければ NotFound。
pub(super) fn unset_attr(
    edits: &mut Edits,
    file: &str,
    owner: &str,
    attr: &str,
    state: Option<&str>,
) -> Result<(), OpError> {
    if wpart::unset_attr(edits, file, attr, state)? {
        Ok(())
    } else {
        Err(OpError::not_found(format!("`{owner}.{attr}` の値")))
    }
}
