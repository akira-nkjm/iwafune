//! 設定（`conditions/`・`analysis/`）の量の次元の表（設定のパスの形 → 次元）。**設定の量の次元はここだけに書く。**
//!
//! 検査（`check`）は書かれた量の葉ごとに [`setting_dimension`] で表を引いて次元を確かめ、射影（fune-projection）は
//! 欄の定数（[`ORBIT_ALTITUDE`] など）を渡して `Qty<D>` に読む。欄の型 `D` が射影の型になるので、検査と射影の次元はずれない。
//! パスの形の [`ANY`] は任意の 1 区切り（地上局の ID・帯域・回線・環境条件・材料の名前・配列の番号）。
//! 配分（`budget.<種類>.*`）の次元は種類から決まる（`BudgetKind::dimension`）。

use crate::path::{SettingPath, SettingScope};
use sat_units::Dimension as RuntimeDimension;
use sat_units::typed::{self, Dimension};
use std::marker::PhantomData;

/// パスの形の中の任意の 1 区切り。
pub const ANY: &str = "*";

/// 表の 1 行（パスの形と次元）。
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SettingDimension {
    pub scope: SettingScope,
    /// `scope` の後ろの区切りの形（[`ANY`] を含む）
    pub pattern: &'static [&'static str],
    /// 次元名（`"Length"`。sat-units の次元の型の名前）か、名前の無い次元なら基準の単位（`"W/(m*K)"`）
    pub dimension: &'static str,
    /// 次元の型を持つ欄の次元（名前の無い次元は None）。`dimension` の名前と同じ次元であることは表のテストが確かめる
    pub typed: Option<RuntimeDimension>,
}

impl SettingDimension {
    /// パスがこの形に当たるか。
    pub fn matches(&self, path: &SettingPath) -> bool {
        pattern_matches(self.scope, self.pattern, path)
    }
}

/// パスが形（`scope` と、[`ANY`] を含む区切りの並び）に当たるか。**設定のパスの形の当て方はここだけ**（次元の表と列挙の表が使う）。
pub fn pattern_matches(scope: SettingScope, pattern: &[&str], path: &SettingPath) -> bool {
    path.scope == scope
        && path.segments.len() == pattern.len()
        && pattern
            .iter()
            .zip(&path.segments)
            .all(|(p, s)| *p == ANY || p == s)
}

/// 設定の量 1 つの欄（パスの形と次元の型 `D`）。射影はこれを渡して `Qty<D>` に読む。
pub struct SettingQuantity<D> {
    pub scope: SettingScope,
    /// `scope` の後ろの区切りの形（[`ANY`] を含む）
    pub pattern: &'static [&'static str],
    _dimension: PhantomData<fn() -> D>,
}

impl<D: Dimension> SettingQuantity<D> {
    const fn new(scope: SettingScope, pattern: &'static [&'static str]) -> Self {
        Self {
            scope,
            pattern,
            _dimension: PhantomData,
        }
    }

    /// 形の [`ANY`] を `fill` で前から順に埋めたパス（`GROUND_LATITUDE.at(&["TOKYO_GS"])` →
    /// `conditions.ground.TOKYO_GS.latitude`）。`fill` が足りなければ `*` のまま。
    pub fn at(&self, fill: &[&str]) -> SettingPath {
        let mut fill = fill.iter();
        SettingPath {
            scope: self.scope,
            segments: self
                .pattern
                .iter()
                .map(|s| match *s {
                    ANY => fill.next().copied().unwrap_or(ANY).to_owned(),
                    s => s.to_owned(),
                })
                .collect(),
        }
    }
}

/// 欄の定数と表（[`TYPED`]）を 1 つの並びから作る（パスの形と次元の型をここで 1 回だけ書く）。
macro_rules! setting_quantities {
    ($( $(#[$meta:meta])* $name:ident : $dim:ident = $scope:ident [$($segment:expr),+ $(,)?]; )+) => {
        $(
            $(#[$meta])*
            pub const $name: SettingQuantity<typed::$dim> =
                SettingQuantity::new(SettingScope::$scope, &[$($segment),+]);
        )+

        /// 次元の型を持つ欄の表（宣言の順）。
        pub const TYPED: &[SettingDimension] = &[$(
            SettingDimension {
                scope: SettingScope::$scope,
                pattern: &[$($segment),+],
                dimension: stringify!($dim),
                typed: Some(<typed::$dim as Dimension>::DIM),
            }
        ),+];
    };
}

setting_quantities! {
    /// 季節解析の標本の間隔
    ORBIT_ECLIPSE_SEASON_STEP: Time = Conditions["orbit", "eclipse_season", "step"];
    /// 高度低下の積分の期間の上限（`[decay]`）
    ORBIT_DECAY_SPAN: Time = Conditions["orbit", "decay", "span"];
    /// 再突入とみなす高度
    ORBIT_DECAY_REENTRY_ALTITUDE: Length = Conditions["orbit", "decay", "reentry_altitude"];
    /// 抗力の面積
    ORBIT_DECAY_DRAG_AREA: Area = Conditions["orbit", "decay", "drag_area"];
    /// 軌道維持の高度の帯
    ORBIT_DECAY_BAND: Length = Conditions["orbit", "decay", "band"];
    /// 軌道の高度（`conditions/orbit.toml`）
    ORBIT_ALTITUDE: Length = Conditions["orbit", "altitude"];
    /// 軌道長半径
    ORBIT_SEMI_MAJOR_AXIS: Length = Conditions["orbit", "semi_major_axis"];
    /// 軌道傾斜角
    ORBIT_INCLINATION: Angle = Conditions["orbit", "inclination"];
    /// 昇交点赤経
    ORBIT_RAAN: Angle = Conditions["orbit", "raan"];
    /// 近地点引数
    ORBIT_ARG_PERIGEE: Angle = Conditions["orbit", "arg_perigee"];
    /// 平均近点角
    ORBIT_MEAN_ANOMALY: Angle = Conditions["orbit", "mean_anomaly"];
    /// 準静的荷重（機軸方向。`conditions/launch.toml`）
    LAUNCH_LOAD_AXIAL: Acceleration = Conditions["launch", "load_axial"];
    /// 準静的荷重（直交方向）
    LAUNCH_LOAD_LATERAL: Acceleration = Conditions["launch", "load_lateral"];
    /// 地上局の緯度（`conditions/ground/<局>.toml`）
    GROUND_LATITUDE: Angle = Conditions["ground", ANY, "latitude"];
    /// 地上局の経度
    GROUND_LONGITUDE: Angle = Conditions["ground", ANY, "longitude"];
    /// 地上局の最低仰角
    GROUND_MIN_ELEVATION: Angle = Conditions["ground", ANY, "min_elevation"];
    /// 地上局の標高
    GROUND_ALTITUDE: Length = Conditions["ground", ANY, "altitude"];
    /// 地上局の帯域ごとのアンテナ利得（`[link.<帯域>]`）
    GROUND_ANTENNA_GAIN: Dimensionless = Conditions["ground", ANY, "link", ANY, "antenna_gain"];
    /// 地上局の受信系の雑音温度
    GROUND_NOISE_TEMPERATURE: Temperature = Conditions["ground", ANY, "link", ANY, "noise_temperature"];
    /// 地上局の G/T
    GROUND_G_OVER_T: Dimensionless = Conditions["ground", ANY, "link", ANY, "g_over_t"];
    /// 地上局の送信電力
    GROUND_TX_POWER: Power = Conditions["ground", ANY, "link", ANY, "tx_power"];
    /// 地上局の給電線の損失
    GROUND_LINE_LOSS: Dimensionless = Conditions["ground", ANY, "link", ANY, "line_loss"];
    /// 電源のモードの並びの区間の長さ（`analysis/power.toml` の `[[sequence]]`）
    POWER_SEQUENCE_DURATION: Time = Analysis["power", "sequence", ANY, "duration"];
    /// 電源の名前付きのモードの並びの区間の長さ（`[[sequences.<名前>.segments]]`）
    POWER_SEQUENCES_DURATION: Time = Analysis["power", "sequences", ANY, "segments", ANY, "duration"];
    /// 電源の名前付きのモードの並びの先頭の基準のオフセット（`[sequences.<名前>]` の `anchor.offset`）
    POWER_SEQUENCES_ANCHOR_OFFSET: Time = Analysis["power", "sequences", ANY, "anchor", "offset"];
    /// サイジングのバス電圧。
    POWER_SIZING_BUS_VOLTAGE: Voltage = Analysis["power", "sizing", "bus_voltage"];
    /// サイジングの太陽距離。
    POWER_SIZING_SUN_DISTANCE: Length = Analysis["power", "sizing", "sun_distance"];
    /// 空力の受圧面積（`analysis/aocs.toml`）
    AOCS_AERO_AREA: Area = Analysis["aocs", "aero_area"];
    /// 太陽輻射圧の受圧面積
    AOCS_SRP_AREA: Area = Analysis["aocs", "srp_area"];
    /// 空力の圧力中心 − 重心（ベクトル）
    AOCS_CP_OFFSET_AERO: Length = Analysis["aocs", "cp_offset_aero"];
    /// 太陽輻射圧の圧力中心 − 重心（ベクトル）
    AOCS_CP_OFFSET_SRP: Length = Analysis["aocs", "cp_offset_srp"];
    /// 残留磁気双極子（ベクトル）
    AOCS_RESIDUAL_DIPOLE: MagneticDipoleMoment = Analysis["aocs", "residual_dipole"];
    /// 姿勢変更の角度
    AOCS_SLEW_ANGLE: Angle = Analysis["aocs", "slew_angle"];
    /// 地上に降ろすまで保持する期間（`analysis/cdh.toml`）
    CDH_RETENTION: Time = Analysis["cdh", "retention"];
    /// 所要 Eb/N0（`[link.<回線>]`。dB）
    COMM_EBN0_REQUIRED: Dimensionless = Analysis["comm", "link", ANY, "ebn0_required"];
    /// 指向損失
    COMM_POINTING_LOSS: Dimensionless = Analysis["comm", "link", ANY, "pointing_loss"];
    /// 偏波損失
    COMM_POLARIZATION_LOSS: Dimensionless = Analysis["comm", "link", ANY, "polarization_loss"];
    /// 大気損失
    COMM_ATMOSPHERIC_LOSS: Dimensionless = Analysis["comm", "link", ANY, "atmospheric_loss"];
    /// 実装損失
    COMM_IMPLEMENTATION_LOSS: Dimensionless = Analysis["comm", "link", ANY, "implementation_loss"];
    /// 衛星側の給電線の損失
    COMM_LINE_LOSS: Dimensionless = Analysis["comm", "link", ANY, "line_loss"];
    /// 電力束密度の基準帯域
    COMM_PFD_REFERENCE_BANDWIDTH: Frequency = Analysis["comm", "link", ANY, "pfd_reference_bandwidth"];
    /// 信号の占有帯域
    COMM_OCCUPIED_BANDWIDTH: Frequency = Analysis["comm", "link", ANY, "occupied_bandwidth"];
    /// 取付の接触熱コンダクタンス（`analysis/thermal.toml`）
    THERMAL_CONTACT_CONDUCTANCE: ThermalConductance = Analysis["thermal", "contact_conductance"];
    /// 節点間の放射結合（面積）
    THERMAL_RADIATIVE_COUPLING: Area = Analysis["thermal", "radiative_coupling"];
    /// 明示の節点間の伝導（`[conductive.<節点>] <節点> = …`）
    THERMAL_CONDUCTIVE: ThermalConductance = Analysis["thermal", "conductive", ANY, ANY];
    /// 明示の節点間の放射（有効放射交換面積）
    THERMAL_RADIATIVE: Area = Analysis["thermal", "radiative", ANY, ANY];
    /// 非定常の初期温度
    THERMAL_INITIAL_TEMPERATURE: Temperature = Analysis["thermal", "initial_temperature"];
    /// 非定常の刻み
    THERMAL_TIME_STEP: Time = Analysis["thermal", "time_step"];
    /// 予測温度の不確かさ（`[margin]`。温度差なので K で書く）
    THERMAL_MARGIN_UNCERTAINTY: TemperatureDifference = Analysis["thermal", "margin", "uncertainty"];
    /// 温度範囲の端の警告の幅（温度差）
    THERMAL_MARGIN_WARNING_BAND: TemperatureDifference = Analysis["thermal", "margin", "warning_band"];
    /// 環境条件の太陽光の強さ（`[case.<条件>]`）
    THERMAL_CASE_SOLAR_FLUX: Irradiance = Analysis["thermal", "case", ANY, "solar_flux"];
    /// 環境条件の中心天体の赤外放射（一律）
    THERMAL_CASE_PLANET_IR: Irradiance = Analysis["thermal", "case", ANY, "planet_ir"];
    /// 環境条件の β 角
    THERMAL_CASE_BETA: Angle = Analysis["thermal", "case", ANY, "beta"];
    /// 締結 1 本の許容せん断荷重（`analysis/structure.toml`）
    STRUCTURE_FASTENER_ALLOWABLE: Force = Analysis["structure", "fastener_allowable"];
    /// 部品の箱の重なりを干渉に数えない深さ（`analysis/structure.toml`）
    STRUCTURE_INTERFERENCE_TOLERANCE: Length = Analysis["structure", "interference_tolerance"];
    /// 噴流・視野の円錐を見る、頂点からの距離の上限（`analysis/structure.toml`）
    STRUCTURE_CONE_CLEARANCE_RANGE: Length = Analysis["structure", "cone_clearance_range"];
    /// 粗い有限要素の固有値解析の要素の大きさ（`analysis/structure.toml` の `[modal]`）
    STRUCTURE_MODAL_MESH_SIZE: Length = Analysis["structure", "modal", "mesh_size"];
    /// パネルの辺・面の位置を一致とみなす許容差
    STRUCTURE_MODAL_JOINT_TOLERANCE: Length = Analysis["structure", "modal", "joint_tolerance"];
    /// 材料の密度（`[material.<材料>]`）
    MATERIAL_DENSITY: MassDensity = Analysis["structure", "material", ANY, "density"];
    /// 材料のヤング率
    MATERIAL_YOUNGS_MODULUS: Pressure = Analysis["structure", "material", ANY, "youngs_modulus"];
    /// 材料の降伏強さ
    MATERIAL_YIELD_STRENGTH: Pressure = Analysis["structure", "material", ANY, "yield_strength"];
    /// 材料の引張強さ
    MATERIAL_ULTIMATE_STRENGTH: Pressure = Analysis["structure", "material", ANY, "ultimate_strength"];
    /// 推進系の解析の乾燥質量の上書き（`analysis/propulsion.toml`）
    PROPULSION_DRY_MASS: Mass = Analysis["propulsion", "dry_mass"];
    /// 日付の探索の出発の窓の長さ（`[launch_window]`）
    PROPULSION_LAUNCH_WINDOW_DEPARTURE_SPAN: Time = Analysis["propulsion", "launch_window", "departure_span"];
    /// 日付の探索の飛行時間の下限
    PROPULSION_LAUNCH_WINDOW_TOF_MIN: Time = Analysis["propulsion", "launch_window", "time_of_flight_min"];
    /// 日付の探索の飛行時間の上限
    PROPULSION_LAUNCH_WINDOW_TOF_MAX: Time = Analysis["propulsion", "launch_window", "time_of_flight_max"];
}

/// 次元の型を持たない欄（名前の無い次元。射影は読まない）。
pub const UNTYPED: &[SettingDimension] = &[SettingDimension {
    scope: SettingScope::Analysis,
    pattern: &["structure", "material", ANY, "thermal_conductivity"],
    dimension: "W/(m*K)",
    typed: None,
}];

/// 設定のパスの次元（次元名か基準の単位。表に無ければ None）。
pub fn setting_dimension(path: &SettingPath) -> Option<&'static str> {
    TYPED
        .iter()
        .chain(UNTYPED)
        .find(|d| d.matches(path))
        .map(|d| d.dimension)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::value::units::{parse_dimension_name, unit_info};

    #[test]
    fn names_are_the_dimensions_of_the_types() {
        for d in TYPED {
            let parsed = parse_dimension_name(d.dimension).ok();
            assert_eq!(parsed, d.typed, "{:?}", d.pattern);
        }
        for d in UNTYPED {
            assert!(
                parse_dimension_name(d.dimension).is_err(),
                "{}",
                d.dimension
            );
            assert!(unit_info(d.dimension).is_ok(), "{}", d.dimension);
        }
    }

    #[test]
    fn each_path_has_one_row() {
        let all: Vec<&SettingDimension> = TYPED.iter().chain(UNTYPED).collect();
        for (i, a) in all.iter().enumerate() {
            for b in &all[i + 1..] {
                assert!(
                    !(a.scope == b.scope && a.pattern == b.pattern),
                    "{:?} appears twice",
                    a.pattern
                );
            }
        }
    }

    #[test]
    fn keys_fill_the_wildcards_in_order() {
        let p = GROUND_TX_POWER.at(&["TOKYO_GS", "s_band"]);
        assert_eq!(
            p.to_string(),
            "conditions.ground.TOKYO_GS.link.s_band.tx_power"
        );
        assert_eq!(setting_dimension(&p), Some("Power"));
        assert_eq!(
            setting_dimension(&SettingPath::analysis(&[
                "power", "sequence", "0", "duration"
            ])),
            Some("Time")
        );
        assert_eq!(
            setting_dimension(&SettingPath::analysis(&["power", "dod_max"])),
            None
        );
    }
}
