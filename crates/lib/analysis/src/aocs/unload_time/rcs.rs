//! RCS でのアンロード（スラスタの配置から求める）。
//!
//! スラスタ i のトルクは `τ_i = (r_i − r_cm) × (F_i d̂_i)`（機体座標系）。角運動量 ĥ を捨てるには −ĥ 向きのトルクが要る。
//! **仮定: −ĥ 向きの成分 `c_i = τ_i · (−ĥ)` が正のスラスタをすべて同時に連続噴射する**（オン・オフだけで
//! −ĥ 向きのトルクを最大にする組み合わせ。パルス幅の調整や最適配分はしない）。
//! 捨てる向きのトルクは `T = Σ c_i`、時間 `t = H / T`、推薬 `m = Σ F_i t / (Isp_i g₀)`。
//! −ĥ と直交するトルクと正味の推力は打ち消さずに残るので、副作用として返す。
//! ĥ を与えなければ機体の ±x・±y・±z の 6 方向で評価し、最も時間がかかる方向を `worst` にする。

use super::UnloadAssumption;
use crate::spec::{AnalysisError, ensure_positive};
use sat_astro::constants::G0;
use sat_astro::math::Vec3;
use sat_units::typed::{Force, Mass, Qty, Time, Torque};
use serde::{Deserialize, Serialize};

/// スラスタ 1 本。
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct UnloadTimeThruster {
    pub id: String,
    pub thrust: Qty<Force>,
    pub isp: Qty<Time>,
    /// 取付位置 [m]（機体座標系）
    pub position: Vec3,
    /// 推力の向き（機体座標系。機体が押される向き。長さは問わない）
    pub direction: Vec3,
}

/// RCS での結果。
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct UnloadTimeRcs {
    /// 方向ごとの結果（`momentum_direction` を与えたらその 1 つ、無ければ機体の ±x・±y・±z）
    pub directions: Vec<UnloadTimeRcsResult>,
    /// 最も時間がかかる方向（捨てられない方向があればそれ）
    pub worst: UnloadTimeRcsResult,
    /// 仮定（文は service が語彙で引く）
    pub assumption: UnloadAssumption,
}

/// 1 方向の結果。
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct UnloadTimeRcsResult {
    /// 方向の名前（`"+x"` など。与えた方向なら `"given"`）
    pub label: String,
    /// 捨てる角運動量の向き ĥ（機体座標系の単位ベクトル。トルクは −ĥ 向き）
    pub momentum_direction: Vec3,
    /// −ĥ 向きのトルク
    #[serde(serialize_with = "sat_units::typed::TorqueKind::serialize")]
    pub torque: Qty<Torque>,
    /// −ĥ と直交して残るトルクの大きさ
    #[serde(serialize_with = "sat_units::typed::TorqueKind::serialize")]
    pub off_axis_torque: Qty<Torque>,
    /// 噴射するスラスタの正味の推力の大きさ（並進の副作用）
    pub net_force: Qty<Force>,
    /// 時間（この方向に捨てられるスラスタが無ければ `None`）
    pub time: Option<Qty<Time>>,
    /// 推薬の量（`time` が無ければ `None`）
    pub propellant: Option<Qty<Mass>>,
    /// 噴射するスラスタの ID
    pub thrusters: Vec<String>,
}

/// スラスタの値を調べ、各スラスタの (トルク [N·m], 推力 [N], Isp [s]) を返す。
fn thruster_torques(
    thrusters: &[UnloadTimeThruster],
    center_of_mass: Vec3,
) -> Result<Vec<(Vec3, f64, f64)>, AnalysisError> {
    thrusters
        .iter()
        .map(|t| {
            let (f, isp) = (t.thrust.value(), t.isp.value());
            ensure_positive(&format!("スラスタ {} の推力", t.id), f)?;
            ensure_positive(&format!("スラスタ {} の比推力", t.id), isp)?;
            let d = t
                .direction
                .normalized()
                .filter(|_| t.position.norm().is_finite())
                .ok_or_else(|| {
                    AnalysisError::InvalidInput(format!("スラスタ {} の位置・向きが不正です", t.id))
                })?;
            Ok(((t.position - center_of_mass).cross(d * f), f, isp))
        })
        .collect()
}

/// 1 方向（ĥ、単位ベクトル）で評価する。
fn evaluate(
    label: &str,
    h_hat: Vec3,
    momentum: f64,
    thrusters: &[UnloadTimeThruster],
    torques: &[(Vec3, f64, f64)],
) -> UnloadTimeRcsResult {
    let dump = -h_hat;
    let mut torque = Vec3::ZERO;
    let mut force = Vec3::ZERO;
    let mut used = Vec::new();
    let mut flow = 0.0; // Σ F / (Isp g₀) [kg/s]
    for (t, &(tau, f, isp)) in thrusters.iter().zip(torques) {
        if tau.dot(dump) > 0.0 {
            torque += tau;
            force += t.direction.normalized().unwrap_or(Vec3::ZERO) * f;
            flow += f / (isp * G0);
            used.push(t.id.clone());
        }
    }
    let along = torque.dot(dump);
    let time = (along > 0.0).then(|| momentum / along);
    UnloadTimeRcsResult {
        label: label.to_owned(),
        momentum_direction: h_hat,
        torque: Qty::from_raw(along.max(0.0)),
        off_axis_torque: Qty::from_raw((torque - dump * along).norm()),
        net_force: Qty::from_raw(force.norm()),
        time: time.map(Qty::from_raw),
        propellant: time.map(|t| Qty::from_raw(flow * t)),
        thrusters: used,
    }
}

/// RCS でのアンロードを評価する。`momentum` [N·m·s]、`direction` は ĥ（機体、省略時は ±x・±y・±z）。
pub fn rcs_unload(
    momentum: f64,
    direction: Option<Vec3>,
    thrusters: &[UnloadTimeThruster],
    center_of_mass: Vec3,
) -> Result<UnloadTimeRcs, AnalysisError> {
    let torques = thruster_torques(thrusters, center_of_mass)?;
    let cases: Vec<(String, Vec3)> = match direction {
        Some(d) => {
            let h = d.normalized().ok_or_else(|| {
                AnalysisError::InvalidInput("角運動量の向きがゼロベクトルです".to_owned())
            })?;
            vec![("given".to_owned(), h)]
        }
        None => [
            ("+x", Vec3::X),
            ("-x", -Vec3::X),
            ("+y", Vec3::Y),
            ("-y", -Vec3::Y),
            ("+z", Vec3::Z),
            ("-z", -Vec3::Z),
        ]
        .iter()
        .map(|(l, v)| ((*l).to_owned(), *v))
        .collect(),
    };
    let directions: Vec<UnloadTimeRcsResult> = cases
        .iter()
        .map(|(l, h)| evaluate(l, *h, momentum, thrusters, &torques))
        .collect();
    // 捨てられない方向（time = None）を最悪とし、そうでなければ最も長い時間
    let key = |r: &UnloadTimeRcsResult| r.time.map_or(f64::INFINITY, |t| t.value());
    let worst = directions
        .iter()
        .max_by(|a, b| key(a).total_cmp(&key(b)))
        .cloned()
        .ok_or_else(|| AnalysisError::InvalidInput("評価する方向がありません".to_owned()))?;
    Ok(UnloadTimeRcs {
        directions,
        worst,
        assumption: UnloadAssumption::RcsSimultaneousFiring,
    })
}
