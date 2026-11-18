//! デモの電力収支の入力: モードの並び・最長継続時間・基本のモード・モードごとの姿勢則の写し替え。

use fune_projection::Context;
use std::path::Path;

fn demo() -> (fune_model::Design, fune_model::Resolved) {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../../fixtures/demo-6u");
    let design = fune_store::load(&root)
        .expect("the demo should load")
        .design;
    let resolved = fune_model::resolve(&design);
    (design, resolved)
}

/// 電力収支にモードの並び・最長継続時間（SI の秒）・モードごとの姿勢則を渡す。基本のモードは書かなければ None（既定は解析）。
#[test]
fn energy_balance_gets_the_sequence_and_max_durations() {
    let (design, resolved) = demo();
    let ctx = Context::new(&design, &resolved, None).unwrap();
    let input = fune_projection::power::energy_balance(&ctx).unwrap();
    let seq: Vec<(&str, f64)> = input
        .sequence
        .iter()
        .map(|s| (s.mode.as_str(), s.duration.value()))
        .collect();
    assert_eq!(
        seq,
        [
            ("NOMINAL", 4560.0),
            ("MISSION", 300.0),
            ("NOMINAL", 300.0),
            ("DOWNLINK", 576.0)
        ]
    );
    let duration = |id: &str| {
        let c = input.cases.iter().find(|c| c.id == id).unwrap();
        c.max_duration.map(|d| d.value())
    };
    assert_eq!(duration("DOWNLINK"), Some(900.0));
    assert_eq!(duration("MISSION"), Some(600.0));
    assert_eq!(duration("SAFE"), None);
    assert_eq!(input.base_mode, None);
    assert_eq!(input.mode_attitudes.len(), 4);
}

/// model と解析の同名の型（依存の向きで共有できない）の綴りが一致する。
#[test]
fn timing_spellings_agree_between_model_and_analysis() {
    use fune_model::design::{ModeWhen, StartFrom};
    use sat_analysis::power::energy_balance::{EclipseEvent, TimeWindow};
    let model_when: Vec<String> = ModeWhen::ALL
        .iter()
        .map(|w| w.as_str().to_owned())
        .collect();
    let analysis_when: Vec<String> = [TimeWindow::Any, TimeWindow::Sunlit, TimeWindow::Eclipse]
        .iter()
        .map(|w| {
            serde_json::to_value(w)
                .unwrap()
                .as_str()
                .unwrap()
                .to_owned()
        })
        .collect();
    assert_eq!(model_when, analysis_when);
    let model_from: Vec<String> = StartFrom::ALL
        .iter()
        .map(|w| w.as_str().to_owned())
        .collect();
    let analysis_from: Vec<String> = [EclipseEvent::EclipseExit, EclipseEvent::EclipseEntry]
        .iter()
        .map(|w| {
            serde_json::to_value(w)
                .unwrap()
                .as_str()
                .unwrap()
                .to_owned()
        })
        .collect();
    assert_eq!(model_from, analysis_from);
}

/// 使う並び（名前付き）・並びの基準・モードの時刻の指定が解析の入力に写る。基準のオフセットは SI の秒。
#[test]
fn the_active_named_sequence_and_mode_timing_reach_the_input() {
    use fune_model::design::{Mode, ModeSegment, ModeWhen, SequenceSetting, StartFrom, StartRef};
    use fune_model::value::Quantity;
    use sat_analysis::power::energy_balance::{EclipseEvent, TimeWindow};
    let (mut design, _) = demo();
    let power = design.analysis.power.as_mut().unwrap();
    power.sequences.insert(
        "imaging_day".into(),
        SequenceSetting {
            anchor: Some(StartRef {
                from: StartFrom::EclipseExit,
                offset: Some(Quantity::new(2.0, "min")),
            }),
            segments: vec![ModeSegment {
                mode: "MISSION".into(),
                duration: Quantity::new(10.0, "min"),
            }],
        },
    );
    let input = |d: &fune_model::Design| {
        let resolved = fune_model::resolve(d);
        let ctx = Context::new(d, &resolved, None).unwrap();
        fune_projection::power::energy_balance(&ctx).unwrap()
    };
    // 使う並びを選ばなければ従来の `[[sequence]]`（default）で、名前も基準も付かない
    let before = input(&design);
    assert_eq!(before.sequence.len(), 4);
    assert_eq!(before.sequence_timing.name, None);
    assert_eq!(before.sequence_timing.anchor, None);
    // 選ぶと、その並びと基準（オフセットは秒）が入る
    design.analysis.power.as_mut().unwrap().active_sequence = Some("imaging_day".into());
    let named = input(&design);
    assert_eq!(named.sequence.len(), 1);
    assert_eq!(named.sequence[0].duration.value(), 600.0);
    assert_eq!(named.sequence_timing.name.as_deref(), Some("imaging_day"));
    let anchor = named.sequence_timing.anchor.unwrap();
    assert_eq!(
        (anchor.from, anchor.offset.value()),
        (EclipseEvent::EclipseExit, 120.0)
    );
    // モードの時刻の指定は、そのモードのケースに入る
    let mission = Mode {
        when: Some(ModeWhen::Sunlit),
        start: Some(StartRef {
            from: StartFrom::EclipseEntry,
            offset: None,
        }),
        ..design.modes["MISSION"].clone()
    };
    design.modes.insert("MISSION".into(), mission);
    let timed = input(&design);
    let case = timed.cases.iter().find(|c| c.id == "MISSION").unwrap();
    assert_eq!(case.timing.when, TimeWindow::Sunlit);
    let start = case.timing.start.unwrap();
    assert_eq!(
        (start.from, start.offset.value()),
        (EclipseEvent::EclipseEntry, 0.0)
    );
    let other = timed.cases.iter().find(|c| c.id == "SAFE").unwrap();
    assert!(other.timing.is_unspecified());
    // 時間の次元でないオフセットは不足入力（誤りとして返る）
    design
        .modes
        .get_mut("MISSION")
        .unwrap()
        .start
        .as_mut()
        .unwrap()
        .offset = Some(Quantity::new(1.0, "kg"));
    let resolved = fune_model::resolve(&design);
    let ctx = Context::new(&design, &resolved, None).unwrap();
    assert!(fune_projection::power::energy_balance(&ctx).is_err());
}
