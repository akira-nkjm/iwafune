//! 集中定数の熱ネットワーク（節点・伝導・放射・外面）と、その解法。
//! **熱ネットワークの解法はここだけ。** `thermal.steady`（Newton 法の定常解）と `thermal.transient`（微分を `sat_astro::ode` で積分）が使う。
//!
//! 節点 i の熱収支: `C_i dT_i/dt = Q_i + Σ_j G_ij (T_j − T_i) + Σ_j σ R_ij (T_j⁴ − T_i⁴) − Σ_s ε_s σ A_s (T_i⁴ − T_space⁴)`。
//! Q_i は外部入熱（吸収した太陽・アルベド・地球赤外）と内部発熱（`load`）の和。外面の α/ε は `surface` が表面材から求める。`fixed_temperature` を持つ節点は境界（温度一定）。

use crate::spec::AnalysisError;
use crate::structure::external_geometry::{Footprint, LitSurfaces, ShadowingInput};
use crate::thermal::enclosure::ThermalEnclosure;
use crate::thermal::range::{ThermalRanges, ThermalRequiredRange};
use sat_astro::constants::SIGMA_SB;
use sat_astro::math::Vec3;
use sat_units::typed::{Area, Mass, Qty, SpecificHeatCapacity, Temperature, ThermalConductance};
use serde::{Deserialize, Serialize};

mod couplings;
mod heater;
mod heater_steady;
mod load;
mod mode_loads;
mod mount_contact;
mod mounts;
mod newton;
mod ranges;
mod subdivide;
mod surface;
mod view_factors;
pub use crate::thermal::cell::ThermalCell;
pub use couplings::{
    ThermalConductanceUsed, ThermalCouplingSource, ThermalCouplings, ThermalEnclosureFace,
    ThermalEnclosureUsed, ThermalExchangeUsed,
};
pub(crate) use heater::{HeaterSpec, enabled_flags};
pub use heater::{ThermalHeater, ThermalHeaterState};
pub(crate) use heater_steady::HeaterSteady;
pub use load::ThermalLoad;
pub use mode_loads::{ThermalModeLoads, ThermalModePart};
pub use mount_contact::{
    ThermalContact, ThermalContactFallback, ThermalContactFallbackReason, ThermalContactPair,
    ThermalContactUsed, ThermalMountBoxes,
};
pub use mounts::{MountConductance, MountLinks, ThermalMounts};
pub use newton::INITIAL_GUESS_K;
pub use subdivide::{MAX_CELLS, ThermalPlate, ThermalPlateAttachment, ThermalPlateUsed};
pub use surface::{ThermalCoating, ThermalSurface};
pub use view_factors::{
    ThermalViewFactorNode, ThermalViewFactorPair, ThermalViewFactors, ThermalViewFactorsUsed,
    gray_pair_exchange,
};

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
    /// 設計の既定の 1 部品 1 節点か（偽は設計が複数の部品をまとめた節点）。表示の区別だけで、計算には使わない
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub per_part: bool,
    /// 温度一定の境界節点にするときの温度
    #[serde(default)]
    pub fixed_temperature: Option<Qty<Temperature>>,
    /// 要求が部品の範囲に重ねる限界（種類の綴り `storage`・`operating`・`performance` → 下限と上限。部品の範囲を厳しくするだけ）
    #[serde(default, skip_serializing_if = "std::collections::BTreeMap::is_empty")]
    pub required: std::collections::BTreeMap<String, ThermalRequiredRange>,
    /// 板の升の節点のとき、どの板のどの升か（入力の節点には書かない。解析が板を分けて作る）
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub cell: Option<ThermalCell>,
}

impl ThermalNode {
    /// 熱容量 Σ 質量 × 員数 × 比熱 [J/K]。
    pub fn heat_capacity(&self) -> f64 {
        self.parts
            .iter()
            .map(|p| p.mass.value() * f64::from(p.count) * p.specific_heat.value())
            .sum()
    }

    /// 熱容量が無い節点か（熱容量が正でなく、温度一定の境界節点でもない）。解析はこの節点を入力の誤りとして断る。
    /// **「この節点には熱容量が無い」の判定はここだけ**（解析の入力検査と、設計の検査の警告が同じ判定を使う）。
    pub fn lacks_heat_capacity(&self) -> bool {
        let capacity = self.heat_capacity();
        self.fixed_temperature.is_none() && (capacity.is_nan() || capacity <= 0.0)
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
    /// 取付から作る既定の熱結合（明示の `conductive`・`radiative` と外殻の放射が、対ごとに置き換える。優先順位は `couplings`）。
    #[serde(default)]
    pub mounts: Option<ThermalMounts>,
    pub nodes: Vec<ThermalNode>,
    /// 明示の伝導（その節点対の取付の値を置き換える。0 で切り離す）
    #[serde(default)]
    pub conductive: Vec<ThermalConductiveLink>,
    /// 明示の放射（その節点対の既定値と外殻の値を置き換える）
    #[serde(default)]
    pub radiative: Vec<ThermalRadiativeLink>,
    /// 箱型の外殻の内面の放射（設計が求めたときだけ）。外面の法線と面積から 6 面の節点を決める
    #[serde(default)]
    pub enclosure: Option<ThermalEnclosure>,
    /// 内部の部品と構造の殻の内側の放射を形態係数から出す設定（設計が求めたときだけ。箱と放射率）。優先順位は `couplings`
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub view_factors: Option<ThermalViewFactors>,
    #[serde(default)]
    pub surfaces: Vec<ThermalSurface>,
    /// 部品の箱。あれば、外面の直達日射に、その面の日照割合（ほかの部品の影を引いた割合）を掛ける
    /// （アルベドと惑星の赤外は広がった光源なので影を数えない）。省略時は影を数えない
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub shadowing: Option<ShadowingInput>,
    /// 升に分ける板（`subdivide`）。書いた部品の節点は升の節点に置き換わる
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub plates: Vec<ThermalPlate>,
}

/// 添字に直した外面。
#[derive(Debug, Clone, Copy)]
pub(crate) struct Surface {
    pub node: usize,
    pub area_m2: f64,
    pub normal: Vec3,
    pub alpha: f64,
    pub eps: f64,
    /// 入力の外面（`ThermalNetwork::surfaces`）の添字。板を升に分けると 1 つの外面が升ごとの複数の `Surface` になるので、
    /// 日照割合（影）は入力の外面ごとに 1 つ求め、升には面積の割合で配った直達日射が同じ割合で掛かる
    pub input: usize,
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
    /// 使う結合の一覧（出力に載せる）
    pub couplings: ThermalCouplings,
    /// 外面（`surfaces` と同じ順）の日照割合の評価器。影を数えないときは `None`
    pub shading: Option<LitSurfaces>,
    /// 板を升に分けたあとの節点（結果は入力ではなくこの順・この ID で並ぶ。板が無ければ入力の節点と同じ）
    pub nodes: Vec<ThermalNode>,
    /// 板の部品 ID → 升の節点の添字と面積の割合（発熱・初期温度・外面を配る）
    pub groups: std::collections::BTreeMap<String, Vec<(usize, f64)>>,
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
        let plan = subdivide::Plan::new(&net.nodes, &net.plates)?;
        let ids: Vec<String> = plan.nodes.iter().map(|n| n.id.clone()).collect();
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
            couplings: ThermalCouplings::default(),
            shading: None,
            groups: plan
                .groups
                .iter()
                .map(|(k, g)| (k.clone(), g.slots.clone()))
                .collect(),
            nodes: Vec::new(),
            ids,
        };
        for n in &plan.nodes {
            let cap = n.heat_capacity();
            let fixed = n.fixed_temperature.map(Qty::value);
            if n.lacks_heat_capacity() || fixed.is_some_and(|t| t.is_nan() || t <= 0.0) {
                return Err(invalid(format!(
                    "Node {}: the sum of part mass × count × specific heat must be positive, and a fixed temperature must be positive",
                    n.id
                )));
            }
            c.capacity.push(cap);
            c.fixed.push(fixed);
        }
        let mut couplings = couplings::resolve(net)?;
        let mut unlocated = std::collections::BTreeMap::new();
        for l in &couplings.conductive {
            let g = nonneg(l.conductance.value(), &l.a, &l.b)?;
            let geometric = matches!(
                l.source,
                ThermalCouplingSource::Mount | ThermalCouplingSource::Contact
            );
            c.cond
                .extend(plan.spread((&l.a, &l.b), g, geometric, &mut unlocated)?);
        }
        for l in &couplings.radiative {
            let r = nonneg(l.exchange_area.value(), &l.a, &l.b)?;
            c.rad
                .extend(plan.spread((&l.a, &l.b), SIGMA_SB * r, false, &mut unlocated)?);
        }
        c.cond.extend(plan.inplane());
        couplings.plates = plan.used(&c.cond, &c.capacity, &unlocated);
        c.couplings = couplings;
        for (k, s) in net.surfaces.iter().enumerate() {
            for surface in c.surface(s, k)? {
                c.space[surface.node] += surface.eps * SIGMA_SB * surface.area_m2;
                c.surfaces.push(surface);
            }
        }
        c.shading = net.shadowing.as_ref().and_then(|input| {
            LitSurfaces::new(
                input,
                Footprint::Credited,
                net.surfaces.iter().map(|s| (s.id.as_str(), s.normal)),
            )
        });
        c.nodes = plan.nodes;
        Ok(c)
    }

    /// 外面を節点の面（板なら升ごとに面積の割合で分けた面）にする。
    fn surface(&self, s: &ThermalSurface, input: usize) -> Result<Vec<Surface>, AnalysisError> {
        let (alpha, eps) = s.optics()?;
        match (s.area.value() >= 0.0, s.normal.normalized()) {
            (true, Some(normal)) => Ok(self
                .slots(&s.node)?
                .into_iter()
                .map(|(node, share)| Surface {
                    node,
                    area_m2: s.area.value() * f64::from(s.count) * share,
                    normal,
                    alpha,
                    eps,
                    input,
                })
                .collect()),
            _ => Err(invalid(format!(
                "Surface {}: area must be 0 or more and the normal must be a non-zero vector",
                s.id
            ))),
        }
    }

    /// 節点 `id` の（添字, 面積の割合）。板なら升すべて、そうでなければ 1 つ（割合 1）。
    pub fn slots(&self, id: &str) -> Result<Vec<(usize, f64)>, AnalysisError> {
        match self.groups.get(id) {
            Some(g) => Ok(g.clone()),
            None => Ok(vec![(self.index(id)?, 1.0)]),
        }
    }

    /// 節点 ID の添字。
    pub fn index(&self, id: &str) -> Result<usize, AnalysisError> {
        self.ids.iter().position(|x| x == id).ok_or_else(|| {
            if self.groups.contains_key(id) {
                invalid(format!(
                    "Thermal node {id} is a subdivided plate (cells {id}[i,j]); a thermostat heater needs a single node, so put it on a part mounted on the plate"
                ))
            } else {
                invalid(format!("Thermal node {id} does not exist"))
            }
        })
    }

    pub fn len(&self) -> usize {
        self.ids.len()
    }

    /// 内部発熱（[`ThermalLoad::dissipation`]）を節点ごとの配列 [W] にする。
    pub fn loads(&self, loads: &[ThermalLoad]) -> Result<Vec<f64>, AnalysisError> {
        let mut q = vec![0.0; self.len()];
        for l in loads {
            let w = l.dissipation()?.value();
            for (i, share) in self.slots(&l.node)? {
                q[i] += w * share;
            }
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
