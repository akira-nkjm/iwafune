//! 依存の向きで共有できず、fune-model と sat-analysis の両方が持つ値の一致（CONTRIBUTING.md の「1 か所だけ」）。

/// 解析の停止状態（cold の予備機・電源のオフ）は、慣例の停止状態と同じ綴り。
#[test]
fn analysis_off_state_is_the_off_state() {
    assert_eq!(
        sat_analysis::modes::OFF_STATE,
        fune_model::builtin::STATE_OFF
    );
}

/// 出典の日付の形の判定（`fune_model::value::is_calendar_date`）は、暦の正本（`sat_astro::time::Epoch::parse`）が
/// 日付だけの文字列を受けるかと一致する（うるう年・月末・形の誤り）。
#[test]
fn calendar_dates_agree_with_the_epoch_parser() {
    let mut texts: Vec<String> = Vec::new();
    for year in [1900, 2000, 2023, 2024, 2026] {
        for month in 0..=13 {
            for day in [0, 1, 28, 29, 30, 31, 32] {
                texts.push(format!("{year:04}-{month:02}-{day:02}"));
            }
        }
    }
    texts.extend(["2026-1-05", "26-01-05", "2026-01-5", "2026/01/05", "x"].map(String::from));
    for t in &texts {
        assert_eq!(
            fune_model::value::is_calendar_date(t),
            sat_astro::time::Epoch::parse(t).is_ok(),
            "{t}"
        );
    }
}
