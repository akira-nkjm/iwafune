//! 軌道上の時系列（時刻・位置・速度・太陽方向・日陰・機体姿勢）。
//! **時系列を使う解析（電源・姿勢・センサ・通信・熱）は、自分でループを書かずにこれを使う。**

mod evaluator;

pub use evaluator::TimelineEvaluator;

use crate::AstroError;
use crate::attitude::{AttitudeLaw, body_to_eci};
use crate::ephem::Body;
use crate::math::{Quat, Vec3};
use crate::orbit::{OrbitSpec, OrbitState, raan_for_beta};
use crate::shadow::{ShadowModel, central_shadow_factor};
use crate::time::Epoch;
use sat_units::typed::{Angle, Qty, Time};
use serde::{Deserialize, Serialize};

/// 軌道上の時系列に共通の既定刻み [s]。
pub const DEFAULT_STEP_S: f64 = 30.0;

/// 軌道の標本の取り方（軌道・期間・刻み・β 角）。時系列（[`TimelineSpec`]）と地上局のパスの探索で共有する。
/// **軌道・期間・刻み・β をまとめて受けるのはこれだけ**（解析ごとに別々の欄で受けない）。
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct OrbitSampling {
    pub orbit: OrbitSpec,
    /// 期間（軌道の元期から）
    pub span: Qty<Time>,
    /// 刻み
    pub step: Qty<Time>,
    /// 始め（軌道の元期）での β 角。指定すると、その β になるように軌道の RAAN を置き換える
    /// （[`crate::orbit::raan_for_beta`]。昇交点が昼側の解）。`None` なら軌道の RAAN のまま。
    #[serde(default)]
    pub beta: Option<Qty<Angle>>,
}

impl OrbitSampling {
    /// β の指定なしで作る。
    pub fn new(orbit: OrbitSpec, span: Qty<Time>, step: Qty<Time>) -> Self {
        Self {
            orbit,
            span,
            step,
            beta: None,
        }
    }

    /// 実際に伝搬に使う軌道（[`OrbitSampling::beta`] を指定したときは RAAN を置き換えたもの）。
    /// パスの検出など、同じ軌道を別の関数に渡すときはこれを使う。**β の当て込みはここだけ。**
    pub fn resolved_orbit(&self) -> Result<OrbitSpec, AstroError> {
        let mut orbit = self.orbit.clone();
        if let Some(beta) = self.beta {
            let raan = raan_for_beta(&orbit, orbit.epoch, beta.value())?;
            orbit.elements.raan = Qty::from_raw(raan);
        }
        Ok(orbit)
    }

    /// 時刻列（[`sample_times`]。期間と刻みを検査する）。
    pub fn times(&self) -> Result<Vec<f64>, AstroError> {
        sample_times(self.span.value(), self.step.value())
    }

    /// 標本の時刻ごとの位置・速度（β を当てた軌道で伝搬する）。姿勢の要らない集計（地磁気の平均など）はこれを使う。
    /// 時系列（[`build`]）もこれから作る。
    pub fn states(&self) -> Result<Vec<OrbitPoint>, AstroError> {
        let orbit = self.resolved_orbit()?;
        let times = self.times()?;
        let states = orbit.states_at(&times)?;
        Ok(times
            .into_iter()
            .zip(states)
            .map(|(t_s, state)| OrbitPoint {
                t_s,
                epoch: orbit.epoch.plus_seconds(t_s),
                state,
            })
            .collect())
    }
}

/// 軌道の標本の 1 点（[`OrbitSampling::states`]）。
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct OrbitPoint {
    /// 元期からの経過 [s]
    pub t_s: f64,
    pub epoch: Epoch,
    /// 位置・速度（中心天体からの相対。向きは ECI）
    pub state: OrbitState,
}

/// 時系列の作り方: 軌道の標本の取り方（[`OrbitSampling`]。JSON では同じ階層に並べる）と姿勢則・影のモデル。
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct TimelineSpec {
    #[serde(flatten)]
    pub sampling: OrbitSampling,
    pub attitude: AttitudeLaw,
    #[serde(default)]
    pub shadow: ShadowModel,
}

impl TimelineSpec {
    /// 既定の設定（円錐影、β の指定なし）で作る。フィールドが増えても呼び出し側を変えずに済むよう、
    /// 構造体リテラルよりこれを使う。
    pub fn new(orbit: OrbitSpec, attitude: AttitudeLaw, span: Qty<Time>, step: Qty<Time>) -> Self {
        Self::from_sampling(OrbitSampling::new(orbit, span, step), attitude)
    }

    /// 軌道の標本の取り方と姿勢則から作る（円錐影）。
    pub fn from_sampling(sampling: OrbitSampling, attitude: AttitudeLaw) -> Self {
        Self {
            sampling,
            attitude,
            shadow: ShadowModel::default(),
        }
    }

    /// 実際に伝搬に使う軌道（[`OrbitSampling::resolved_orbit`]）。
    pub fn resolved_orbit(&self) -> Result<OrbitSpec, AstroError> {
        self.sampling.resolved_orbit()
    }

    /// 軌道（β を当てる前）。中心天体と周期はこれから取る（β は RAAN だけを変える）。
    pub fn orbit(&self) -> &OrbitSpec {
        &self.sampling.orbit
    }
}

/// 時系列の 1 点。
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct TimelineSample {
    /// 元期からの経過 [s]
    pub t_s: f64,
    pub epoch: Epoch,
    /// 位置 [m]（軌道の中心天体からの相対。向きは ECI）
    pub r_eci: Vec3,
    /// 速度 [m/s]
    pub v_eci: Vec3,
    /// 太陽方向（単位ベクトル）
    pub sun_dir_eci: Vec3,
    /// 太陽までの距離 [m]
    pub sun_distance_m: f64,
    /// 日照率（1 = 全日照）
    pub sunlit: f64,
    /// 機体 → ECI
    pub q_body_to_eci: Quat,
}

impl TimelineSample {
    /// ECI のベクトルを機体座標系に直す。
    pub fn to_body(&self, v_eci: Vec3) -> Vec3 {
        self.q_body_to_eci.rotate_inverse(v_eci)
    }
    /// 太陽方向（機体座標系）。
    pub fn sun_dir_body(&self) -> Vec3 {
        self.to_body(self.sun_dir_eci)
    }
    /// 地心方向（機体座標系）。
    pub fn nadir_body(&self) -> Vec3 {
        self.to_body((-self.r_eci).normalized().unwrap_or(-Vec3::X))
    }
    /// 機体座標系のベクトルを ECI に直す。
    pub fn to_eci(&self, v_body: Vec3) -> Vec3 {
        self.q_body_to_eci.rotate(v_body)
    }
    /// 速度方向（機体座標系、単位ベクトル）。
    pub fn velocity_dir_body(&self) -> Vec3 {
        self.to_body(self.v_eci.normalized().unwrap_or(Vec3::X))
    }
    /// 位置・速度。
    pub fn state(&self) -> OrbitState {
        OrbitState {
            r_eci: self.r_eci,
            v_eci: self.v_eci,
        }
    }
}

/// 時系列を作る。
pub fn build(spec: &TimelineSpec) -> Result<Vec<TimelineSample>, AstroError> {
    Ok(spec
        .sampling
        .states()?
        .iter()
        .map(|p| sample(spec, p))
        .collect())
}

/// 1 点の標本を作る。
fn sample(spec: &TimelineSpec, point: &OrbitPoint) -> TimelineSample {
    let OrbitPoint { t_s, epoch, state } = *point;
    let state = &state;
    let central = spec.orbit().central;
    let r_sun = central.relative_position_m(Body::Sun, epoch);
    let to_sun = r_sun - state.r_eci;
    let sunlit = central_shadow_factor(central, state.r_eci, r_sun, spec.shadow);
    let sun_distance_m = to_sun.norm();
    let sun_dir_eci = to_sun.normalized().unwrap_or(Vec3::X);
    TimelineSample {
        t_s,
        epoch,
        r_eci: state.r_eci,
        v_eci: state.v_eci,
        sun_dir_eci,
        sun_distance_m,
        sunlit,
        q_body_to_eci: body_to_eci(&spec.attitude, state, sun_dir_eci),
    }
}

/// 標本の上限（これを超える刻みはエラーにする）。
pub const MAX_SAMPLES: usize = 5_000_000;

/// 時刻列 `0, step, 2·step, …, span`（最後は端数でも `span` ちょうどを含める）。
pub fn sample_times(span_s: f64, step_s: f64) -> Result<Vec<f64>, AstroError> {
    if !(step_s > 0.0 && step_s.is_finite()) {
        return Err(AstroError::InvalidInput(format!(
            "Step must be a positive finite value ({step_s} s)"
        )));
    }
    if !(span_s >= 0.0 && span_s.is_finite()) {
        return Err(AstroError::InvalidInput(format!(
            "Span must be a finite value of 0 or more ({span_s} s)"
        )));
    }
    if span_s / step_s + 2.0 > MAX_SAMPLES as f64 {
        return Err(AstroError::InvalidInput(format!(
            "Too many samples (span {span_s} s / step {step_s} s)"
        )));
    }
    Ok(crate::ode::step_offsets(span_s, step_s))
}

/// 時系列の中の区間（元期からの経過）。日陰・センサの禁止角への侵入・可視などに使う。
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct Window {
    pub start: Qty<Time>,
    pub end: Qty<Time>,
    pub duration: Qty<Time>,
}

/// `flags[i]` が真の連続区間。`t` は昇順 [s]。端はサンプル時刻（補間しない）。長さが違えば短い方に合わせる。
/// **区間の抽出はここだけ。**
pub fn windows(t: &[f64], flags: &[bool]) -> Vec<Window> {
    let n = t.len().min(flags.len());
    let mut out = Vec::new();
    let mut start: Option<f64> = None;
    for i in 0..n {
        match (flags[i], start) {
            (true, None) => start = Some(t[i]),
            (false, Some(s)) => {
                out.push(window(s, t[i - 1]));
                start = None;
            }
            _ => {}
        }
    }
    if let (Some(s), true) = (start, n > 0) {
        out.push(window(s, t[n - 1]));
    }
    out
}

fn window(start_s: f64, end_s: f64) -> Window {
    Window {
        start: Qty::from_raw(start_s),
        end: Qty::from_raw(end_s),
        duration: Qty::from_raw(end_s - start_s),
    }
}

/// 日陰の区間（日照率が `threshold` を下回る標本の連続区間）。端は標本の時刻（[`windows`] と同じ）。
pub fn shadow_windows(samples: &[TimelineSample], threshold: f64) -> Vec<Window> {
    let t: Vec<f64> = samples.iter().map(|s| s.t_s).collect();
    let flags: Vec<bool> = samples.iter().map(|s| s.sunlit < threshold).collect();
    windows(&t, &flags)
}

/// 日照率の時間平均（台形則）。標本が 1 つ以下ならその値（空なら 1）。
pub fn mean_sunlit(samples: &[TimelineSample]) -> f64 {
    if samples.is_empty() {
        return 1.0;
    }
    let t: Vec<f64> = samples.iter().map(|s| s.t_s).collect();
    let v: Vec<f64> = samples.iter().map(|s| s.sunlit).collect();
    time_mean(&t, &v)
}

/// 時間の重みを付けた平均（台形則）。`t` は昇順 [s]。**時系列の時間平均はここだけ。**
///
/// 長さが違えば短い方に合わせる。空なら NaN、1 点か期間 0 なら最初の値。
pub fn time_mean(t: &[f64], v: &[f64]) -> f64 {
    let n = t.len().min(v.len());
    match n {
        0 => f64::NAN,
        1 => v[0],
        _ => {
            let span = t[n - 1] - t[0];
            if span <= 0.0 {
                return v[0];
            }
            cumulative_trapezoid(&t[..n], &v[..n])[n - 1] / span
        }
    }
}

/// 台形則の累積積分 `∫_{t0}^{t_i} v dt`（`t` と同じ長さ、先頭は 0）。電池の充放電量などに使う。
///
/// 長さが違えば短い方に合わせる。
pub fn cumulative_trapezoid(t: &[f64], v: &[f64]) -> Vec<f64> {
    let n = t.len().min(v.len());
    let mut out = Vec::with_capacity(n);
    let mut acc = 0.0;
    for i in 0..n {
        if i > 0 {
            acc += 0.5 * (v[i - 1] + v[i]) * (t[i] - t[i - 1]);
        }
        out.push(acc);
    }
    out
}

#[cfg(test)]
mod tests;
