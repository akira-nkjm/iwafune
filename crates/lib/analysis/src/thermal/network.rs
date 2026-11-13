//! 集中定数の熱ネットワーク（節点・伝導・放射・外面）と、その解法。
//! **熱ネットワークの解法はここだけ。** `thermal.steady`（Newton 法の定常解）と `thermal.transient`（微分を `sat_astro::ode` で積分）が使う。
//!
//! 節点 i の熱収支: `C_i dT_i/dt = Q_i + Σ_j G_ij (T_j − T_i) + Σ_j σ R_ij (T_j⁴ − T_i⁴) − Σ_s ε_s σ A_s (T_i⁴ − T_space⁴)`。
//! Q_i は外部入熱（吸収した太陽・アルベド・地球赤外）と内部発熱の和。`fixed_temperature` を持つ節点は境界（温度一定）。

use crate::spec::AnalysisError;
use sat_astro::constants::SIGMA_SB;
use sat_astro::math::Vec3;
use sat_units::typed::{
    Area, Mass, Power, Qty, SpecificHeatCapacity, Temperature, ThermalConductance,
};
use serde::{Deserialize, Serialize};

mod mode_loads;
mod mounts;
mod newton;
pub use mode_loads::{ThermalModeLoads, ThermalModePart};
pub use mounts::ThermalMounts;
pub use newton::INITIAL_GUESS_K;

/// 温度の範囲（動作範囲など）。
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct ThermalTempRange {
    pub min: Qty<Temperature>,
    pub max: Qty<Temperature>,
}

/// 節点に含まれる部品。熱容量と動作温度範囲は、部品からここで節点にまとめる。
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
    /// 動作温度範囲（節点の範囲は部品の範囲の共通部分）
    #[serde(default)]
    pub operating: Option<ThermalTempRange>,
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

    /// 動作温度範囲 = 範囲を持つ部品の範囲の共通部分（無ければ `None`。重ならなければ min > max になる）。
    pub fn operating(&self) -> Option<ThermalTempRange> {
        self.parts
            .iter()
            .filter_map(|p| p.operating)
            .reduce(|a, b| ThermalTempRange {
                min: Qty::from_raw(a.min.value().max(b.min.value())),
                max: Qty::from_raw(a.max.value().min(b.max.value())),
            })
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

/// 宇宙を向く外面（入熱を受け、宇宙へ放射する）。1 節点に複数の面を付けてよい。
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct ThermalSurface {
    pub id: String,
    /// 面が属する節点
    pub node: String,
    pub area: Qty<Area>,
    /// 外向きの法線（機体座標系。正規化して使う）
    pub normal: Vec3,
    /// 枚数（実効面積 = 面積 × 枚数。省略で 1、0 なら数えない）
    #[serde(default = "crate::spec::default_count")]
    pub count: u32,
    /// 太陽光の吸収率 α
    pub absorptivity: f64,
    /// 赤外の放射率 ε
    pub emissivity: f64,
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

/// 節点への内部発熱の源（部品など）。発熱 = 消費電力 × `heat_fraction`（ここで計算する）。
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct ThermalLoad {
    /// 員数。power は単体値（省略時 1）。
    #[serde(default = "crate::spec::default_count")]
    pub count: u32,
    /// 熱を出す節点
    pub node: String,
    /// 1 台あたりの消費電力（モードの平均も単体値）
    pub power: Qty<Power>,
    /// 消費電力のうち熱になる割合（0〜1）。省略時 1（すべて熱になる）
    #[serde(default)]
    pub heat_fraction: Option<f64>,
}

impl ThermalLoad {
    /// 熱になる割合（省略時 1）。
    pub fn heat_fraction_or_all(&self) -> f64 {
        self.heat_fraction.unwrap_or(1.0)
    }
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
            return Err(invalid("熱の節点がありません".to_owned()));
        }
        let ids: Vec<String> = net.nodes.iter().map(|n| n.id.clone()).collect();
        for (i, id) in ids.iter().enumerate() {
            if ids[..i].contains(id) {
                return Err(invalid(format!("熱の節点 {id} が重複しています")));
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
                    "節点 {}: 部品の質量 × 員数 × 比熱の合計は正、固定温度は正の値にしてください",
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
        let ok = s.area.value() >= 0.0
            && (0.0..=1.0).contains(&s.absorptivity)
            && (0.0..=1.0).contains(&s.emissivity);
        let normal = s.normal.normalized();
        match (ok, normal) {
            (true, Some(normal)) => Ok(Surface {
                node: self.index(&s.node)?,
                area_m2: s.area.value() * f64::from(s.count),
                normal,
                alpha: s.absorptivity,
                eps: s.emissivity,
            }),
            _ => Err(invalid(format!(
                "外面 {}: 面積は 0 以上、α と ε は 0〜1、法線は 0 でないベクトルにしてください",
                s.id
            ))),
        }
    }

    /// 節点 ID の添字。
    pub fn index(&self, id: &str) -> Result<usize, AnalysisError> {
        self.ids
            .iter()
            .position(|x| x == id)
            .ok_or_else(|| invalid(format!("熱の節点 {id} がありません")))
    }

    pub fn len(&self) -> usize {
        self.ids.len()
    }

    /// 内部発熱（消費電力 × `heat_fraction`）を節点ごとの配列 [W] にする。
    pub fn loads(&self, loads: &[ThermalLoad]) -> Result<Vec<f64>, AnalysisError> {
        let mut q = vec![0.0; self.len()];
        for l in loads {
            let (p, f) = (l.power.value(), l.heat_fraction_or_all());
            if !(p.is_finite() && f.is_finite() && (0.0..=1.0).contains(&f)) {
                return Err(invalid(format!(
                    "節点 {} の発熱: 電力は有限、heat_fraction は 0〜1 にしてください",
                    l.node
                )));
            }
            q[self.index(&l.node)?] += p * f64::from(l.count) * f;
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
        Err(invalid(format!(
            "{a}–{b} の結合は 0 以上の値にしてください"
        )))
    }
}

#[cfg(test)]
pub(crate) mod tests;
