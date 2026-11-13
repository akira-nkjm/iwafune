//! 名詞のコマンドに打った引数 → Op（`fune net set|unset`・`fune part place`・`fune mode set|unset`、`POST /ops/from_command`）。入口は [`command_ops`]（CLI はコマンドごとの関数を直接呼ぶ）。定義は `super::defs`、マヌーバは `super::maneuver`、値の読み方は `super::text`。

use super::text::{
    given, invalid, list_from_text, net_kind, pair_from_text, quantity_from_text,
    quaternion_from_text, vector_from_text,
};
use crate::dto::{CommandText, ModeCellText, ModeSetText, NetSetText, NetUnsetText, PlaceText};
use crate::error::Result;
use fune_model::design::{NetField, NetFieldName};
use fune_model::path::PortRef;
use fune_ops::{ModeCell, Op};
use std::collections::BTreeMap;

/// 打った引数 → Op の列（`POST /ops/from_command`）。適用はしない（`POST /ops` で送る）。
pub fn command_ops(t: &CommandText) -> Result<Vec<Op>> {
    Ok(match t {
        CommandText::NetSet(t) => net_set_ops(t)?,
        CommandText::NetUnset(t) => vec![net_unset_op(t)?],
        CommandText::PartPlace(t) => vec![place_op(t)?],
        CommandText::ModeMove {
            id,
            before,
            after,
            first,
            last,
        } => {
            vec![mode_move_op(
                id,
                before.as_deref(),
                after.as_deref(),
                *first,
                *last,
            )?]
        }
        CommandText::ModeSet(t) => vec![mode_set_op(t)?],
        CommandText::ModeDuration { mode, duration } => {
            vec![mode_duration_op(mode, duration.as_deref())?]
        }
        CommandText::AttrAdd(t) => vec![super::defs::attr_add_op(t)?],
        CommandText::KindAdd(t) => vec![super::defs::kind_add_op(t)?],
        CommandText::ManeuverAdd(t) => vec![super::maneuver::maneuver_op(t, true)?],
        CommandText::ManeuverSet(t) => vec![super::maneuver::maneuver_op(t, false)?],
    })
}

/// `fune net set <ID> --rail "5 V" …` → 書いた旗ごとに `SetNetField`（`NetFieldName` の順）。
pub fn net_set_ops(t: &NetSetText) -> Result<Vec<Op>> {
    let ops = NetFieldName::ALL
        .iter()
        .filter_map(|&name| flag_text(t, name).map(|text| (name, text)))
        .map(|(name, text)| {
            Ok(Op::SetNetField {
                net: t.net.clone(),
                field: net_field(name, Some(text))?,
            })
        })
        .collect::<Result<Vec<_>>>()?;
    if ops.is_empty() {
        return Err(invalid("書くフィールドの旗がありません（--rail など）"));
    }
    Ok(ops)
}

/// `fune net set` の旗（GUI の欄）に書いた文字列。
fn flag_text(t: &NetSetText, name: NetFieldName) -> Option<&str> {
    match name {
        NetFieldName::Kind => &t.kind,
        NetFieldName::Label => &t.label,
        NetFieldName::Source => &t.source,
        NetFieldName::Rail => &t.rail,
        NetFieldName::Limit => &t.limit,
        NetFieldName::Protocol => &t.protocol,
        NetFieldName::Bitrate => &t.bitrate,
        NetFieldName::Master => &t.master,
    }
    .as_deref()
}

/// `fune net unset <ID> <フィールド>` → 空にした `SetNetField`。
pub fn net_unset_op(t: &NetUnsetText) -> Result<Op> {
    let name = t.field.trim().parse::<NetFieldName>().map_err(invalid)?;
    Ok(Op::SetNetField {
        net: t.net.clone(),
        field: net_field(name, None)?,
    })
}

/// フィールドと打った文字列 → 型付きのフィールド（`text` が None なら消す）。**ネットのフィールドの読み方はここだけ**。
fn net_field(name: NetFieldName, text: Option<&str>) -> Result<NetField> {
    let quantity = || text.map(quantity_from_text).transpose();
    let port = || {
        text.map(|t| t.trim().parse::<PortRef>().map_err(invalid))
            .transpose()
    };
    let owned = text.map(str::to_owned);
    Ok(match name {
        NetFieldName::Kind => NetField::Kind(net_kind(
            text.ok_or_else(|| invalid("ネットの kind は消せません"))?,
        )?),
        NetFieldName::Label => NetField::Label(owned),
        NetFieldName::Source => NetField::Source(port()?),
        NetFieldName::Rail => NetField::Rail(quantity()?),
        NetFieldName::Limit => NetField::Limit(quantity()?),
        NetFieldName::Protocol => NetField::Protocol(owned),
        NetFieldName::Bitrate => NetField::Bitrate(quantity()?),
        NetFieldName::Master => NetField::Master(port()?),
    })
}

/// `fune part place` → `Place`（取付と、取付点に対する位置・姿勢を丸ごと置き換える）。取付先が無ければ機体の根。
pub fn place_op(t: &PlaceText) -> Result<Op> {
    let on = given(&t.on);
    Ok(Op::Place {
        part: t.part.trim().to_owned(),
        on: on.map(str::to_owned),
        point: on.and(given(&t.point)).map(str::to_owned),
        position: given(&t.position).map(vector_from_text).transpose()?,
        orientation: given(&t.orientation)
            .map(quaternion_from_text)
            .transpose()?,
    })
}

/// `fune mode set|unset` のセルの列 → 1 つの `SetModeStates`。
pub fn mode_set_op(t: &ModeSetText) -> Result<Op> {
    if t.cells.is_empty() {
        return Err(invalid("モード表のセルがありません"));
    }
    let cells = t.cells.iter().map(mode_cell).collect::<Result<_>>()?;
    Ok(Op::SetModeStates { cells })
}

fn mode_cell(c: &ModeCellText) -> Result<ModeCell> {
    let state = c.state.as_deref().map(str::trim);
    if state == Some("") {
        return Err(invalid(format!(
            "{} の {} の基本の状態が空です（行を消すときは `fune mode unset`）",
            c.mode, c.part
        )));
    }
    let state = state.map(str::to_owned);
    let mut duty = BTreeMap::new();
    // 行を消すセル（状態が無い）は割合も持たない
    if state.is_some() {
        for (s, text) in &c.duty {
            if !text.trim().is_empty() {
                duty.insert(s.trim().to_owned(), fraction_from_text(text)?);
            }
        }
    }
    Ok(ModeCell {
        mode: c.mode.trim().to_owned(),
        part: c.part.trim().to_owned(),
        state,
        duty,
        active: c
            .active
            .as_deref()
            .filter(|s| !s.trim().is_empty())
            .map(|s| s.trim().parse::<u32>().map_err(invalid))
            .transpose()?,
        spare_state: c
            .spare_state
            .as_deref()
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .map(str::to_owned),
        eclipse: c.eclipse,
    })
}

/// 時間割合（`"0.1"`）。範囲と合計は設計の検査（`fune_model::check`）が見る。
fn fraction_from_text(text: &str) -> Result<f64> {
    text.trim()
        .parse::<f64>()
        .ok()
        .filter(|f| f.is_finite())
        .ok_or_else(|| invalid(format!("時間割合は数（0〜1）で書きます: {text}")))
}

/// `--duty tx=0.1,rx=0.2` → 状態 → 打った割合（`ModeCellText.duty`）。一覧は `list_from_text`、組は `pair_from_text`。
pub fn duty_from_text(text: &str) -> Result<BTreeMap<String, String>> {
    list_from_text(text)
        .into_iter()
        .map(|pair| {
            pair_from_text(&pair)
                .ok_or_else(|| invalid(format!("--duty は 状態=割合 の形です: {pair}")))
        })
        .collect()
}

/// `fune mode duration` と HTTP で共有する量の読み方。
pub fn mode_duration_op(mode: &str, duration: Option<&str>) -> Result<Op> {
    Ok(Op::SetModeDuration {
        mode: mode.trim().to_owned(),
        duration: duration.map(quantity_from_text).transpose()?,
    })
}

/// CLI と GUI の移動指定を同じ Op にする。移動先は必ず 1 つだけ指定する。
pub fn mode_move_op(
    id: &str,
    before: Option<&str>,
    after: Option<&str>,
    first: bool,
    last: bool,
) -> Result<Op> {
    use fune_ops::op::ModePosition;
    let position = match (before, after, first, last) {
        (Some(anchor), None, false, false) => ModePosition::Before(anchor.trim().to_owned()),
        (None, Some(anchor), false, false) => ModePosition::After(anchor.trim().to_owned()),
        (None, None, true, false) => ModePosition::First,
        (None, None, false, true) => ModePosition::Last,
        _ => {
            return Err(invalid(
                "--before / --after / --first / --last のどれか 1 つを指定します",
            ));
        }
    };
    Ok(Op::MoveMode {
        id: id.trim().to_owned(),
        position,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    /// ネットのフィールドの名前（`NetFieldName::NAMES`）は、どれも読めて同じ名前の型付きのフィールドになる。
    #[test]
    fn every_net_field_name_reads_back_to_the_same_name() {
        let sample = |name: NetFieldName| match name {
            NetFieldName::Kind => "data",
            NetFieldName::Source | NetFieldName::Master => "PCDU.OUT3",
            NetFieldName::Rail => "5 V",
            NetFieldName::Limit => "2 A",
            NetFieldName::Bitrate => "1 Mbit/s",
            NetFieldName::Label | NetFieldName::Protocol => "text",
        };
        for &name in NetFieldName::ALL {
            assert_eq!(net_field(name, Some(sample(name))).unwrap().key(), name);
            if name != NetFieldName::Kind {
                assert_eq!(net_field(name, None).unwrap().key(), name);
                let unset = NetUnsetText {
                    net: "P5V".into(),
                    field: format!(" {name} "),
                };
                assert!(net_unset_op(&unset).is_ok(), "{name}");
            }
        }
        assert!(net_field(NetFieldName::Kind, None).is_err());
        let unknown = NetUnsetText {
            net: "P5V".into(),
            field: "voltage".into(),
        };
        assert!(net_unset_op(&unknown).is_err());
    }

    /// `fune net set` は書いた旗だけを宣言の順に（GUI のレールの欄は `rail` だけ）。
    #[test]
    fn net_set_writes_only_given_flags_in_order() {
        let t = NetSetText {
            net: "P5V".into(),
            rail: Some("5.1 V".into()),
            label: Some("5 V 系".into()),
            ..Default::default()
        };
        let names: Vec<_> = net_set_ops(&t)
            .unwrap()
            .into_iter()
            .map(|op| match op {
                Op::SetNetField { field, .. } => field.name(),
                _ => "",
            })
            .collect();
        assert_eq!(names, ["label", "rail"]);
        // 旗を全部書けば、フィールドの名前の一覧（`NetFieldName::NAMES`）と同じ順になる
        let all = NetSetText {
            net: "CAN".into(),
            kind: Some("data".into()),
            label: Some("CAN".into()),
            source: Some("OBC.CAN1".into()),
            rail: Some("5 V".into()),
            limit: Some("1 A".into()),
            protocol: Some("CAN".into()),
            bitrate: Some("1 Mbit/s".into()),
            master: Some("OBC.CAN1".into()),
        };
        let names: Vec<_> = net_set_ops(&all)
            .unwrap()
            .into_iter()
            .map(|op| match op {
                Op::SetNetField { field, .. } => field.name(),
                _ => "",
            })
            .collect();
        assert_eq!(names, NetFieldName::NAMES);
        assert!(net_set_ops(&NetSetText::default()).is_err());
        let bad = NetSetText {
            net: "P5V".into(),
            rail: Some("5 parsec".into()),
            ..Default::default()
        };
        assert!(net_set_ops(&bad).is_err());
    }

    /// `--duty` の一覧と、GUI のセルの欄の割合（空の欄は書かない。行を消すセルは割合を持たない）。
    #[test]
    fn duty_text_reads_pairs_and_fractions() {
        let duty = duty_from_text(" tx=0.1 , rx = 0.25,").unwrap();
        assert_eq!(duty.get("tx").map(String::as_str), Some("0.1"));
        assert_eq!(duty.get("rx").map(String::as_str), Some("0.25"));
        assert!(duty_from_text("tx").is_err());
        assert!(duty_from_text("tx=").is_err());
        let cell = |state: Option<&str>, duty: &[(&str, &str)]| ModeCellText {
            mode: "NOMINAL".into(),
            part: "UHF_TRX".into(),
            state: state.map(str::to_owned),
            duty: duty
                .iter()
                .map(|(s, f)| ((*s).to_owned(), (*f).to_owned()))
                .collect(),
            active: None,
            spare_state: None,
            eclipse: false,
        };
        let op = mode_set_op(&ModeSetText {
            cells: vec![
                cell(Some("rx"), &[("tx", "0.1"), ("idle", " ")]),
                cell(None, &[("tx", "0.3")]),
            ],
        })
        .unwrap();
        let Op::SetModeStates { cells } = op else {
            panic!("{op:?}")
        };
        assert_eq!(cells[0].duty, BTreeMap::from([("tx".to_owned(), 0.1)]));
        assert_eq!((cells[1].state.as_deref(), cells[1].duty.len()), (None, 0));
        let bad = ModeSetText {
            cells: vec![cell(Some("rx"), &[("tx", "abc")])],
        };
        assert!(mode_set_op(&bad).is_err());
        assert!(mode_set_op(&ModeSetText::default()).is_err());
        // 空の状態は「行を消す」ではなく誤り（行を消すのは state が null のとき）
        let blank = ModeSetText {
            cells: vec![cell(Some(" "), &[])],
        };
        assert!(mode_set_op(&blank).is_err());
    }

    /// 配置: 位置・姿勢の文字列は `fune part place --position` と同じ読み方。取付先が空なら機体の根で、取付点も書かない。
    #[test]
    fn place_reads_vector_and_quaternion_text() {
        let t = PlaceText {
            part: "RW_X".into(),
            on: Some("PANEL_MZ".into()),
            point: Some("RW_SLOT_X".into()),
            position: Some("0 0 12 mm".into()),
            orientation: Some("1 0 0 0".into()),
        };
        let Op::Place {
            on,
            point,
            position,
            orientation,
            ..
        } = place_op(&t).unwrap()
        else {
            panic!()
        };
        assert_eq!(
            (on.as_deref(), point.as_deref()),
            (Some("PANEL_MZ"), Some("RW_SLOT_X"))
        );
        let p = position.unwrap();
        assert_eq!((p.z, p.unit.as_deref()), (12.0, Some("mm")));
        assert_eq!(orientation.unwrap().w, 1.0);
        let root = PlaceText {
            on: Some(" ".into()),
            position: Some(String::new()),
            ..t.clone()
        };
        let Op::Place {
            on,
            point,
            position,
            ..
        } = place_op(&root).unwrap()
        else {
            panic!()
        };
        assert_eq!((on, point, position), (None, None, None));
        let bad = PlaceText {
            position: Some("0 0 mm".into()),
            ..t
        };
        assert!(place_op(&bad).is_err());
    }
}
