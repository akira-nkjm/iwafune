//! 推力を入れた数値伝搬（Cowell 法に推力の項と質量を足す）。[`crate::force`] の加速度と
//! [`crate::force::thrust`] の推力を [`crate::ode::Dop853`] で積分する。
//!
//! - 状態は位置・速度・質量・積んだ ΔV の 8 成分。質量は推薬の流量 `F / v_e`（[`crate::rocket::mass_flow_kg_s`]）で減る。
//! - 噴射の有無と面外の向きの符号は、制御の刻み（`control_step_s`）の始めに決めて刻みの間は保つ（sample-and-hold。
//!   刻みの中の微分は滑らかなので適応刻みが崩れない）。日照の判定は刻みの始めの円錐影（[`crate::shadow`]）。
//! - デューティ `d` は「噴いた時間 ≤ d × 噴いてよかった時間」になるように刻みごとに噴くかを決める。
//! - 目標は平均の長半径（[`super::mean_semi_major_axis_m`]。重力が点質量でなければ J2 の短周期項を引く）と傾斜角（接触）。
//!   ΔV の大きい方（長半径の変化 `|v₀ − v₁|` と面の変化 `v₀ π |Δi| / 2`）が目標を越えた刻みの中を線形に按分し、
//!   その長さで積分し直して終える。上限の時間までに届かなければ `reached = false`。

use super::mean::mean_semi_major_axis_m;
use super::numerical::NUMERICAL_INTEGRATOR;
use super::{OrbitState, mean_motion_rad_s, osculating_elements};
use crate::AstroError;
use crate::ephem::{Body, CentralBody};
use crate::force::thrust::{
    ThrustSteering, cos_argument_of_latitude, edelbaum_initial_yaw_rad, edelbaum_yaw_rad,
    tangential_direction, thrust_acceleration_eci,
};
use crate::force::{ForceConfig, GravityModel, acceleration_eci_about, validate_about};
use crate::math::Vec3;
use crate::rocket::mass_flow_kg_s;
use crate::shadow::{ShadowModel, central_shadow_factor};
use crate::time::Epoch;
use core::f64::consts::{FRAC_PI_2, TAU};

/// 噴射の仕方。
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ThrustArc {
    /// 推力 [N]
    pub thrust_n: f64,
    /// 有効排気速度 [m/s]
    pub exhaust_velocity_m_s: f64,
    pub steering: ThrustSteering,
    /// 噴射してよい日照率の下限（`None` なら日陰でも噴く）
    pub min_sunlit_factor: Option<f64>,
    /// デューティ（噴いてよい時間のうち噴く割合、0 < d ≤ 1）
    pub duty: f64,
    /// 制御の刻み [s]
    pub control_step_s: f64,
    /// 伝搬の上限の時間 [s]
    pub max_duration_s: f64,
}

/// 目標の軌道（平均の長半径と傾斜角）。
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PoweredTarget {
    pub semi_major_axis_m: f64,
    pub inclination_rad: f64,
}

/// 時系列の 1 点（ほぼ 1 周ごと）。
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PoweredSample {
    /// 噴射の始めからの時間 [s]
    pub t_s: f64,
    pub state: OrbitState,
    /// 平均の長半径 [m]
    pub mean_semi_major_axis_m: f64,
    pub eccentricity: f64,
    pub inclination_rad: f64,
    pub mass_kg: f64,
    /// 積んだ ΔV [m/s]
    pub delta_v_m_s: f64,
    /// 前の点からの間に噴いた時間の割合
    pub firing_fraction: f64,
}

/// 推力を入れた伝搬の結果。
#[derive(Debug, Clone, PartialEq)]
pub struct PoweredArc {
    /// 始め・ほぼ 1 周ごと・終わりの点
    pub samples: Vec<PoweredSample>,
    pub end_state: OrbitState,
    /// 始めから終わりまでの時間 [s]
    pub duration_s: f64,
    /// 噴いた時間の合計 [s]
    pub firing_time_s: f64,
    /// 積んだ ΔV [m/s]（`∫ F/m dt`）
    pub delta_v_m_s: f64,
    /// 使った推薬 [kg]
    pub propellant_kg: f64,
    pub end_mass_kg: f64,
    /// 目標に届いたか
    pub reached: bool,
}

type Y = [f64; 8];

fn pack(s: &OrbitState, mass: f64, dv: f64) -> Y {
    let (r, v) = (s.r_eci, s.v_eci);
    [r.x, r.y, r.z, v.x, v.y, v.z, mass, dv]
}

fn unpack(y: &Y) -> OrbitState {
    OrbitState {
        r_eci: Vec3::new(y[0], y[1], y[2]),
        v_eci: Vec3::new(y[3], y[4], y[5]),
    }
}

/// 刻みの間に保つ制御。
#[derive(Clone, Copy)]
struct Control {
    fire: bool,
    normal_sign: f64,
}

/// 伝搬の文脈（変わらない値）。
struct Powered<'a> {
    central: CentralBody,
    forces: &'a ForceConfig,
    epoch: Epoch,
    arc: &'a ThrustArc,
    v0: f64,
    yaw0: f64,
    inertial_dir: Vec3,
    with_j2: bool,
}

/// どの量で目標に着いたかを見るか。
#[derive(Clone, Copy)]
enum Goal {
    SemiMajorAxis(f64),
    Inclination(f64),
}

impl Powered<'_> {
    fn step(&self, ctl: Control, t: f64, y: &Y, h: f64) -> Result<Y, AstroError> {
        if h <= 0.0 {
            return Ok(*y);
        }
        let f = |s: f64, y: &Y| {
            let st = unpack(y);
            let g =
                acceleration_eci_about(self.central, self.forces, self.epoch.plus_seconds(s), &st);
            let (a, mdot, dv) = if ctl.fire {
                let dir = match self.arc.steering {
                    ThrustSteering::Tangential => {
                        let yaw = edelbaum_yaw_rad(self.v0, self.yaw0, y[7]);
                        tangential_direction(&st, yaw, ctl.normal_sign)
                    }
                    ThrustSteering::Inertial => self.inertial_dir,
                };
                let a = thrust_acceleration_eci(dir, self.arc.thrust_n, y[6]);
                let mdot = mass_flow_kg_s(self.arc.thrust_n, self.arc.exhaust_velocity_m_s);
                (a, mdot, a.norm())
            } else {
                (Vec3::ZERO, 0.0, 0.0)
            };
            let acc = g + a;
            [y[3], y[4], y[5], acc.x, acc.y, acc.z, -mdot, dv]
        };
        NUMERICAL_INTEGRATOR
            .integrate_at(f, t, *y, &[t + h])?
            .pop()
            .ok_or_else(|| AstroError::InvalidInput("推力を入れた伝搬の結果が空です".into()))
    }

    fn value(&self, goal: Goal, y: &Y) -> Result<f64, AstroError> {
        let st = unpack(y);
        match goal {
            Goal::SemiMajorAxis(_) => mean_semi_major_axis_m(self.central, &st, self.with_j2),
            Goal::Inclination(_) => Ok(osculating_elements(&st, self.central.gm())?.i_rad),
        }
    }

    fn lit(&self, t: f64, y: &Y) -> bool {
        let Some(min) = self.arc.min_sunlit_factor else {
            return true;
        };
        let epoch = self.epoch.plus_seconds(t);
        let sun = self.central.relative_position_m(Body::Sun, epoch);
        central_shadow_factor(self.central, unpack(y).r_eci, sun, ShadowModel::Conical) >= min
    }

    fn sample(&self, t: f64, y: &Y, firing_fraction: f64) -> Result<PoweredSample, AstroError> {
        let state = unpack(y);
        let el = osculating_elements(&state, self.central.gm())?;
        Ok(PoweredSample {
            t_s: t,
            state,
            mean_semi_major_axis_m: mean_semi_major_axis_m(self.central, &state, self.with_j2)?,
            eccentricity: el.e,
            inclination_rad: el.i_rad,
            mass_kg: y[6],
            delta_v_m_s: y[7],
            firing_fraction,
        })
    }
}

fn check(arc: &ThrustArc, mass0: f64) -> Result<(), AstroError> {
    let pos = |x: f64| x.is_finite() && x > 0.0;
    let ok = pos(arc.thrust_n)
        && pos(arc.exhaust_velocity_m_s)
        && pos(arc.control_step_s)
        && pos(arc.max_duration_s)
        && pos(mass0)
        && arc.duty > 0.0
        && arc.duty <= 1.0;
    if ok {
        Ok(())
    } else {
        Err(AstroError::InvalidInput(
            "推力・排気速度・質量・制御の刻み・上限の時間は正、デューティは 0 より大きく 1 以下にしてください".into(),
        ))
    }
}

/// 中心天体 `central` のまわりを、元期 `epoch` の状態 `x0`・質量 `mass0` [kg] から、目標 `target` に着くまで噴射して伝搬する。
pub fn propagate_powered(
    central: CentralBody,
    forces: &ForceConfig,
    epoch: Epoch,
    x0: OrbitState,
    mass0: f64,
    arc: &ThrustArc,
    target: &PoweredTarget,
) -> Result<PoweredArc, AstroError> {
    validate_about(central, forces)?;
    check(arc, mass0)?;
    let mu = central.gm();
    let with_j2 = forces.gravity != GravityModel::PointMass;
    let a0 = mean_semi_major_axis_m(central, &x0, with_j2)?;
    let i0 = osculating_elements(&x0, mu)?.i_rad;
    let (v0, v1) = ((mu / a0).sqrt(), (mu / target.semi_major_axis_m).sqrt());
    let di = target.inclination_rad - i0;
    let yaw0 = edelbaum_initial_yaw_rad(v0, v1, di);
    let along = x0.v_eci.normalized().unwrap_or(Vec3::X);
    let p = Powered {
        central,
        forces,
        epoch,
        arc,
        v0,
        yaw0,
        inertial_dir: along * yaw0.cos().signum(),
        with_j2,
    };
    let goal = if (v0 - v1).abs() >= v0 * FRAC_PI_2 * di.abs() {
        Goal::SemiMajorAxis(target.semi_major_axis_m)
    } else {
        Goal::Inclination(target.inclination_rad)
    };
    run(&p, goal, x0, mass0, di.signum())
}

/// 伝搬の途中の値（状態・時刻・噴いた時間・時系列）。
struct Progress {
    y: Y,
    t: f64,
    fired: f64,
    eligible: f64,
    window_fired: f64,
    samples: Vec<PoweredSample>,
}

impl Progress {
    /// 時系列に今の点を足す（前の点からの噴いた割合と一緒に）。
    fn record(&mut self, p: &Powered<'_>) -> Result<(), AstroError> {
        let span = self.t - self.samples.last().map_or(0.0, |s| s.t_s);
        let frac = if span > 0.0 {
            self.window_fired / span
        } else {
            0.0
        };
        self.samples.push(p.sample(self.t, &self.y, frac)?);
        self.window_fired = 0.0;
        Ok(())
    }

    /// 刻みの始めに、噴くか（日照・デューティ）と面外の向きの符号を決める。
    fn control(&mut self, p: &Powered<'_>, di_sign: f64) -> Control {
        let dt = p.arc.control_step_s;
        let lit = p.lit(self.t, &self.y);
        if lit {
            self.eligible += dt;
        }
        let fire = lit && self.fired + dt <= p.arc.duty * self.eligible + 0.5 * dt;
        let normal_sign = cos_argument_of_latitude(&unpack(&self.y)).signum() * di_sign;
        Control { fire, normal_sign }
    }

    /// 長さ `h` だけ進んだ（噴いていれば噴いた時間に足す）。
    fn advance(&mut self, y: Y, h: f64, fire: bool) -> Result<(), AstroError> {
        self.y = y;
        self.t += h;
        if fire {
            self.fired += h;
            self.window_fired += h;
        }
        if self.y[6] <= 0.0 {
            return Err(AstroError::InvalidInput(
                "推薬を使い切って質量が 0 以下になりました".into(),
            ));
        }
        Ok(())
    }
}

/// 制御の刻みごとに進め、目標を越えた刻みで按分して終える。
fn run(
    p: &Powered<'_>,
    goal: Goal,
    x0: OrbitState,
    mass0: f64,
    di_sign: f64,
) -> Result<PoweredArc, AstroError> {
    let dt = p.arc.control_step_s;
    let goal_value = match goal {
        Goal::SemiMajorAxis(v) | Goal::Inclination(v) => v,
    };
    let y = pack(&x0, mass0, 0.0);
    let mut value = p.value(goal, &y)?;
    let rising = goal_value > value;
    let period = TAU / mean_motion_rad_s(p.central.gm(), x0.r_eci.norm());
    let every = ((period / dt).round() as usize).max(1);
    let max_steps = (p.arc.max_duration_s / dt).ceil() as usize;
    let first = p.sample(0.0, &y, 0.0)?;
    let mut s = Progress {
        y,
        t: 0.0,
        fired: 0.0,
        eligible: 0.0,
        window_fired: 0.0,
        samples: vec![first],
    };
    let mut reached = (goal_value - value).abs() <= f64::EPSILON * goal_value.abs();
    let mut k = 0;
    while !reached && k < max_steps {
        let ctl = s.control(p, di_sign);
        let next = p.step(ctl, s.t, &s.y, dt)?;
        let next_value = p.value(goal, &next)?;
        reached = if rising {
            next_value >= goal_value
        } else {
            next_value <= goal_value
        };
        if reached {
            let w = ((goal_value - value) / (next_value - value)).clamp(0.0, 1.0);
            let y = p.step(ctl, s.t, &s.y, w * dt)?;
            s.advance(y, w * dt, ctl.fire)?;
        } else {
            value = next_value;
            s.advance(next, dt, ctl.fire)?;
        }
        k += 1;
        if reached || k % every == 0 {
            s.record(p)?;
        }
    }
    if s.samples.last().is_some_and(|x| x.t_s < s.t) {
        s.record(p)?;
    }
    Ok(PoweredArc {
        end_state: unpack(&s.y),
        duration_s: s.t,
        firing_time_s: s.fired,
        delta_v_m_s: s.y[7],
        propellant_kg: mass0 - s.y[6],
        end_mass_kg: s.y[6],
        reached,
        samples: s.samples,
    })
}

#[cfg(test)]
mod tests;
