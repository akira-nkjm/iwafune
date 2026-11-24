//! # sat-astro — 解析ライブラリの共通部品
//!
//! 複数の解析が使う物理・数学の部品を **ここだけ** に置く（docs/architecture/registry.md）。
//! 解析固有の判断はしない。I/O もしない。
//!
//! | モジュール | 責務 |
//! |---|---|
//! | [`math`] | 3 次元ベクトル・3×3 行列・クォータニオン（f64、SI）、対称行列の固有値分解、密行列の連立一次方程式 |
//! | [`ode`] | 常微分方程式の積分器（固定刻み RK4・適応刻み DOP853）。積分器はこれだけ |
//! | [`constants`] | 物理定数・天体定数（μ、半径、光速、太陽定数など） |
//! | [`time`] | 元期（[`time::Epoch`]）と恒星時 |
//! | [`state`] / [`kepler`] / [`body`] | 共有の状態型・Kepler 幾何・天体型と定数（leaf） |
//! | [`orbit`] | 軌道要素・伝搬（Kepler / J2 永年項 / 力モデルの数値積分 / 推力を入れた数値積分・円制限三体問題 `orbit::cr3bp`）・平均の長半径・周期・太陽同期傾斜角・β 角（β からの RAAN）。中心天体は地球・月・太陽・惑星。解析が読む軌道の共通の入口 `Trajectory`（時刻での状態・中心天体・元期）と繰り返しの長さ `Cycle` |
//! | [`ephem`] | 天体の位置（太陽・月・惑星。解析式と、登録すれば JPL DE440）・重力定数・中心天体・天体の自転 |
//! | [`force`] | 軌道の力モデル（EGM96 高次重力場・第三体・太陽輻射圧・大気抵抗・固体潮汐・相対論・推力の向きと加速度）と項ごとの内訳 |
//! | [`shadow`] | 地球・月・惑星の影（円柱・円錐）。食の判定はこれだけ |
//! | [`earth`] | ECI ⇄ ECEF、測地座標、歳差 |
//! | [`geomag`] | 地磁気（傾斜双極子） |
//! | [`atmosphere`] | 大気密度（指数大気・NRLMSISE-00）、宇宙天気（F10.7・Ap）の日ごとの表と太陽活動のプリセット |
//! | [`ground`] | 地上局・見通し角・パスの検出・地上軌跡・斜距離（中心天体の半径） |
//! | [`attitude`] | 姿勢則（太陽指向・地心指向・慣性固定）→ 機体姿勢 |
//! | [`rocket`] | ロケット方程式（推薬 ⇄ ΔV、有効排気速度、噴射時間、推薬の流量）。換算はこれだけ |
//! | [`transfer`] | 軌道遷移の ΔV（ホーマン・平面変更・バイエリプティック・Edelbaum の低推力・有限燃焼の損失の近似）。遷移の式はこれだけ |
//! | [`illumination`] | 面への太陽入射（cos 則と日陰）。電源と熱で共有する |
//! | [`view_factor`] | 長方形どうしの形態係数（閉じた式: 真向かいの平行面・共通の辺を持つ直交面・箱の 6 面。任意の位置・向きの長方形: 遮蔽つきの数値積分 `view_factor::rect`）。長方形間の形態係数はこれだけ |
//! | [`timeline`] | 軌道上の時系列（時刻・位置・速度・太陽方向・日陰・機体姿勢）をまとめて作る。区間の抽出。ミッション幾何 `timeline::geometry`（太陽・地球までの距離・SEP・SPE・光の伝搬時間と極値） |
//!
//! ## 単位の扱い
//! 入力の仕様（[`orbit::OrbitSpec`]・[`ground::GroundStation`] など）は `sat_units` の型付きの量で受ける。
//! 内部計算の関数は f64 の SI 値（m・s・rad・T・kg/m³）で受け渡し、引数名と doc に単位を書く。

pub mod atmosphere;
pub mod attitude;
pub mod constants;
pub mod earth;
pub mod ephem;
pub mod force;
pub mod geomag;
pub mod ground;
pub mod illumination;
pub mod math;
pub mod ode;
pub mod orbit;
pub mod rocket;
pub mod shadow;
pub mod time;
pub mod timeline;
pub mod transfer;
pub mod view_factor;

/// sat-astro のエラー。
#[derive(Debug, thiserror::Error)]
pub enum AstroError {
    /// 入力が物理的に不正（負の長半径、離心率 ≥ 1 など）。
    #[error("Invalid input: {0}")]
    InvalidInput(String),
    /// 時刻の文字列を解釈できない。
    #[error("Cannot parse time: {0}")]
    InvalidEpoch(String),
    /// 日付つきの遷移の 2 天体が、同じ親を回る別々の天体でない。
    #[error(
        "A dated transfer needs two different bodies orbiting the same parent ({from:?} to {to:?})"
    )]
    NotSiblingBodies {
        from: crate::body::CentralBody,
        to: crate::body::CentralBody,
    },
    /// 到着が出発より後でない。
    #[error("The arrival ({arrive}) must be after the departure ({depart})")]
    ArrivalNotAfterDeparture { depart: String, arrive: String },
    /// 日付が暦（解析式の範囲か DE440 の期間）の外にある。
    #[error("{epoch} is outside the ephemeris validity (1800 to 2050 for the analytic planets)")]
    OutsideEphemeris { epoch: String },
}

pub mod body;
pub mod kepler;
pub mod state;
