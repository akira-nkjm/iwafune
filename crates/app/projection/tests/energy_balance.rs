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
