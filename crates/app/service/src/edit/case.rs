//! `fune case set` に打った引数 → `SetCase`（`POST /ops/from_command` の `case_set`）。
//! **解析ケースの旗の読み方はここだけ**（量は `text::quantity_from_text`、時刻は `case_epoch_op` と同じ `Epoch::parse`、
//! 周回数は正の整数、`side` は `UncertaintySide` の綴り）。モード・フェーズ・熱の環境条件の実在は適用のときの検査が見る。

use super::text::{given, invalid, quantity_from_text};
use crate::dto::CaseSetText;
use crate::error::Result;
use fune_model::design::Case;
use fune_ops::Op;

/// 打った引数 → `SetCase`（前後の空白を除き、空の旗は書かない。ケースを丸ごと置き換える Op なので、書かなかった欄は無くなる）。
pub fn case_set_op(t: &CaseSetText) -> Result<Op> {
    let orbits = given(&t.orbits)
        .map(|n| {
            n.parse::<u32>()
                .map_err(|_| invalid(format!("orbits must be a whole number ({n})")))
        })
        .transpose()?;
    if let Some(epoch) = given(&t.epoch) {
        sat_astro::time::Epoch::parse(epoch)
            .map_err(|e| invalid(format!("Cannot read the case epoch `{epoch}` ({e})")))?;
    }
    let text = |v: &Option<String>| given(v).map(str::to_owned);
    let value = serde_json::json!({
        "label": text(&t.label),
        "description": text(&t.description),
        "mode": text(&t.mode),
        "phase": text(&t.phase),
        "side": text(&t.side),
        "thermal_case": text(&t.thermal_case),
        "orbits": orbits,
        "epoch": text(&t.epoch),
    });
    let mut case: Case = serde_json::from_value(value)
        .map_err(|e| invalid(format!("Cannot create the analysis case: {e}")))?;
    case.id = t.id.trim().into();
    case.beta = given(&t.beta).map(quantity_from_text).transpose()?;
    case.sun_distance = given(&t.sun_distance).map(quantity_from_text).transpose()?;
    Ok(Op::SetCase { case })
}

#[cfg(test)]
mod tests {
    use super::*;
    use fune_model::value::Quantity;

    /// 前後の空白を除き、空の旗は書かず、量・周回数・時刻を読む（CLI と GUI の欄が同じここを通る）。
    #[test]
    fn typed_text_becomes_a_case() {
        let t = CaseSetText {
            id: " hot ".into(),
            label: Some(" Hot case ".into()),
            mode: Some("SAFE".into()),
            side: Some("max".into()),
            beta: Some("60 deg".into()),
            orbits: Some(" 12 ".into()),
            sun_distance: Some("  ".into()),
            epoch: Some("2031-05-02T00:00:00Z".into()),
            ..Default::default()
        };
        let Op::SetCase { case } = case_set_op(&t).unwrap() else {
            panic!("not SetCase");
        };
        assert_eq!(case.id.as_str(), "hot");
        assert_eq!(case.label.as_deref(), Some("Hot case"));
        assert_eq!(case.mode.as_ref().map(|m| m.as_str()), Some("SAFE"));
        assert_eq!(case.beta, Some(Quantity::new(60.0, "deg")));
        assert_eq!(case.orbits, Some(12));
        assert_eq!(case.sun_distance, None);
        assert_eq!(case.description, None);
        for bad in [
            CaseSetText {
                orbits: Some("1.5".into()),
                ..t.clone()
            },
            CaseSetText {
                side: Some("high".into()),
                ..t.clone()
            },
            CaseSetText {
                beta: Some("sixty".into()),
                ..t.clone()
            },
            CaseSetText {
                epoch: Some("tuesday".into()),
                ..t
            },
        ] {
            assert!(case_set_op(&bad).is_err());
        }
    }
}
