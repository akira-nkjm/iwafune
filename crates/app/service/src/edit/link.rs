//! `fune link add` に打った引数 → `CreateCommLink`（`POST /ops/from_command` の `link_add`）。
//! **回線の旗の読み方はここだけ**（向きは `LinkDirection` の綴り。部品・帯域の実在は適用のときの検査が見る）。

use super::text::invalid;
use crate::dto::LinkAddText;
use crate::error::Result;
use fune_model::design::LinkDirection;
use fune_ops::Op;

/// 打った引数 → `CreateCommLink`（前後の空白を除く）。
pub fn link_add_op(t: &LinkAddText) -> Result<Op> {
    let direction: LinkDirection = t.direction.trim().parse().map_err(|_| {
        invalid(format!(
            "cannot read as link direction (down / up): {}",
            t.direction.trim()
        ))
    })?;
    Ok(Op::CreateCommLink {
        id: t.id.trim().to_owned(),
        direction,
        part: t.part.trim().to_owned(),
        band: t.band.trim().to_owned(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn typed_text_becomes_a_link() {
        let t = LinkAddText {
            id: " x_down ".into(),
            direction: " down".into(),
            part: "STX ".into(),
            band: "s_band".into(),
        };
        assert_eq!(
            link_add_op(&t).unwrap(),
            Op::CreateCommLink {
                id: "x_down".into(),
                direction: LinkDirection::Down,
                part: "STX".into(),
                band: "s_band".into(),
            }
        );
        let bad = LinkAddText {
            direction: "sideways".into(),
            ..t
        };
        assert!(link_add_op(&bad).is_err());
    }
}
