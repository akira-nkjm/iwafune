//! `fune ground add` に打った引数 → `CreateGroundStation`（`POST /ops/from_command` の `ground_add`）。
//! **地上局の旗の読み方はここだけ**（量は `text::quantity_from_text`。単位の次元は適用のときの検査が見る）。

use super::text::{given, quantity_from_text};
use crate::dto::GroundAddText;
use crate::error::Result;
use fune_model::value::Quantity;
use fune_ops::Op;

/// 打った引数 → `CreateGroundStation`（前後の空白を除き、空の旗は書かない）。
pub fn ground_add_op(t: &GroundAddText) -> Result<Op> {
    let quantity = |text: &Option<String>| -> Result<Option<Quantity>> {
        given(text).map(quantity_from_text).transpose()
    };
    Ok(Op::CreateGroundStation {
        id: t.id.trim().to_owned(),
        label: given(&t.label).map(str::to_owned),
        latitude: quantity(&t.latitude)?,
        longitude: quantity(&t.longitude)?,
        altitude: quantity(&t.altitude)?,
        min_elevation: quantity(&t.min_elevation)?,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 前後の空白を除き、空の旗は書かず、量は単位付きで読む（CLI と GUI の欄が同じここを通る）。
    #[test]
    fn typed_text_becomes_a_station() {
        let t = GroundAddText {
            id: " SVALBARD ".into(),
            label: Some(" Svalbard (example) ".into()),
            latitude: Some("78.2 deg".into()),
            longitude: Some(" 15.4 deg".into()),
            altitude: Some("  ".into()),
            min_elevation: Some("5 deg".into()),
        };
        let Op::CreateGroundStation {
            id,
            label,
            latitude,
            altitude,
            min_elevation,
            ..
        } = ground_add_op(&t).unwrap()
        else {
            panic!("not CreateGroundStation");
        };
        assert_eq!(id, "SVALBARD");
        assert_eq!(label.as_deref(), Some("Svalbard (example)"));
        assert_eq!(latitude, Some(Quantity::new(78.2, "deg")));
        assert_eq!(altitude, None);
        assert_eq!(min_elevation, Some(Quantity::new(5.0, "deg")));
        let bad = GroundAddText {
            latitude: Some("north".into()),
            ..t
        };
        assert!(ground_add_op(&bad).is_err());
    }
}
