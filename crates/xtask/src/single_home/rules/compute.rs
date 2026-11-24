//! 計算（sat-astro・sat-analysis）の「1 か所だけ」の規則（データだけ。照合は `single_home`）。台帳（registry.md §2）に行を足したら、ここにも足す。

use super::Rule;

/// (探す文字列, 許すパスの接尾辞の一覧, 台帳での機能名)
pub(in crate::single_home) const RULES: &[Rule] = &[
    (
        "struct Sizing",
        &["crates/lib/analysis/src/power/sizing.rs"],
        "the types of power back-calculation sizing",
    ),
    (
        "fn smad_requirements",
        &["crates/lib/analysis/src/power/sizing/calculate.rs"],
        "the formulas of power back-calculation sizing",
    ),
    // 積分器
    ("fn rk4", &["crates/lib/astro/src/ode.rs"], "integrators"),
    (
        "struct Dop853",
        &["crates/lib/astro/src/ode.rs"],
        "integrators",
    ),
    // 定数（値の手書き）
    (
        "3.986_004_418e14",
        &["crates/lib/astro/src/constants.rs"],
        "Earth's mu",
    ),
    (
        "3.986004418e14",
        &["crates/lib/astro/src/constants.rs"],
        "Earth's mu",
    ),
    (
        "398600.4418",
        &["crates/lib/astro/src/constants.rs"],
        "Earth's mu",
    ),
    (
        "6_378_137",
        &["crates/lib/astro/src/constants.rs"],
        "Earth's equatorial radius",
    ),
    (
        "6378137",
        &["crates/lib/astro/src/constants.rs"],
        "Earth's equatorial radius",
    ),
    (
        "5.670_374_419e-8",
        &["crates/lib/astro/src/constants.rs"],
        "the Stefan-Boltzmann constant",
    ),
    (
        "5.670374419e-8",
        &["crates/lib/astro/src/constants.rs"],
        "the Stefan-Boltzmann constant",
    ),
    (
        "1.380_649e-23",
        &["crates/lib/astro/src/constants.rs"],
        "the Boltzmann constant",
    ),
    (
        "1.380649e-23",
        &["crates/lib/astro/src/constants.rs"],
        "the Boltzmann constant",
    ),
    (
        "2.997_924_58e8",
        &["crates/lib/astro/src/constants.rs"],
        "the speed of light",
    ),
    (
        "299792458",
        &["crates/lib/astro/src/constants.rs"],
        "the speed of light",
    ),
    (
        "fn solve_dense",
        &["crates/lib/astro/src/math"],
        "the linear system solver",
    ),
    (
        "struct Skyline",
        &["crates/lib/astro/src/math/skyline.rs"],
        "the sparse symmetric (skyline) solver",
    ),
    (
        "fn generalized_symmetric_eigen",
        &["crates/lib/astro/src/math/sym_eigen.rs"],
        "the dense symmetric eigensolver",
    ),
    (
        "fn lowest_modes",
        &["crates/lib/astro/src/math/subspace.rs"],
        "the generalized eigenproblem (modal) solver",
    ),
    // 天体・軌道・環境の計算
    (
        "fn earth_shadow_factor",
        &["crates/lib/astro/src/shadow.rs"],
        "Earth's shadow",
    ),
    (
        "fn sun_direction",
        &["crates/lib/astro/src/ephem.rs"],
        "the direction of the Sun",
    ),
    (
        "fn gmst",
        &["crates/lib/astro/src/time.rs"],
        "sidereal time",
    ),
    (
        "fn eci_to_ecef",
        &["crates/lib/astro/src/earth.rs"],
        "coordinate transforms",
    ),
    (
        "fn dipole_field",
        &["crates/lib/astro/src/geomag.rs"],
        "the geomagnetic field",
    ),
    (
        "fn density_exponential",
        &["crates/lib/astro/src/atmosphere.rs"],
        "atmospheric density",
    ),
    (
        "fn look_angles",
        &["crates/lib/astro/src/ground.rs"],
        "line-of-sight angle",
    ),
    (
        "fn find_passes",
        &["crates/lib/astro/src/ground.rs"],
        "pass detection",
    ),
    (
        "fn solar_incidence",
        &["crates/lib/astro/src/illumination.rs"],
        "solar incidence on a surface",
    ),
    (
        "fn body_to_eci",
        &["crates/lib/astro/src/attitude.rs"],
        "attitude laws",
    ),
    (
        "fn sso_inclination",
        &["crates/lib/astro/src/orbit.rs"],
        "the Sun-synchronous inclination",
    ),
    (
        "fn beta_angle",
        &["crates/lib/astro/src/orbit.rs"],
        "beta angle",
    ),
    (
        "fn delta_v(",
        &["crates/lib/astro/src/rocket.rs"],
        "the rocket equation",
    ),
    (
        "fn edelbaum(",
        &["crates/lib/astro/src/transfer.rs"],
        "orbit transfer formulas",
    ),
    (
        "fn patched_conic(",
        &["crates/lib/astro/src/transfer.rs"],
        "transfer formulas",
    ),
    (
        "fn lambert(",
        &["crates/lib/astro/src/transfer.rs"],
        "transfer formulas",
    ),
    (
        "fn hyperbolic_periapsis_dv_m_s(",
        &["crates/lib/astro/src/transfer/patched.rs"],
        "the delta-v of a hyperbolic periapsis burn",
    ),
    (
        "fn dated_transfer(",
        &["crates/lib/astro/src/transfer/dated.rs"],
        "dated interplanetary transfers",
    ),
    (
        "fn flyby_geometry(",
        &["crates/lib/astro/src/transfer/flyby.rs"],
        "swing-by (gravity assist) geometry and its periapsis burn",
    ),
    (
        "fn default_min_altitude_m(",
        &["crates/lib/astro/src/transfer/flyby.rs"],
        "the default minimum flyby altitude per body class",
    ),
    (
        "fn hyperbolic_periapsis_speed_m_s(",
        &["crates/lib/astro/src/transfer/patched.rs"],
        "the speed at the periapsis of a hyperbola",
    ),
    (
        "fn lambert_chain(",
        &["crates/lib/astro/src/transfer/dated.rs"],
        "the legs of a dated transfer",
    ),
    (
        "fn lambert_about_normal(",
        &["crates/lib/astro/src/transfer/lambert.rs"],
        "transfer formulas",
    ),
    (
        "fn soi_radius_m(",
        &["crates/lib/astro/src/ephem/central.rs"],
        "the sphere-of-influence radius",
    ),
    (
        "fn state_about_parent(",
        &["crates/lib/astro/src/ephem/central.rs"],
        "the state of a body about its parent",
    ),
    (
        "fn body_view_factor(",
        &["crates/lib/astro/src/illumination.rs"],
        "view factors",
    ),
    (
        "fn planet_environment(",
        &["crates/lib/analysis/src/thermal"],
        "the thermal environment of a body",
    ),
    // 区間の抽出
    (
        "fn windows(",
        &["crates/lib/astro/src/timeline.rs"],
        "extracting intervals from a time series",
    ),
    (
        "fn orbit_ranges(",
        &["crates/lib/analysis/src/spec.rs"],
        "cutting out orbits",
    ),
    // 時系列の集計（spec の子 `spec/orbit.rs` も spec.rs の許可に入る）
    (
        "fn orbit_mean(",
        &["crates/lib/analysis/src/spec.rs"],
        "orbit averaging",
    ),
    (
        "fn time_fraction(",
        &["crates/lib/analysis/src/spec.rs"],
        "time fractions",
    ),
    (
        "SUNLIT_THRESHOLD: f64",
        &["crates/lib/analysis/src/spec.rs"],
        "the sunlit threshold",
    ),
    (
        "fn slant_range",
        &["crates/lib/astro/src/ground.rs"],
        "slant range",
    ),
    (
        "fn drag_force",
        &["crates/lib/astro/src/force.rs"],
        "drag force",
    ),
    (
        "struct StateValues",
        &["crates/lib/analysis/src/modes.rs"],
        "per-state value tables",
    ),
    // 温度範囲の判定（片側の判定と、全体の悪いほうの選び方）
    (
        "fn side_verdict(",
        &["crates/lib/analysis/src/thermal/range.rs"],
        "temperature range judgement",
    ),
    (
        "fn worst_verdict(",
        &["crates/lib/analysis/src/thermal/range.rs"],
        "temperature range judgement",
    ),
    // 形態係数・放射の囲い（熱）
    (
        "fn parallel_opposed(",
        &["crates/lib/astro/src/view_factor.rs"],
        "view factors (parallel opposed rectangles)",
    ),
    (
        "fn box_faces(",
        &["crates/lib/astro/src/view_factor.rs"],
        "view factors (faces of a box)",
    ),
    (
        "fn gebhart(",
        &["crates/lib/analysis/src/thermal/enclosure.rs"],
        "Gebhart factors of an enclosure",
    ),
    // 積分の 1 区間（`Dop853::integrate_segment` の公開口と、`ode/segment.rs` の本体。どちらも ode の中）
    (
        "fn integrate_segment",
        &["crates/lib/astro/src/ode.rs"],
        "integrators (one segment)",
    ),
    // 時系列の集計と求根
    (
        "fn time_mean(",
        &["crates/lib/astro/src/timeline.rs"],
        "time averaging of a time series",
    ),
    (
        "fn cumulative_trapezoid(",
        &["crates/lib/astro/src/timeline.rs"],
        "cumulative integral of a time series",
    ),
    (
        "fn bisect(",
        &["crates/lib/astro/src/math/roots.rs"],
        "root finding by bisection",
    ),
    // 平面の凸多角形の和集合・求積則・箱の外形（#388）
    (
        "fn union_measure(",
        &["crates/lib/astro/src/math/planar.rs"],
        "the planar union of convex polygons",
    ),
    (
        "fn gauss_legendre(",
        &["crates/lib/astro/src/math/quadrature.rs"],
        "Gauss-Legendre quadrature nodes",
    ),
    (
        "struct DirectionSet",
        &["crates/lib/analysis/src/structure/external_geometry/directions.rs"],
        "the direction set of the mean projected area",
    ),
    (
        "fn silhouette(",
        &["crates/lib/analysis/src/structure/interference/obb/shape.rs"],
        "the silhouette of an oriented box",
    ),
    (
        "fn mean_projected_area(",
        &["crates/lib/analysis/src/structure/external_geometry/geometry.rs"],
        "the mean projected area of the part boxes",
    ),
    // 余裕の判定と余裕率
    (
        "fn margin_verdict(",
        &["crates/lib/analysis/src/spec.rs"],
        "margin verdict",
    ),
    (
        "fn ratio_margin(",
        &["crates/lib/analysis/src/spec.rs"],
        "margin ratio",
    ),
    // 推力をかけた伝搬・平均要素・推薬の流量（#79）
    (
        "fn propagate_powered",
        &["crates/lib/astro/src/orbit/powered.rs"],
        "orbit propagation with thrust",
    ),
    (
        "fn mean_semi_major_axis_m",
        &["crates/lib/astro/src/orbit/mean.rs"],
        "the mean semi-major axis (J2 short-period terms)",
    ),
    (
        "fn mass_flow_kg_s",
        &["crates/lib/astro/src/rocket.rs"],
        "propellant mass flow",
    ),
    (
        "fn exhaust_velocity(isp_s",
        &["crates/lib/astro/src/rocket.rs"],
        "the effective exhaust velocity",
    ),
    // 固定軸 X → Y → Z の回転角 ⇄ クォータニオン
    (
        "fn from_fixed_xyz",
        &["crates/lib/astro/src/math/euler.rs"],
        "rotation angle <-> quaternion",
    ),
    (
        "fn to_fixed_xyz",
        &["crates/lib/astro/src/math/euler.rs"],
        "rotation angle <-> quaternion",
    ),
];
