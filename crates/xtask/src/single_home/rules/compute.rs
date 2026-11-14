//! 計算（sat-astro・sat-analysis）の「1 か所だけ」の規則（データだけ。照合は `single_home`）。台帳（registry.md §2）に行を足したら、ここにも足す。

use super::Rule;

/// (探す文字列, 許すパスの接尾辞の一覧, 台帳での機能名)
pub(in crate::single_home) const RULES: &[Rule] = &[
    (
        "struct Sizing",
        &["crates/lib/analysis/src/power/sizing.rs"],
        "電源の逆算サイジングの型",
    ),
    (
        "fn smad_requirements",
        &["crates/lib/analysis/src/power/sizing/calculate.rs"],
        "電源の逆算サイジングの式",
    ),
    // 積分器
    ("fn rk4", &["crates/lib/astro/src/ode.rs"], "積分器"),
    ("struct Dop853", &["crates/lib/astro/src/ode.rs"], "積分器"),
    // 定数（値の手書き）
    (
        "3.986_004_418e14",
        &["crates/lib/astro/src/constants.rs"],
        "地球の μ",
    ),
    (
        "3.986004418e14",
        &["crates/lib/astro/src/constants.rs"],
        "地球の μ",
    ),
    (
        "398600.4418",
        &["crates/lib/astro/src/constants.rs"],
        "地球の μ",
    ),
    (
        "6_378_137",
        &["crates/lib/astro/src/constants.rs"],
        "地球の赤道半径",
    ),
    (
        "6378137",
        &["crates/lib/astro/src/constants.rs"],
        "地球の赤道半径",
    ),
    (
        "5.670_374_419e-8",
        &["crates/lib/astro/src/constants.rs"],
        "シュテファン＝ボルツマン定数",
    ),
    (
        "5.670374419e-8",
        &["crates/lib/astro/src/constants.rs"],
        "シュテファン＝ボルツマン定数",
    ),
    (
        "1.380_649e-23",
        &["crates/lib/astro/src/constants.rs"],
        "ボルツマン定数",
    ),
    (
        "1.380649e-23",
        &["crates/lib/astro/src/constants.rs"],
        "ボルツマン定数",
    ),
    (
        "2.997_924_58e8",
        &["crates/lib/astro/src/constants.rs"],
        "光速",
    ),
    ("299792458", &["crates/lib/astro/src/constants.rs"], "光速"),
    (
        "fn solve_dense",
        &["crates/lib/astro/src/math"],
        "連立一次方程式の解法",
    ),
    (
        "fn gauss_elim",
        &["crates/lib/astro/src/math"],
        "連立一次方程式の解法",
    ),
    // 天体・軌道・環境の計算
    (
        "fn earth_shadow_factor",
        &["crates/lib/astro/src/shadow.rs"],
        "地球の影",
    ),
    (
        "fn sun_direction",
        &["crates/lib/astro/src/ephem.rs"],
        "太陽の方向",
    ),
    ("fn gmst", &["crates/lib/astro/src/time.rs"], "恒星時"),
    (
        "fn eci_to_ecef",
        &["crates/lib/astro/src/earth.rs"],
        "座標変換",
    ),
    (
        "fn dipole_field",
        &["crates/lib/astro/src/geomag.rs"],
        "地磁気",
    ),
    (
        "fn density_exponential",
        &["crates/lib/astro/src/atmosphere.rs"],
        "大気密度",
    ),
    (
        "fn look_angles",
        &["crates/lib/astro/src/ground.rs"],
        "見通し角",
    ),
    (
        "fn find_passes",
        &["crates/lib/astro/src/ground.rs"],
        "パスの検出",
    ),
    (
        "fn solar_incidence",
        &["crates/lib/astro/src/illumination.rs"],
        "面への太陽入射",
    ),
    (
        "fn body_to_eci",
        &["crates/lib/astro/src/attitude.rs"],
        "姿勢則",
    ),
    (
        "fn sso_inclination",
        &["crates/lib/astro/src/orbit.rs"],
        "太陽同期傾斜角",
    ),
    ("fn beta_angle", &["crates/lib/astro/src/orbit.rs"], "β 角"),
    (
        "fn delta_v(",
        &["crates/lib/astro/src/rocket.rs"],
        "ロケット方程式",
    ),
    (
        "fn edelbaum(",
        &["crates/lib/astro/src/transfer.rs"],
        "軌道遷移の式",
    ),
    (
        "fn patched_conic(",
        &["crates/lib/astro/src/transfer.rs"],
        "遷移の式",
    ),
    (
        "fn lambert(",
        &["crates/lib/astro/src/transfer.rs"],
        "遷移の式",
    ),
    (
        "fn body_view_factor(",
        &["crates/lib/astro/src/illumination.rs"],
        "形態係数",
    ),
    (
        "fn planet_environment(",
        &["crates/lib/analysis/src/thermal"],
        "天体の熱環境",
    ),
    // 区間の抽出
    (
        "fn windows(",
        &["crates/lib/astro/src/timeline.rs"],
        "時系列の区間の抽出",
    ),
    (
        "fn orbit_ranges(",
        &["crates/lib/analysis/src/spec.rs"],
        "周回の切り出し",
    ),
    // 時系列の集計（spec の子 `spec/orbit.rs` も spec.rs の許可に入る）
    (
        "fn orbit_mean(",
        &["crates/lib/analysis/src/spec.rs"],
        "周回平均",
    ),
    (
        "fn time_fraction(",
        &["crates/lib/analysis/src/spec.rs"],
        "時間の割合",
    ),
    (
        "SUNLIT_THRESHOLD: f64",
        &["crates/lib/analysis/src/spec.rs"],
        "日照の閾値",
    ),
    (
        "fn slant_range",
        &["crates/lib/astro/src/ground.rs"],
        "斜距離",
    ),
    ("fn drag_force", &["crates/lib/astro/src/force.rs"], "抗力"),
    (
        "struct StateValues",
        &["crates/lib/analysis/src/modes.rs"],
        "状態ごとの値の表",
    ),
    (
        "fn range_verdict",
        &["crates/lib/analysis/src/thermal/range.rs"],
        "温度範囲の判定",
    ),
    // 推力をかけた伝搬・平均要素・推薬の流量（#79）
    (
        "fn propagate_powered",
        &["crates/lib/astro/src/orbit/powered.rs"],
        "推力をかけた軌道伝搬",
    ),
    (
        "fn mean_semi_major_axis_m",
        &["crates/lib/astro/src/orbit/mean.rs"],
        "平均の長半径（J2 の短周期項）",
    ),
    (
        "fn mass_flow_kg_s",
        &["crates/lib/astro/src/rocket.rs"],
        "推薬の質量流量",
    ),
];
