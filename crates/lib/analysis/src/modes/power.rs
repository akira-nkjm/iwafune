//! モードでの部品の電源の状態（オンの時間・オフの時間があるか）。熱の温度範囲の選び方（保管か、動作と性能か）に使う。
//! **電源の状態の読み方はここだけ**: 台数・予備・duty は [`counted_states`] で数え、時間割合と台数が正の状態のうち
//! 停止状態 [`OFF_STATE`] をオフ、それ以外をオンとする（cold の予備機はオフ、warm の予備機は予備状態なのでオン）。

use super::{OFF_STATE, Population, StateAssignment, counted_states};
use crate::spec::AnalysisError;

/// モードでの部品の電源の状態。duty や予備機があれば両方とも真になりうる。
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct PowerStates {
    /// 電源の入った時間（停止状態でない状態）がある
    pub on: bool,
    /// 電源を切った時間（停止状態）がある
    pub off: bool,
}

impl PowerStates {
    /// オンの時間もオフの時間も無い（員数 0 など。電源の状態が決まらない）。
    pub fn is_unknown(self) -> bool {
        !self.on && !self.off
    }
}

/// 割り当て `a` での部品の電源の状態（台数・冗長・語彙は `p`）。
pub fn power_states(a: &StateAssignment, p: Population<'_>) -> Result<PowerStates, AnalysisError> {
    let mut out = PowerStates::default();
    for (state, fraction, count) in counted_states(a, p)? {
        if fraction <= 0.0 || count == 0 {
            continue;
        }
        if state == OFF_STATE {
            out.off = true;
        } else {
            out.on = true;
        }
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::modes::Redundancy;

    fn assign(state: &str, duty: &[(&str, f64)], active: Option<u32>) -> StateAssignment {
        StateAssignment {
            part: "P".to_owned(),
            state: state.to_owned(),
            duty: duty.iter().map(|(s, f)| ((*s).to_owned(), *f)).collect(),
            active,
            spare_state: None,
        }
    }

    fn population(count: u32, redundancy: Redundancy, states: &[String]) -> Population<'_> {
        Population {
            count,
            redundancy,
            states,
        }
    }

    /// off だけ・on だけ・off に on の duty 10 %・cold の予備 1 台（稼働 1 台は on）・員数 0。
    #[test]
    fn on_and_off_by_hand() {
        let states = vec!["off".to_owned(), "on".to_owned()];
        let one = population(1, Redundancy::None, &states);
        let both = PowerStates {
            on: true,
            off: true,
        };
        let read = |a: StateAssignment, p| power_states(&a, p).unwrap();
        assert_eq!(
            read(assign("off", &[], None), one),
            PowerStates {
                on: false,
                off: true
            }
        );
        assert_eq!(
            read(assign("on", &[], None), one),
            PowerStates {
                on: true,
                off: false
            }
        );
        assert_eq!(read(assign("off", &[("on", 0.1)], None), one), both);
        // duty 0 の状態は数えない
        assert!(!read(assign("off", &[("on", 0.0)], None), one).on);
        let cold = population(2, Redundancy::Cold, &states);
        assert_eq!(read(assign("on", &[], Some(1)), cold), both);
        let none = population(0, Redundancy::None, &states);
        assert!(read(assign("on", &[], None), none).is_unknown());
    }
}
