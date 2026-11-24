//! `structure.modal_fem` の入力の型。座標はすべて機体座標（SI）。パネルは平らな長方形（座標軸に平行か、`orientation` で斜めに置く）。

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
    /// すべての面（綴りは `+x`・`-x`・…。打ち上げ条件の属性 `launch_constraint_face` の語彙と同じで、projection のテストが照合する）。
    pub const ALL: [Self; 6] = [
        Self::PlusX,
        Self::MinusX,
        Self::PlusY,
        Self::MinusY,
        Self::PlusZ,
        Self::MinusZ,
    ];

    /// 面の綴り（`+x`・`-z` など。serde の綴りと同じ）。
    pub fn as_str(self) -> &'static str {
        match self {
            Self::PlusX => "+x",
            Self::MinusX => "-x",
            Self::PlusY => "+y",
            Self::MinusY => "-y",
            Self::PlusZ => "+z",
            Self::MinusZ => "-z",
        }
    }

    /// 綴りから面（知らない綴りは `None`）。
    pub fn parse(text: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|f| f.as_str() == text)
    }

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

/// 長方形のパネル（中央面）。`orientation` が無ければ座標軸に平行（法線は座標軸）、あれば箱の姿勢で斜めに置く。
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct ModalFemPanel {
    pub id: String,
    /// 中央面の中心 [m]
    pub center: [Qty<Length>; 3],
    /// 板の法線の軸（`orientation` があれば、箱の座標系（部品の座標系）の軸）
    pub normal: ModalFemAxis,
    /// 面内の 2 辺の長さ [m]（法線以外の 2 軸を x, y, z の昇順に並べた順。`orientation` があれば箱の座標系の軸の順）
    pub size: [Qty<Length>; 2],
    /// 箱の座標系 → 機体座標系の回転 `[w, x, y, z]`（単位クォータニオン）。**あれば斜めのパネル**で、`center` は箱の中心、
    /// 面内の格子はパネルの面の中で作る（軸に平行なパネルと共通の格子は使わない）。無ければ座標軸に平行（`normal` は機体の軸）
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub orientation: Option<[f64; 4]>,
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
    /// 取り付けるパネルの ID（無ければ、`frame`、さらに無ければ最も近いパネル。同じ距離のパネルが複数あれば質量を等分する）
    #[serde(default)]
    pub panel: Option<String>,
    /// 取り付け先をたどったフレーム（`ModalFemFrame::id`）。`panel` が無く、このフレームに部材があれば、質量はフレームの最も近い 2 本の辺に配る
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub frame: Option<String>,
}

/// フレームの部材の断面（正方形。中実か、肉厚一定の角パイプ）。面積と断面二次モーメントは解析が出す（`beam`）。
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct ModalFemMemberSection {
    /// 外側の辺の長さ [m]
    pub width: Qty<Length>,
    /// 肉厚 [m]（無ければ中実）
    #[serde(default)]
    pub wall: Option<Qty<Length>>,
}

/// フレーム（構体の枠）。箱（軸に平行）の 12 本の辺を、辺の上のパネルの節点どうしをつなぐ梁（部材）にする。
/// 質量は部材に沿って配る（断面の寸法から出る質量ではなく、`mass` が正）。
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct ModalFemFrame {
    pub id: String,
    /// 箱の中心 [m]
    pub center: [Qty<Length>; 3],
    /// 箱の辺の長さ（機体の x, y, z 方向）[m]
    pub size: [Qty<Length>; 3],
    pub youngs_modulus: Qty<Pressure>,
    pub poisson_ratio: f64,
    pub section: ModalFemMemberSection,
    /// フレームの全質量（員数を掛けた素の質量）
    pub mass: Qty<Mass>,
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

/// 拘束の種類の綴りから拘束を作れなかった理由。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ModalFemSupportError {
    /// 知らない綴り
    UnknownKind,
    /// 面が要る種類なのに、面が無い
    FaceRequired,
}

impl ModalFemSupport {
    /// 打ち上げ条件の拘束の種類の綴り（属性 `launch_constraint` の語彙。projection のテストが属性定義と照合する）。
    pub const LAUNCH_KINDS: [&'static str; 5] = ["base_fixed", "rails", "tabs", "fixed", "free"];

    /// 打ち上げ条件の拘束の種類（`LAUNCH_KINDS` の綴り）と面から拘束を作る。**綴りの意味はここだけ**:
    /// `base_fixed` は面の全自由度、`tabs` は面の機軸に平行な 2 本の辺、`fixed` は面の節点の並進（回転は自由）、
    /// `rails` と `free` は面を使わない。
    pub fn from_launch_kind(
        kind: &str,
        face: Option<ModalFemFace>,
    ) -> Result<Self, ModalFemSupportError> {
        use ModalFemSupportError::{FaceRequired, UnknownKind};
        match (kind, face) {
            ("rails", _) => Ok(Self::Rails),
            ("free", _) => Ok(Self::Free),
            ("base_fixed" | "tabs" | "fixed", None) => Err(FaceRequired),
            ("base_fixed", Some(face)) => Ok(Self::BaseFixed { face }),
            ("tabs", Some(face)) => Ok(Self::Tabs { face }),
            ("fixed", Some(face)) => Ok(Self::Fixed {
                items: vec![ModalFemFixedItem {
                    target: ModalFemTarget::Face { face },
                    clamp: false,
                }],
            }),
            _ => Err(UnknownKind),
        }
    }
}

/// モデルにしなかったパネル（理由つき。入力から出力へそのまま渡して、結果の読み手に見せる）。
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct ModalFemSkipped {
    pub id: String,
    /// モデルにしなかった理由
    pub reason: String,
}

/// `structure.modal_fem` の入力。
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct ModalFemInput {
    pub panels: Vec<ModalFemPanel>,
    #[serde(default)]
    pub lumped: Vec<ModalFemLumped>,
    /// フレーム（部材の剛性を入れるもの。無ければフレームの質量は `distributed_mass` として全体に配るだけ）
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    #[cfg_attr(feature = "ts", ts(optional, as = "Option<Vec<ModalFemFrame>>"))]
    pub frames: Vec<ModalFemFrame>,
    /// 射影がモデルにできなかったパネル（出力の `skipped_panels` にそのまま出す。質量は `lumped` に入れる）
    #[serde(default)]
    pub skipped_panels: Vec<ModalFemSkipped>,
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
