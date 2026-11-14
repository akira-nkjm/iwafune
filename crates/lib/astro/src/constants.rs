//! 物理定数・天体定数（SI）。**定数はここだけで定義する。**

/// 地球の重力定数 μ [m³/s²]（WGS84）
pub const MU_EARTH: f64 = 3.986_004_418e14;
/// 地球の赤道半径 [m]（WGS84）
pub const R_EARTH: f64 = 6_378_137.0;
/// 地球の J2
pub const J2_EARTH: f64 = 1.082_626_68e-3;
/// 地球の自転角速度 [rad/s]
pub const OMEGA_EARTH: f64 = 7.292_115_146_7e-5;
/// 太陽の半径 [m]
pub const R_SUN: f64 = 6.957e8;
/// 月の半径 [m]
pub const R_MOON: f64 = 1.7374e6;
/// 地球のまわりの月の軌道の長半径 [m]（平均距離。NASA GSFC Moon Fact Sheet の 384 400 km）
pub const A_MOON: f64 = 3.844e8;
/// 天文単位 [m]
pub const AU: f64 = 1.495_978_707e11;
/// 光速 [m/s]
pub const C_LIGHT: f64 = 2.997_924_58e8;
/// 1 AU での太陽定数 [W/m²]
pub const SOLAR_CONSTANT: f64 = 1361.0;
/// ボルツマン定数 [J/K]
pub const K_BOLTZMANN: f64 = 1.380_649e-23;
/// シュテファン＝ボルツマン定数 [W/(m²·K⁴)]
pub const SIGMA_SB: f64 = 5.670_374_419e-8;
/// 地球の平均アルベド
pub const EARTH_ALBEDO: f64 = 0.30;
/// 地球の赤外放射（大気上端の平均）[W/m²]
pub const EARTH_IR: f64 = 237.0;
/// 標準重力加速度 [m/s²]
pub const G0: f64 = 9.806_65;
/// 地球の双極子磁場の赤道表面強度 [T]
pub const B0_EARTH: f64 = 3.12e-5;
/// 太陽の重力定数 [m³/s²]（IAU 2015 公称値）
pub const GM_SUN: f64 = 1.327_124_400_18e20;
/// 月の重力定数 [m³/s²]（DE 系暦）
pub const GM_MOON: f64 = 4.902_800e12;
/// 水星の重力定数 [m³/s²]
pub const GM_MERCURY: f64 = 2.2032e13;
/// 金星の重力定数 [m³/s²]
pub const GM_VENUS: f64 = 3.24859e14;
/// 火星の重力定数 [m³/s²]
pub const GM_MARS: f64 = 4.282837e13;
/// 木星の重力定数 [m³/s²]
pub const GM_JUPITER: f64 = 1.26686534e17;
/// 土星の重力定数 [m³/s²]
pub const GM_SATURN: f64 = 3.7931187e16;
/// 天王星の重力定数 [m³/s²]
pub const GM_URANUS: f64 = 5.793939e15;
/// 海王星の重力定数 [m³/s²]
pub const GM_NEPTUNE: f64 = 6.836529e15;
/// 地球の扁平率 f（WGS84）
pub const FLATTENING_EARTH: f64 = 1.0 / 298.257_223_563;
/// 1 日 [s]
pub const SECONDS_PER_DAY: f64 = 86_400.0;
/// ユリウス世紀 [s]
pub const SECONDS_PER_JULIAN_CENTURY: f64 = 36_525.0 * SECONDS_PER_DAY;
/// 平均太陽日に対する恒星年の太陽の平均運動 [rad/s]（太陽同期軌道の RAAN 回転率 = 360°/365.2422 日）
pub const SUN_MEAN_MOTION: f64 = core::f64::consts::TAU / (365.242_2 * SECONDS_PER_DAY);
/// 地磁気の北磁極（双極子軸の北端）の地理緯度 [deg]（IGRF-13, 2020 年）
pub const GEOMAG_NORTH_POLE_LAT_DEG: f64 = 80.65;
/// 地磁気の北磁極の地理経度 [deg]（IGRF-13, 2020 年）
pub const GEOMAG_NORTH_POLE_LON_DEG: f64 = -72.68;
/// 水星の赤道半径 [m]（IAU WGCCRE 2015）
pub const R_MERCURY: f64 = 2.4397e6;
/// 金星の赤道半径 [m]
pub const R_VENUS: f64 = 6.0518e6;
/// 火星の赤道半径 [m]
pub const R_MARS: f64 = 3.396_19e6;
/// 木星の赤道半径 [m]（1 bar）
pub const R_JUPITER: f64 = 7.1492e7;
/// 土星の赤道半径 [m]（1 bar）
pub const R_SATURN: f64 = 6.0268e7;
/// 天王星の赤道半径 [m]（1 bar）
pub const R_URANUS: f64 = 2.5559e7;
/// 海王星の赤道半径 [m]（1 bar）
pub const R_NEPTUNE: f64 = 2.4764e7;
/// 月の J2（GRAIL。基準半径は [`R_MOON`]）
pub const J2_MOON: f64 = 2.0330e-4;
/// 太陽の J2
pub const J2_SUN: f64 = 2.2e-7;
/// 水星の J2
pub const J2_MERCURY: f64 = 5.03e-5;
/// 金星の J2
pub const J2_VENUS: f64 = 4.458e-6;
/// 火星の J2
pub const J2_MARS: f64 = 1.960_45e-3;
/// 木星の J2
pub const J2_JUPITER: f64 = 1.4736e-2;
/// 土星の J2
pub const J2_SATURN: f64 = 1.6298e-2;
/// 天王星の J2
pub const J2_URANUS: f64 = 3.343e-3;
/// 海王星の J2
pub const J2_NEPTUNE: f64 = 3.411e-3;
/// 秒角 → ラジアン
pub const ARCSEC_TO_RAD: f64 = core::f64::consts::PI / (180.0 * 3600.0);
