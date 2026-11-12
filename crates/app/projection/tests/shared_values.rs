//! 依存の向きで共有できず、fune-model と sat-analysis の両方が持つ値の一致（CONTRIBUTING.md の「1 か所だけ」）。

/// cold の予備機の状態は、慣例の停止状態と同じ綴り。
#[test]
fn cold_spare_state_is_the_off_state() {
    assert_eq!(
        sat_analysis::modes::COLD_SPARE_STATE,
        fune_model::builtin::STATE_OFF
    );
}
