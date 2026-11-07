//! テスト用の軌道と時系列（電源と姿勢のテストで共有する）。

use sat_astro::attitude::AttitudeLaw;
use sat_astro::math::Vec3;
use sat_astro::orbit::OrbitSpec;
use sat_astro::shadow::ShadowModel;
use sat_astro::time::Epoch;
use sat_astro::timeline::TimelineSpec;
use sat_units::typed::Qty;

/// 2027-04-01 元期、高度 500 km・傾斜角 97.4°・RAAN 0 の円軌道（J2）。β ≈ −11°。
pub(crate) fn leo() -> OrbitSpec {
    let epoch = Epoch::parse("2027-04-01T00:00:00Z").unwrap();
    OrbitSpec::circular(
        epoch,
        Qty::from_raw(500e3),
        Qty::from_raw(97.4f64.to_radians()),
        Qty::from_raw(0.0),
    )
}

/// `leo()` を `orbits` 周、`step_s` 刻みで、円柱影で回す時系列。
pub(crate) fn leo_timeline(attitude: AttitudeLaw, orbits: f64, step_s: f64) -> TimelineSpec {
    let orbit = leo();
    let span = Qty::from_raw(orbit.period_s() * orbits);
    let mut spec = TimelineSpec::new(orbit, attitude, span, Qty::from_raw(step_s));
    spec.shadow = ShadowModel::Cylindrical;
    spec
}

/// +Z を太陽に、+X を軌道面の法線に向ける太陽指向。
pub(crate) fn sun_pointing_z() -> AttitudeLaw {
    AttitudeLaw::SunPointing {
        axis: Vec3::Z,
        secondary: Vec3::X,
    }
}

/// +Z を地心に、+X を速度方向に向ける地心指向。
pub(crate) fn nadir_pointing_z() -> AttitudeLaw {
    AttitudeLaw::NadirPointing {
        axis: Vec3::Z,
        along_track: Vec3::X,
    }
}
