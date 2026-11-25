//! 名詞のコマンドに打った引数 → Op（`fune net set|unset`・`fune part place`・`fune mode set|unset`、`POST /ops/from_command`）。入口は [`command_ops`]（CLI はコマンドごとの関数を直接呼ぶ）。定義は `super::defs`、マヌーバは `super::maneuver`、配置は `super::place`、値の読み方は `super::text`。

use super::text::{
    fraction_from_text, invalid, list_from_text, net_kind, pair_from_text, quantity_from_text,
};
use crate::dto::{CommandText, ModeCellText, ModeSetText, NetSetText, NetUnsetText};
use crate::error::Result;
use fune_model::design::{ModeSegment, ModeWhen, NetField, NetFieldName, StartFrom, StartRef};
use fune_model::path::PortRef;
use fune_ops::{ModeCell, Op};
use std::collections::BTreeMap;

/// 打った引数 → Op の列（`POST /ops/from_command`）。適用はしない（`POST /ops` で送る）。
pub fn command_ops(t: &CommandText) -> Result<Vec<Op>> {
    Ok(match t {
        CommandText::NetSet(t) => net_set_ops(t)?,
        CommandText::NetUnset(t) => vec![net_unset_op(t)?],
        CommandText::PartPlace(t) => vec![super::place::place_op(t)?],
        CommandText::ThermalSet(t) => super::thermal::thermal_set_ops(t)?,
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
        CommandText::ModeTiming {
            mode,
            when,
            start,
            offset,
        } => vec![mode_timing_op(
            mode,
            when.as_deref(),
            start.as_deref(),
            offset.as_deref(),
        )?],
        CommandText::SequenceSet {
            name,
            segments,
            anchor,
            offset,
        } => vec![sequence_set_op(
            name,
            segments,
            anchor.as_deref(),
            offset.as_deref(),
        )?],
        CommandText::SequenceRemove { name } => vec![Op::RemoveSequence {
            name: name.trim().to_owned(),
        }],
        CommandText::AttrAdd(t) => vec![super::defs::attr_add_op(t)?],
        CommandText::AttrReplace(t) => vec![super::defs::attr_replace_op(t)?],
        CommandText::KindAdd(t) => vec![super::defs::kind_add_op(t)?],
        CommandText::KindReplace(t) => vec![super::defs::kind_replace_op(t)?],
        CommandText::ManeuverAdd(t) => vec![super::maneuver::maneuver_op(t, true)?],
        CommandText::ManeuverSet(t) => vec![super::maneuver::maneuver_op(t, false)?],
        CommandText::PhaseAdd(t) => vec![super::phase::phase_op(t, true)?],
        CommandText::PhaseSet(t) => vec![super::phase::phase_op(t, false)?],
        CommandText::PhaseField { phase, path, value } => {
            vec![super::phase::phase_field_op(phase, path, value.as_deref())?]
        }
        CommandText::PhaseCopyOrbit { phase } => vec![super::phase::copy_orbit_op(phase)],
        CommandText::PhaseUnsetOrbit { phase } => vec![super::phase::unset_orbit_op(phase)],
        CommandText::PhaseUnsetLaw { phase, mode } => {
            vec![super::phase::unset_law_op(phase, mode)]
        }
        CommandText::CasePhase { case, phase } => {
            vec![super::phase::case_phase_op(case, phase.as_deref())]
        }
        CommandText::CaseEpoch { case, epoch } => {
            vec![super::phase::case_epoch_op(case, epoch.as_deref())?]
        }
        CommandText::ConfigurationAdd(t) => vec![super::configuration::configuration_op(t, true)],
        CommandText::ConfigurationSet(t) => vec![super::configuration::configuration_op(t, false)],
        CommandText::GroundAdd(t) => vec![super::ground::ground_add_op(t)?],
        CommandText::RequirementAdd(t) => vec![super::requirement::requirement_op(t, true)?],
        CommandText::RequirementSet(t) => vec![super::requirement::requirement_op(t, false)?],
        CommandText::CatalogSourceAdd(t) => vec![super::catalog::catalog_source_add_op(t)],
        CommandText::CatalogLocalAdd(t) => vec![super::catalog::catalog_local_add_op(t)],
        CommandText::LinkAdd(t) => vec![super::link::link_add_op(t)?],
        CommandText::CaseSet(t) => vec![super::case::case_set_op(t)?],
        CommandText::ProjectSet(t) => vec![super::project::project_field_op(t)?],
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
        return Err(invalid("No field flag to write (such as --rail)"));
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
            text.ok_or_else(|| invalid("Cannot unset the kind of a net"))?,
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

/// `fune mode set|unset` のセルの列 → 1 つの `SetModeStates`。
pub fn mode_set_op(t: &ModeSetText) -> Result<Op> {
    if t.cells.is_empty() {
        return Err(invalid("No mode-table cell given"));
    }
    let cells = t.cells.iter().map(mode_cell).collect::<Result<_>>()?;
    Ok(Op::SetModeStates { cells })
}

fn mode_cell(c: &ModeCellText) -> Result<ModeCell> {
    let state = c.state.as_deref().map(str::trim);
    if state == Some("") {
        return Err(invalid(format!(
            "The base state of {} for {} is empty (to remove the row, use `fune mode unset`)",
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

/// `--duty tx=0.1,rx=0.2` → 状態 → 打った割合（`ModeCellText.duty`）。一覧は `list_from_text`、組は `pair_from_text`。
pub fn duty_from_text(text: &str) -> Result<BTreeMap<String, String>> {
    list_from_text(text)
        .into_iter()
        .map(|pair| {
            pair_from_text(&pair)
                .ok_or_else(|| invalid(format!("--duty must be state=fraction: {pair}")))
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

/// `fune mode timing` と HTTP で共有する時刻の指定の読み方（綴りは `ModeWhen`・`StartFrom` の `FromStr`）。
pub fn mode_timing_op(
    mode: &str,
    when: Option<&str>,
    start: Option<&str>,
    offset: Option<&str>,
) -> Result<Op> {
    Ok(Op::SetModeTiming {
        mode: mode.trim().to_owned(),
        when: trimmed(when)
            .map(|w| w.parse::<ModeWhen>().map_err(|e| invalid(e.to_string())))
            .transpose()?,
        start: start_from_text(start, offset, "--start")?,
    })
}

/// `fune sequence set` と HTTP で共有する並びの読み方。区間は `モード=長さ` のカンマ区切り（1 つ以上）。
pub fn sequence_set_op(
    name: &str,
    segments: &str,
    anchor: Option<&str>,
    offset: Option<&str>,
) -> Result<Op> {
    let segments = list_from_text(segments)
        .into_iter()
        .map(|pair| {
            let (mode, duration) = pair_from_text(&pair)
                .ok_or_else(|| invalid(format!("--segments must be mode=duration: {pair}")))?;
            Ok(ModeSegment {
                mode,
                duration: quantity_from_text(&duration)?,
            })
        })
        .collect::<Result<Vec<_>>>()?;
    if segments.is_empty() {
        return Err(invalid("--segments needs at least one mode=duration"));
    }
    Ok(Op::SetSequence {
        name: name.trim().to_owned(),
        anchor: start_from_text(anchor, offset, "--anchor")?,
        segments,
    })
}

/// 基準（`eclipse_exit`・`eclipse_entry`）とオフセットの文字列 → 食の出入りからの指定。基準が無ければ None（オフセットだけは誤り）。
fn start_from_text(
    from: Option<&str>,
    offset: Option<&str>,
    flag: &str,
) -> Result<Option<StartRef>> {
    match (trimmed(from), trimmed(offset)) {
        (None, None) => Ok(None),
        (None, Some(_)) => Err(invalid(format!("--offset needs {flag}"))),
        (Some(from), offset) => Ok(Some(StartRef {
            from: from
                .parse::<StartFrom>()
                .map_err(|e| invalid(e.to_string()))?,
            offset: offset.map(quantity_from_text).transpose()?,
        })),
    }
}

/// 旗の文字列（前後の空白を除く。空なら None）。
fn trimmed(text: Option<&str>) -> Option<&str> {
    text.map(str::trim).filter(|t| !t.is_empty())
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
                "Give exactly one of --before / --after / --first / --last",
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
}
