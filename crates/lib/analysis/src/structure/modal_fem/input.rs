//! `structure.modal_fem` の入力の型。座標はすべて機体座標（SI）。パネルは座標軸に平行な長方形に限る。

use sat_units::typed::{Length, Mass, MassDensity, Pressure, Qty};
use serde::{Deserialize, Serialize};

/// 座標軸（添字は x = 0, y = 1, z = 2）。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(rename_all = "snake_case")]
pub enum ModalFemAxis {
    X,
    Y,
    Z,
}

impl ModalFemAxis {
    /// 軸の添字（x = 0, y = 1, z = 2）。
    pub fn index(self) -> usize {
        match self {
            Self::X => 0,
            Self::Y => 1,
            Self::Z => 2,
        }
    }

    /// 添字から軸（0..3 以外は `None`）。
    pub fn from_index(i: usize) -> Option<Self> {
        [Self::X, Self::Y, Self::Z].get(i).copied()
    }
}

/// 機体を囲む箱の面（法線の向き付きの軸）。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub enum ModalFemFace {
    #[serde(rename = "+x")]
    PlusX,
    #[serde(rename = "-x")]
    MinusX,
    #[serde(rename = "+y")]
    PlusY,
    #[serde(rename = "-y")]
    MinusY,
    #[serde(rename = "+z")]
    PlusZ,
    #[serde(rename = "-z")]
    MinusZ,
}

impl ModalFemFace {
    /// 面の法線の軸。
    pub fn axis(self) -> ModalFemAxis {
        match self {
            Self::PlusX | Self::MinusX => ModalFemAxis::X,
            Self::PlusY | Self::MinusY => ModalFemAxis::Y,
            Self::PlusZ | Self::MinusZ => ModalFemAxis::Z,
        }
    }

    /// 面が箱の最大側（`+`）か。
    pub fn is_max(self) -> bool {
        matches!(self, Self::PlusX | Self::PlusY | Self::PlusZ)
    }
}

/// 軸に平行な長方形のパネル（中央面）。
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct ModalFemPanel {
    pub id: String,
    /// 中央面の中心 [m]
    pub center: [Qty<Length>; 3],
    /// 板の法線の軸
    pub normal: ModalFemAxis,
    /// 面内の 2 辺の長さ [m]（法線以外の 2 軸を x, y, z の昇順に並べた順）
    pub size: [Qty<Length>; 2],
    pub thickness: Qty<Length>,
    pub youngs_modulus: Qty<Pressure>,
    pub poisson_ratio: f64,
    pub density: Qty<MassDensity>,
    /// 板の全質量。与えると質量行列だけ密度 × 板厚 × 面積の代わりにこの値を使う（剛性は変えない。肉抜きの表現）
    #[serde(default)]
    pub mass: Option<Qty<Mass>>,
}

/// 集中質量（搭載部品など）。
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct ModalFemLumped {
    pub id: String,
    /// 重心の位置 [m]
    pub position: [Qty<Length>; 3],
    pub mass: Qty<Mass>,
    /// 外形の箱の x, y, z 方向の長さ [m]。与えると、取り付けるパネルの面への投影の範囲に質量を配る（無ければ 1 点）
    #[serde(default)]
    pub footprint: Option<[Qty<Length>; 3]>,
    /// 取り付けるパネルの ID（無ければ最も近いパネル）
    #[serde(default)]
    pub panel: Option<String>,
}

/// 固定する対象。
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ModalFemTarget {
    /// 面（機体を囲む箱の面）にある節点
    Face { face: ModalFemFace },
    /// 2 つの面が交わる辺（直方体の辺）にある節点
    Edge { faces: [ModalFemFace; 2] },
    /// 指定した位置に最も近い節点 1 つ
    Point { position: [Qty<Length>; 3] },
}

/// 固定の 1 件。
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct ModalFemFixedItem {
    pub target: ModalFemTarget,
    /// `true` なら並進 3 と回転 3 をすべて固定（クランプ）。省略か `false` は並進 3 だけ（回転は自由）
    #[serde(default)]
    pub clamp: bool,
}

/// 拘束の種類。
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ModalFemSupport {
    /// 拘束なし（テストと診断）
    Free,
    /// 面の節点の全自由度を固定（ロケットのアダプタ）
    BaseFixed { face: ModalFemFace },
    /// 機軸に平行な 4 本の辺を横方向に固定し、8 つの角を機軸方向に固定（CubeSat のディスペンサ）
    Rails,
    /// 面の、機軸に平行な 2 本の辺の並進を固定
    Tabs { face: ModalFemFace },
    /// 面・辺・点を列挙して固定
    Fixed { items: Vec<ModalFemFixedItem> },
}

/// `structure.modal_fem` の入力。
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct ModalFemInput {
    pub panels: Vec<ModalFemPanel>,
    #[serde(default)]
    pub lumped: Vec<ModalFemLumped>,
    /// 全節点に、担当する面積に比例して配る非構造質量（配線・断熱材など）
    #[serde(default)]
    pub distributed_mass: Option<Qty<Mass>>,
    /// 機軸（打ち上げ方向の軸。モードの分類、`rails`・`tabs` が使う）
    pub launch_axis: ModalFemAxis,
    pub support: ModalFemSupport,
    /// 要素の大きさ（既定 min(最短の辺 / 4, 25 mm)）
    #[serde(default)]
    pub mesh_size: Option<Qty<Length>>,
    /// 求めるモードの数（既定 6）
    #[serde(default)]
    pub mode_count: Option<u32>,
    /// 辺・面の位置を一致とみなす許容差（既定 1 mm）
    #[serde(default)]
    pub merge_tolerance: Option<Qty<Length>>,
    /// 面内回転（ドリリング）の拘束の強さ（せん断弾性係数 × 板厚に対する倍率。既定は `DRILLING_FACTOR`）
    #[serde(default)]
    pub drilling_factor: Option<f64>,
}
