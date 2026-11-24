//! 中心天体が地球以外のときの、大気・磁場のモデルについての判定。
//!
//! 大気と磁場のモデルは地球だけにある（`sat_astro::atmosphere`・`sat_astro::geomag`）。ほかの中心天体では
//! 空力トルクと磁気トルクは 0 になるが、その 0 は 2 通りある。**月のように大気・磁場が無い**なら 0 が正しい。
//! **火星・木星のように天体には大気・磁場があってモデルが無い**なら 0 は正しくなく、結果は過小評価になりうる。
//! 後者を「0」とだけ出さず、判定の文で利用者に伝える。

use super::DisturbanceSpacecraft;
use crate::spec::{Check, Verdict};
use sat_astro::body::EnvironmentModel;
use sat_astro::ephem::CentralBody;

impl DisturbanceSpacecraft {
    /// 地球以外の中心天体 `central` で、空力トルク・磁気トルクが 0 になることを伝える判定。
    /// 地球のとき、また空力の面も残留磁気も与えていない（0 になるものが無い）ときは `None`。
    /// 天体に大気・磁場が無いなら `Ok`、有るのにモデルが無いなら `Warning`。
    pub fn environment_check(&self, central: CentralBody) -> Option<Check> {
        let mut lines = Vec::new();
        let mut verdict = Verdict::Ok;
        let mut note = |torque: &str, cause: &str, model: EnvironmentModel| match model {
            EnvironmentModel::Modelled => {}
            EnvironmentModel::Absent => {
                lines.push(format!("{torque} is 0 for this body (no {cause})"));
            }
            EnvironmentModel::NotModelled => {
                verdict = Verdict::Warning;
                lines.push(format!(
                    "{torque} is not included (this body has {cause} but there is no model for it; it is not 0, so the result may be an underestimate)"
                ));
            }
        };
        if self.aero.is_some() {
            note(
                "Aerodynamic torque",
                "atmosphere",
                central.atmosphere_model(),
            );
        }
        if self.residual_dipole().norm() > 0.0 {
            note(
                "Magnetic torque",
                "magnetic field",
                central.magnetic_field_model(),
            );
        }
        (!lines.is_empty()).then(|| {
            Check::new(
                "environment_models",
                "Atmosphere and magnetic field models",
                verdict,
                lines.join(". "),
            )
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::aocs::disturbance::DisturbanceSurface;
    use sat_astro::math::{Mat3, Vec3};
    use sat_units::typed::Qty;

    fn spacecraft(aero: bool, dipole: bool) -> DisturbanceSpacecraft {
        DisturbanceSpacecraft {
            inertia: Mat3::diagonal(Vec3::new(0.05, 0.05, 0.05)),
            aero: aero.then(|| DisturbanceSurface {
                coefficient: 2.2,
                area: Some(Qty::from_raw(0.1)),
                cp_offset: None,
            }),
            srp: None,
            geometry: None,
            residual_dipole: dipole.then_some(Vec3::new(0.1, 0.0, 0.0)),
            atmosphere: Default::default(),
            density_scale: None,
            thrust: None,
        }
    }

    /// 月は大気も磁場も無いので 0 が正しく `Ok`。地球は注記なし。
    #[test]
    fn moon_is_ok_and_earth_has_no_note() {
        let sc = spacecraft(true, true);
        let c = sc.environment_check(CentralBody::Moon).expect("note");
        assert_eq!(c.verdict, Verdict::Ok);
        assert_eq!(
            c.message,
            "Aerodynamic torque is 0 for this body (no atmosphere). Magnetic torque is 0 for this body (no magnetic field)"
        );
        assert!(sc.environment_check(CentralBody::Earth).is_none());
    }

    /// 火星は大気があるのにモデルが無いので「含めていない」と `Warning`（0 とは言わない）。磁場は無いので 0 が正しい。
    /// 木星は磁場があるのにモデルが無い。
    #[test]
    fn missing_models_warn_instead_of_claiming_zero() {
        let sc = spacecraft(true, true);
        let mars = sc.environment_check(CentralBody::Mars).expect("note");
        assert_eq!(mars.verdict, Verdict::Warning);
        assert!(
            mars.message.starts_with(
                "Aerodynamic torque is not included (this body has atmosphere but there is no model for it"
            )
        );
        assert!(
            mars.message
                .ends_with("Magnetic torque is 0 for this body (no magnetic field)")
        );
        let jupiter = sc.environment_check(CentralBody::Jupiter).expect("note");
        assert_eq!(jupiter.verdict, Verdict::Warning);
        assert!(
            jupiter
                .message
                .contains("Magnetic torque is not included (this body has magnetic field but")
        );
    }

    /// 空力の面も残留磁気も無ければ、0 になるものが無いので注記は要らない。片方だけならその分だけ。
    #[test]
    fn only_the_torques_that_exist_are_mentioned() {
        assert!(
            spacecraft(false, false)
                .environment_check(CentralBody::Moon)
                .is_none()
        );
        let c = spacecraft(true, false)
            .environment_check(CentralBody::Moon)
            .expect("note");
        assert_eq!(
            c.message,
            "Aerodynamic torque is 0 for this body (no atmosphere)"
        );
    }
}
