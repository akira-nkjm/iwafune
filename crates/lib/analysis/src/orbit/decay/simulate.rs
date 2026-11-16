//! 周回平均の式（[`super::rates`]）を時間に積分する: 再突入までの高度低下と、高度を保つときの ΔV。
//!
//! 状態は `[長半径 a, RAAN, 積算 ΔV]`。RAAN は J2 の永年変化率（`orbit::j2_secular_rates`。今の `a` で）で進める
//! （太陽同期でない軌道では、大気の昼側の膨らみに対する軌道面の向きが変わる）。積分は `sat_astro::ode::rk4_step`。
//! 刻みは「1 刻みの高度低下が [`MAX_DROP_PER_STEP_M`] 以下」で決め、[`MAX_STEP_S`]〜[`MIN_STEP_S`] に収める。
//! 日付で引いた宇宙天気の標本の境目では刻みを切る（標本の間は一定）。

use super::rates::{MeanDrag, mean_drag};
use super::weather::Weather;
use crate::spec::AnalysisError;
use sat_astro::constants::SECONDS_PER_DAY;
use sat_astro::ode::rk4_step;
use sat_astro::orbit::{OrbitSpec, Propagator, altitude_from_semi_major_axis, j2_secular_rates};
use std::cell::RefCell;

/// 刻みの上限 [s]（10 日。季節変化・RAAN の歳差・宇宙天気の標本の間隔より短い）。
pub const MAX_STEP_S: f64 = 10.0 * SECONDS_PER_DAY;
/// 刻みの下限 [s]（10 分。再突入の直前で刻みが縮みすぎないように）。
pub const MIN_STEP_S: f64 = 600.0;
/// 1 刻みの高度低下の上限 [m]。
pub const MAX_DROP_PER_STEP_M: f64 = 1_000.0;
/// 高度の時系列を記録する最小の間隔 [s]（10 日。再突入の点と最後の点は必ず残す）。
pub const RECORD_STEP_S: f64 = 10.0 * SECONDS_PER_DAY;
/// 積分の刻みの数の上限（入力の誤りで止まらないときの安全弁）。
const MAX_STEPS: usize = 200_000;

/// 積分する式のもと（軌道・`Cd·A/m`・宇宙天気）。
pub(super) struct Model<'a> {
    pub base: &'a OrbitSpec,
    pub cd_a_over_m: f64,
    pub weather: Weather<'a>,
}

impl Model<'_> {
    /// 経過 `t` [s] の長半径 `a` [m]・RAAN [rad] での周回平均。
    pub fn mean(&self, t: f64, a: f64, raan: f64) -> Result<MeanDrag, AnalysisError> {
        mean_drag(
            self.base,
            t,
            a,
            raan,
            self.cd_a_over_m,
            &self.weather.atmosphere(t),
        )
    }

    /// 平均の高度（長半径 − 中心天体の半径）[m]。
    fn altitude(&self, a: f64) -> f64 {
        altitude_from_semi_major_axis(self.base.central, a)
    }

    /// 近地点の高度 [m]（離心率は元期の値のまま）。再突入はこれで決める。
    pub fn perigee_altitude(&self, a: f64) -> f64 {
        altitude_from_semi_major_axis(
            self.base.central,
            a * (1.0 - self.base.elements.eccentricity),
        )
    }

    fn raan_rate(&self, a: f64) -> f64 {
        let el = &self.base.elements;
        match self.base.propagator {
            Propagator::Kepler => 0.0,
            _ => j2_secular_rates(a, el.eccentricity, el.inclination.value()).0,
        }
    }

    /// `[a, RAAN, ΔV]` の微分。`hold` なら長半径を保つ（抗力を連続して打ち消す）。誤りは `err` に残して 0 を返す。
    fn rates(
        &self,
        t: f64,
        x: &[f64; 3],
        hold: bool,
        err: &RefCell<Option<AnalysisError>>,
    ) -> [f64; 3] {
        match self.mean(t, x[0], x[1]) {
            Ok(m) => [
                if hold { 0.0 } else { m.a_dot },
                self.raan_rate(x[0]),
                m.dv_rate,
            ],
            Err(e) => {
                err.borrow_mut().get_or_insert(e);
                [0.0; 3]
            }
        }
    }

    /// 長半径の低下の速さ [m/s]（正が低下）。刻みの最初の見積もりに使う。
    fn drop_rate(&self, t: f64, x: &[f64; 3]) -> Result<f64, AnalysisError> {
        Ok(-self.mean(t, x[0], x[1])?.a_dot)
    }

    /// 1 刻み進める（刻みは高度低下・上下限・期間の端・宇宙天気の境目で決める）。
    /// `max_drop` [m] は 1 刻みの高度低下の上限、`drop` [m/s] はその見積もりに使う低下の速さ（前の刻みの平均）。
    fn step(
        &self,
        t: f64,
        x: &[f64; 3],
        end: f64,
        hold: bool,
        (max_drop, drop): (f64, f64),
    ) -> Result<(f64, [f64; 3]), AnalysisError> {
        let err = RefCell::new(None);
        let f = |t: f64, x: &[f64; 3]| self.rates(t, x, hold, &err);
        let mut dt = if drop > 0.0 {
            max_drop / drop
        } else {
            MAX_STEP_S
        };
        dt = dt.clamp(MIN_STEP_S, MAX_STEP_S).min(end - t);
        if let Some(next) = self.weather.next_change(t).filter(|n| *n - t > 1.0) {
            dt = dt.min(next - t);
        }
        let x1 = rk4_step(f, t, x, dt);
        match err.into_inner() {
            Some(e) => Err(e),
            None => Ok((t + dt, x1)),
        }
    }
}

/// 再突入まで（または期間の上限まで）の高度の推移。
#[derive(Debug, Clone, PartialEq)]
pub(super) struct DecayTrack {
    /// 再突入の経過 [s]（期間の中で再突入しなければ `None`）
    pub reentry: Option<f64>,
    /// 最後の高度 [m]（再突入したら再突入の高度）
    pub final_altitude: f64,
    /// 経過 `probe` [s] の高度 [m]（それより前に再突入したら再突入の高度）
    pub altitude_at_probe: f64,
    /// 記録した (経過 [s], 高度 [m])
    pub points: Vec<(f64, f64)>,
}

/// 長半径 `a0` から、近地点の高度が `reentry_altitude` [m] を下回るか `span` [s] に達するまで積分する。
/// 記録する高度は平均の高度（近円軌道では近地点の高度と同じ）。
pub(super) fn decay(
    model: &Model<'_>,
    a0: f64,
    span: f64,
    reentry_altitude: f64,
    probe: f64,
) -> Result<DecayTrack, AnalysisError> {
    let (mut t, mut x) = (0.0, [a0, model.base.elements.raan.value(), 0.0]);
    let mut points = vec![(0.0, model.altitude(a0))];
    let mut at_probe = None;
    let mut drop = model.drop_rate(t, &x)?;
    for _ in 0..MAX_STEPS {
        if t >= span {
            break;
        }
        let (t1, x1) = model.step(t, &x, span, false, (MAX_DROP_PER_STEP_M, drop))?;
        drop = (x[0] - x1[0]) / (t1 - t);
        let (h0, h1) = (model.altitude(x[0]), model.altitude(x1[0]));
        let (p0, p1) = (model.perigee_altitude(x[0]), model.perigee_altitude(x1[0]));
        if at_probe.is_none() && t1 >= probe {
            at_probe = Some(interpolate(t, h0, t1, h1, probe));
        }
        if p1 <= reentry_altitude {
            let tr = t + (t1 - t) * (p0 - reentry_altitude) / (p0 - p1);
            let hr = interpolate(t, h0, t1, h1, tr);
            points.push((tr, hr));
            return Ok(DecayTrack {
                reentry: Some(tr),
                final_altitude: hr,
                altitude_at_probe: at_probe.unwrap_or(hr),
                points,
            });
        }
        (t, x) = (t1, x1);
        let last = points.last().map_or(0.0, |p| p.0);
        if t - last >= RECORD_STEP_S || t >= span {
            points.push((t, h1));
        }
    }
    if t < span {
        return Err(AnalysisError::InvalidInput(
            "The altitude decay integration does not finish within the step limit".into(),
        ));
    }
    let h = model.altitude(x[0]);
    Ok(DecayTrack {
        reentry: None,
        final_altitude: h,
        altitude_at_probe: at_probe.unwrap_or(h),
        points,
    })
}

fn interpolate(t0: f64, v0: f64, t1: f64, v1: f64, t: f64) -> f64 {
    if t1 > t0 {
        v0 + (v1 - v0) * (t - t0) / (t1 - t0)
    } else {
        v1
    }
}

/// 高度を保つときの ΔV の積算。
#[derive(Debug, Clone, Copy, PartialEq)]
pub(super) struct HoldTrack {
    /// 期間全体の ΔV [m/s]
    pub delta_v: f64,
    /// 帯の下端で上げた回数（帯が無ければ `None`）
    pub burns: Option<u32>,
    /// 最後に上げた経過 [s]（上げていなければ `None`）
    pub last_burn: Option<f64>,
}

/// 長半径 `a0` の高度を `span` [s] のあいだ保つ ΔV。`band` [m] があれば、高度が `band` だけ下がるたびに
/// `a0` へ瞬時に戻す（上げる ΔV は、その間に打ち消さなかった進行方向の抗力の積分に等しい。1 次の近似）。
/// 無ければ抗力を連続して打ち消す。
pub(super) fn hold(
    model: &Model<'_>,
    a0: f64,
    span: f64,
    band: Option<f64>,
) -> Result<HoldTrack, AnalysisError> {
    let (mut t, mut x) = (0.0, [a0, model.base.elements.raan.value(), 0.0]);
    let (mut burns, mut last_burn) = (0_u32, None);
    // 帯を飛び越さないよう、1 刻みの高度低下を帯の 1/4 までにする。
    let max_drop = band.map_or(MAX_DROP_PER_STEP_M, |b| (b / 4.0).min(MAX_DROP_PER_STEP_M));
    let mut drop = match band {
        Some(_) => model.drop_rate(t, &x)?,
        None => 0.0,
    };
    for _ in 0..MAX_STEPS {
        if t >= span {
            return Ok(HoldTrack {
                delta_v: x[2],
                burns: band.map(|_| burns),
                last_burn,
            });
        }
        let (t1, x1) = model.step(t, &x, span, band.is_none(), (max_drop, drop))?;
        if band.is_some() {
            drop = (x[0] - x1[0]) / (t1 - t);
        }
        (t, x) = (t1, x1);
        if band.is_some_and(|b| x[0] <= a0 - b) {
            x[0] = a0;
            burns += 1;
            last_burn = Some(t);
        }
    }
    Err(AnalysisError::InvalidInput(
        "The orbit maintenance integration does not finish within the step limit".into(),
    ))
}
