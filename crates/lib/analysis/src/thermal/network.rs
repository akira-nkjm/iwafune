//! 集中定数の熱ネットワーク（節点・伝導・放射・外面）と、その解法。
//! **熱ネットワークの解法はここだけ。** `thermal.steady`（Newton 法の定常解）と `thermal.transient`（微分を `sat_astro::ode` で積分）が使う。
//!
//! 節点 i の熱収支: `C_i dT_i/dt = Q_i + Σ_j G_ij (T_j − T_i) + Σ_j σ R_ij (T_j⁴ − T_i⁴) − Σ_s ε_s σ A_s (T_i⁴ − T_space⁴)`。
//! Q_i は外部入熱（吸収した太陽・アルベド・地球赤外）と内部発熱（`load`）の和。外面の α/ε は `surface` が表面材から求める。`fixed_temperature` を持つ節点は境界（温度一定）。

use crate::spec::AnalysisError;
use crate::thermal::range::ThermalRanges;
use sat_astro::constants::SIGMA_SB;
use sat_astro::math::Vec3;
use sat_units::typed::{Area, Mass, Qty, SpecificHeatCapacity, Temperature, ThermalConductance};
use serde::{Deserialize, Serialize};

mod load;
mod mode_loads;
mod mounts;
mod newton;
mod ranges;
mod surface;
pub use load::ThermalLoad;
pub use mode_loads::{ThermalModeLoads, ThermalModePart};
pub use mounts::ThermalMounts;
pub use newton::INITIAL_GUESS_K;
pub use surface::{ThermalCoating, ThermalSurface};

/// 節点に含まれる部品。熱容量と温度範囲は、部品からここで節点にまとめる。
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct ThermalNodePart {
    pub id: String,
    /// 1 台あたりの質量
    pub mass: Qty<Mass>,
    /// 員数（省略で 1、0 なら数えない）
    #[serde(default = "crate::spec::default_count")]
    pub count: u32,
    pub specific_heat: Qty<SpecificHeatCapacity>,
    /// 保管・動作・性能の温度範囲（欄は `storage`・`operating`・`performance` を部品の直下に書く。
    /// 節点の範囲は種類ごとに部品の範囲の共通部分。モードのあるケースは電源の状態で選ぶ（`ranges_in`））
    #[serde(default, flatten)]
    pub ranges: ThermalRanges,
    /// 冗長の方式（モードでの電源の状態を数える）
    #[serde(default)]
    pub redundancy: crate::modes::Redundancy,
    /// 種別の状態の語彙（warm の予備状態を選ぶ）
    #[serde(default)]
    pub states: Vec<String>,
}

/// 熱の節点。
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct ThermalNode {
    /// 節点の ID
    pub id: String,
    /// 節点に含まれる部品（熱容量 = Σ 質量 × 員数 × 比熱）
    #[serde(default)]
    pub parts: Vec<ThermalNodePart>,
    /// 温度一定の境界節点にするときの温度
    #[serde(default)]
    pub fixed_temperature: Option<Qty<Temperature>>,
}

impl ThermalNode {
    /// 熱容量 Σ 質量 × 員数 × 比熱 [J/K]。
    pub fn heat_capacity(&self) -> f64 {
        self.parts
            .iter()
            .map(|p| p.mass.value() * f64::from(p.count) * p.specific_heat.value())
            .sum()
    }

    /// 種類ごとの温度範囲 = 範囲を持つ部品の範囲の共通部分（モードによらない。無い種類は `None`。重ならなければ min > max
    /// になる）。モードのあるケースの判定は [`Self::ranges_in`]。
    pub fn ranges(&self) -> ThermalRanges {
        self.parts
            .iter()
            .map(|p| p.ranges)
            .reduce(ThermalRanges::intersect)
            .unwrap_or_default()
    }
}

/// 伝導の結合（対称）。
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct ThermalConductiveLink {
    pub a: String,
    pub b: String,
    pub conductance: Qty<ThermalConductance>,
}

/// 節点間の放射の結合（対称）。`exchange_area` は有効放射交換面積 ε_eff·A·F。
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct ThermalRadiativeLink {
    pub a: String,
    pub b: String,
    pub exchange_area: Qty<Area>,
}

/// 熱ネットワーク。
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct ThermalNetwork {
    /// 取付から作る熱結合（明示の conductive/radiative に追加）。
    #[serde(default)]
    pub mounts: Option<ThermalMounts>,
    pub nodes: Vec<ThermalNode>,
    #[serde(default)]
    pub conductive: Vec<ThermalConductiveLink>,
    #[serde(default)]
    pub radiative: Vec<ThermalRadiativeLink>,
    #[serde(default)]
    pub surfaces: Vec<ThermalSurface>,
}

/// 添字に直した外面。
#[derive(Debug, Clone, Copy)]
pub(crate) struct Surface {
    pub node: usize,
    pub area_m2: f64,
    pub normal: Vec3,
    pub alpha: f64,
    pub eps: f64,
}

/// 添字に直した熱ネットワーク（SI の f64）。
#[derive(Debug, Clone)]
pub(crate) struct Compiled {
    pub ids: Vec<String>,
    /// 熱容量 [J/K]
    pub capacity: Vec<f64>,
    pub fixed: Vec<Option<f64>>,
    /// (i, j, G [W/K])
    pub cond: Vec<(usize, usize, f64)>,
    /// (i, j, σ·R [W/K⁴])
    pub rad: Vec<(usize, usize, f64)>,
    pub surfaces: Vec<Surface>,
    /// 節点ごとの宇宙への放射係数 Σ ε σ A [W/K⁴]
    pub space: Vec<f64>,
}

fn invalid(msg: String) -> AnalysisError {
    AnalysisError::InvalidInput(msg)
}

impl Compiled {
    /// 入力を検査して添字に直す。
    pub fn new(net: &ThermalNetwork) -> Result<Self, AnalysisError> {
        if net.nodes.is_empty() {
            return Err(invalid("No thermal nodes".to_owned()));
        }
        let ids: Vec<String> = net.nodes.iter().map(|n| n.id.clone()).collect();
        for (i, id) in ids.iter().enumerate() {
            if ids[..i].contains(id) {
                return Err(invalid(format!("Thermal node {id} is duplicated")));
            }
        }
        let mut c = Self {
            capacity: Vec::with_capacity(ids.len()),
            fixed: Vec::with_capacity(ids.len()),
            cond: Vec::new(),
            rad: Vec::new(),
            surfaces: Vec::new(),
            space: vec![0.0; ids.len()],
            ids,
        };
        for n in &net.nodes {
            let cap = n.heat_capacity();
            let fixed = n.fixed_temperature.map(Qty::value);
            if !(cap > 0.0 || fixed.is_some()) || fixed.is_some_and(|t| t.is_nan() || t <= 0.0) {
                return Err(invalid(format!(
                    "Node {}: the sum of part mass × count × specific heat must be positive, and a fixed temperature must be positive",
                    n.id
                )));
            }
            c.capacity.push(cap);
            c.fixed.push(fixed);
        }
        let (mount_cond, mount_rad) = net
            .mounts
            .as_ref()
            .map(ThermalMounts::links)
            .unwrap_or_default();
        for l in net.conductive.iter().chain(&mount_cond) {
            let g = nonneg(l.conductance.value(), &l.a, &l.b)?;
            c.cond.push((c.index(&l.a)?, c.index(&l.b)?, g));
        }
        for l in net.radiative.iter().chain(&mount_rad) {
            let r = nonneg(l.exchange_area.value(), &l.a, &l.b)?;
            c.rad.push((c.index(&l.a)?, c.index(&l.b)?, SIGMA_SB * r));
        }
        for s in &net.surfaces {
            let surface = c.surface(s)?;
            c.space[surface.node] += surface.eps * SIGMA_SB * surface.area_m2;
            c.surfaces.push(surface);
        }
        Ok(c)
    }

    fn surface(&self, s: &ThermalSurface) -> Result<Surface, AnalysisError> {
        let (alpha, eps) = s.optics()?;
        match (s.area.value() >= 0.0, s.normal.normalized()) {
            (true, Some(normal)) => Ok(Surface {
                node: self.index(&s.node)?,
                area_m2: s.area.value() * f64::from(s.count),
                normal,
                alpha,
                eps,
            }),
            _ => Err(invalid(format!(
                "Surface {}: area must be 0 or more and the normal must be a non-zero vector",
                s.id
            ))),
        }
    }

    /// 節点 ID の添字。
    pub fn index(&self, id: &str) -> Result<usize, AnalysisError> {
        self.ids
            .iter()
            .position(|x| x == id)
            .ok_or_else(|| invalid(format!("Thermal node {id} does not exist")))
    }

    pub fn len(&self) -> usize {
        self.ids.len()
    }

    /// 内部発熱（[`ThermalLoad::dissipation`]）を節点ごとの配列 [W] にする。
    pub fn loads(&self, loads: &[ThermalLoad]) -> Result<Vec<f64>, AnalysisError> {
        let mut q = vec![0.0; self.len()];
        for l in loads {
            q[self.index(&l.node)?] += l.dissipation()?.value();
        }
        Ok(q)
    }

    /// 節点ごとの正味の熱流入 [W]。`q` は外部入熱と内部発熱の和。
    pub fn net_heat(&self, t: &[f64], q: &[f64], space_k: f64) -> Vec<f64> {
        let t_space4 = space_k.powi(4);
        let mut f: Vec<f64> = (0..self.len())
            .map(|i| q[i] - self.space[i] * (t[i].powi(4) - t_space4))
            .collect();
        for &(i, j, g) in &self.cond {
            let flow = g * (t[j] - t[i]);
            f[i] += flow;
            f[j] -= flow;
        }
        for &(i, j, sr) in &self.rad {
            let flow = sr * (t[j].powi(4) - t[i].powi(4));
            f[i] += flow;
            f[j] -= flow;
        }
        f
    }

    /// 温度の時間微分 [K/s]（境界節点は 0）。`sat_astro::ode` に渡す。
    pub fn derivative(&self, t: &[f64], q: &[f64], space_k: f64) -> Vec<f64> {
        let f = self.net_heat(t, q, space_k);
        (0..self.len())
            .map(|i| match self.fixed[i] {
                Some(_) => 0.0,
                None => f[i] / self.capacity[i],
            })
            .collect()
    }

    /// 陽的 RK4 が安定に回る刻みの目安 [s]（温度 `t_ref_k` での線形化の固有値の上界から）。
    pub fn stable_step_s(&self, t_ref_k: f64) -> f64 {
        let mut k = self
            .space
            .iter()
            .map(|s| 4.0 * s * t_ref_k.powi(3))
            .collect::<Vec<_>>();
        for &(i, j, g) in &self.cond {
            k[i] += g;
            k[j] += g;
        }
        for &(i, j, sr) in &self.rad {
            let h = 4.0 * sr * t_ref_k.powi(3);
            k[i] += h;
            k[j] += h;
        }
        (0..self.len())
            .filter(|&i| self.fixed[i].is_none() && k[i] > 0.0)
            .map(|i| self.capacity[i] / k[i])
            .fold(f64::INFINITY, f64::min)
    }
}

fn nonneg(v: f64, a: &str, b: &str) -> Result<f64, AnalysisError> {
    if v.is_finite() && v >= 0.0 {
        Ok(v)
    } else {
        Err(invalid(format!("Coupling {a}–{b} must be 0 or more")))
    }
}

#[cfg(test)]
pub(crate) mod tests;
