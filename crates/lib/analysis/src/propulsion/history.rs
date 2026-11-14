//! 軌道の推移: マヌーバの順に「いつ・どの軌道にいるか」（[`OrbitHistory`]）。**時刻ごとの軌道の引き方はここだけ。**
//!
//! - 時刻は最初の軌道の元期からの経過。最初のマヌーバは元期に始め、次のマヌーバは前のマヌーバの終わりに続けて始める
//!   （待ちの時間はフェーズ（#9）で足す）。
//! - 区間の軌道は [`OrbitSpec`]（元期は区間の始め、伝搬の方法は最初の軌道と同じ）。数値伝搬のマヌーバの間は伝搬の点
//!   （ほぼ 1 周ごと）ごとに区間を作り、終わりの状態から次の区間を作る。ほかのマヌーバは終わりの時刻に、前の軌道の
//!   平均要素の長半径・離心率・傾斜角を目標に替えた軌道に移る（RAAN・近点引数・平均近点角はそのまま）。
//! - 区間の軌道を作るのは `flight`（伝搬とまとめて 1 回だけ歩く）。ここは引く側だけを持つ。

use crate::spec::AnalysisError;
use sat_astro::orbit::OrbitSpec;
use sat_units::typed::{Mass, Qty, Time};
use serde::{Deserialize, Serialize};

/// 軌道の推移の 1 区間。
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct OrbitSegment {
    /// 区間の始め（最初の軌道の元期からの経過）
    pub start: Qty<Time>,
    /// この区間の軌道を作ったマヌーバ（最初の軌道なら None）
    pub maneuver: Option<String>,
    /// 数値伝搬のマヌーバの噴射の途中か
    pub thrusting: bool,
    /// 区間の始めの質量（推薬を求めたときだけ）
    pub mass: Option<Qty<Mass>>,
    /// 区間の軌道（元期は区間の始め）
    pub orbit: OrbitSpec,
}

/// マヌーバの順の軌道の推移（区間は始めの時刻の昇順。最初の区間は最初の軌道で、始めは 0）。
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct OrbitHistory {
    pub segments: Vec<OrbitSegment>,
}

impl OrbitHistory {
    /// 最初の軌道だけの推移（マヌーバが無いとき）。
    pub fn constant(orbit: OrbitSpec) -> Self {
        Self {
            segments: vec![OrbitSegment {
                start: Qty::ZERO,
                maneuver: None,
                thrusting: false,
                mass: None,
                orbit,
            }],
        }
    }

    /// 時刻 `t_s`（最初の軌道の元期からの経過）の区間と、区間の元期からの経過 [s]。
    /// `t_s` が最初の区間より前なら最初の区間（経過は負のまま）。
    pub fn locate(&self, t_s: f64) -> Result<(&OrbitSegment, f64), AnalysisError> {
        let i = self
            .segments
            .partition_point(|s| s.start.value() <= t_s)
            .saturating_sub(1);
        let seg = self
            .segments
            .get(i)
            .ok_or_else(|| AnalysisError::InvalidInput("軌道の推移が空です".into()))?;
        Ok((seg, t_s - seg.start.value()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use sat_astro::time::Epoch;

    fn spec(alt_km: f64) -> OrbitSpec {
        OrbitSpec::circular(
            Epoch { j2000_s: 0.0 },
            Qty::from_raw(alt_km * 1e3),
            Qty::from_raw(0.5),
            Qty::ZERO,
        )
    }

    #[test]
    fn locate_picks_the_last_segment_started() {
        let mut h = OrbitHistory::constant(spec(550.0));
        h.segments.push(OrbitSegment {
            start: Qty::from_raw(100.0),
            maneuver: Some("RAISE".into()),
            thrusting: false,
            mass: None,
            orbit: spec(600.0),
        });
        let (seg, dt) = h.locate(50.0).unwrap();
        assert!(seg.maneuver.is_none() && (dt - 50.0).abs() < 1e-12);
        let (seg, dt) = h.locate(250.0).unwrap();
        assert_eq!(seg.maneuver.as_deref(), Some("RAISE"));
        assert!((dt - 150.0).abs() < 1e-12);
        assert!((h.locate(100.0).unwrap().1).abs() < 1e-12);
    }
}
