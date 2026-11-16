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
                lines.push(format!("{torque}はこの天体では 0（{cause}が無い）"));
            }
            EnvironmentModel::NotModelled => {
                verdict = Verdict::Warning;
                lines.push(format!(
                    "{torque}は含めていません（この天体には{cause}があるが、そのモデルが無い。0 ではなく、結果は過小評価になりえます）"
                ));
            }
        };
        if self.aero.is_some() {
            note("空力トルク", "大気", central.atmosphere_model());
        }
        if self.residual_dipole().norm() > 0.0 {
            note("磁気トルク", "磁場", central.magnetic_field_model());
        }
        (!lines.is_empty()).then(|| {
            Check::new(
                "environment_models",
                "大気・磁場のモデル",
                verdict,
                lines.join("。"),
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
                cp_offset: None,
                coefficient_area: Qty::from_raw(0.1),
            }),
            srp: None,
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
        let c = sc.environment_check(CentralBody::Moon).expect("注記");
        assert_eq!(c.verdict, Verdict::Ok);
        assert_eq!(
            c.message,
            "空力トルクはこの天体では 0（大気が無い）。磁気トルクはこの天体では 0（磁場が無い）"
        );
        assert!(sc.environment_check(CentralBody::Earth).is_none());
    }

    /// 火星は大気があるのにモデルが無いので「含めていない」と `Warning`（0 とは言わない）。磁場は無いので 0 が正しい。
    /// 木星は磁場があるのにモデルが無い。
    #[test]
    fn missing_models_warn_instead_of_claiming_zero() {
        let sc = spacecraft(true, true);
        let mars = sc.environment_check(CentralBody::Mars).expect("注記");
        assert_eq!(mars.verdict, Verdict::Warning);
        assert!(
            mars.message.starts_with(
                "空力トルクは含めていません（この天体には大気があるが、そのモデルが無い"
            )
        );
        assert!(
            mars.message
                .ends_with("磁気トルクはこの天体では 0（磁場が無い）")
        );
        let jupiter = sc.environment_check(CentralBody::Jupiter).expect("注記");
        assert_eq!(jupiter.verdict, Verdict::Warning);
        assert!(
            jupiter
                .message
                .contains("磁気トルクは含めていません（この天体には磁場があるが")
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
            .expect("注記");
        assert_eq!(c.message, "空力トルクはこの天体では 0（大気が無い）");
    }
}
