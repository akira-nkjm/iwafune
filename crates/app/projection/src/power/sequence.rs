//! モードの並びとモードの時刻の指定の射影（`analysis.power` の `[[sequence]]`・`[sequences.<名前>]`・`active_sequence` と、
//! モードの `when`・`start`）。**どの並びを使うかは model の `PowerSettings::active_sequence_view`**（ここは解析の型に写すだけ）。

use super::power_key;
use crate::context::Context;
use crate::missing::MissingInputs;
use crate::quantity::{qty_opt, setting};
use fune_model::Target;
use fune_model::design::{Mode, ModeWhen, SequenceView, StartFrom, StartRef, setting_dims as dims};
use sat_analysis::power::energy_balance::EclipseEvent as AnalysisFrom;
use sat_analysis::power::energy_balance::TimeWindow as AnalysisWhen;
use sat_analysis::power::energy_balance::{
    EnergyBalanceSegment, EventStart as AnalysisStart, ModeTiming, SequenceTiming,
};
use sat_units::typed::Qty;

/// 解析に使うモードの並び（`analysis.power.active_sequence`、無ければ `default`、それも無ければ名前の順で最初）と、
/// その名前・先頭の基準。区間の長さ・基準のオフセットの次元が違えば不足入力。
pub(super) fn active(
    ctx: &Context<'_>,
    m: &mut MissingInputs,
) -> (Vec<EnergyBalanceSegment>, SequenceTiming) {
    let Some(p) = ctx.design.analysis.power.as_ref() else {
        return (Vec::new(), SequenceTiming::default());
    };
    let present = [
        ("sequence", !p.sequence.is_empty()),
        ("sequences", !p.sequences.is_empty()),
        ("active_sequence", p.active_sequence.is_some()),
    ];
    for (key, _) in present.iter().filter(|(_, written)| *written) {
        ctx.note(power_key(&[key]));
    }
    let Some(view) = p.active_sequence_view() else {
        return (Vec::new(), SequenceTiming::default());
    };
    let segments = segments(&view, m);
    let timing = SequenceTiming {
        // 従来の形（`[[sequence]]`）は名前が無いので、出力の名前も付けない
        name: (!view.legacy).then(|| view.name.to_owned()),
        anchor: view.anchor.and_then(|a| {
            start_ref(
                a,
                dims::POWER_SEQUENCES_ANCHOR_OFFSET.at(&[view.name]).into(),
                m,
            )
        }),
    };
    (segments, timing)
}

/// 並びの区間。長さの次元の表の欄は、従来の形と名前付きの形で別（どちらも `Time`）。
fn segments(view: &SequenceView<'_>, m: &mut MissingInputs) -> Vec<EnergyBalanceSegment> {
    view.segments
        .iter()
        .enumerate()
        .filter_map(|(i, s)| {
            let index = i.to_string();
            let duration = if view.legacy {
                setting(
                    &dims::POWER_SEQUENCE_DURATION,
                    &[index.as_str()],
                    Some(&s.duration),
                    "Length of the mode sequence interval",
                    m,
                )
            } else {
                setting(
                    &dims::POWER_SEQUENCES_DURATION,
                    &[view.name, index.as_str()],
                    Some(&s.duration),
                    "Length of the mode sequence interval",
                    m,
                )
            }?;
            Some(EnergyBalanceSegment {
                mode: s.mode.clone(),
                duration,
            })
        })
        .collect()
}

/// モードの時刻の指定（使う時間帯・開始の基準）。オフセットの次元が違えば不足入力。
pub(super) fn mode_timing(md: &Mode, m: &mut MissingInputs) -> ModeTiming {
    ModeTiming {
        when: match md.when.unwrap_or(ModeWhen::Any) {
            ModeWhen::Any => AnalysisWhen::Any,
            ModeWhen::Sunlit => AnalysisWhen::Sunlit,
            ModeWhen::Eclipse => AnalysisWhen::Eclipse,
        },
        start: md
            .start
            .as_ref()
            .and_then(|s| start_ref(s, Target::mode(md.id.as_str(), None), m)),
    }
}

/// 食の出入りからの指定（オフセットが無ければ 0 s）。
fn start_ref(r: &StartRef, target: Target, m: &mut MissingInputs) -> Option<AnalysisStart> {
    let offset = match &r.offset {
        None => Qty::from_raw(0.0),
        Some(q) => qty_opt::<sat_units::typed::Time>(Some(q), target, m)?,
    };
    Some(AnalysisStart {
        from: match r.from {
            StartFrom::EclipseExit => AnalysisFrom::EclipseExit,
            StartFrom::EclipseEntry => AnalysisFrom::EclipseEntry,
        },
        offset,
    })
}
