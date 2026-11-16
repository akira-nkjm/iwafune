//! 中心天体の磁場で MTQ を評価できるか（`aocs.unload_time` の `mtq_field_gap`）。

use super::UnloadTimeField;
use crate::spec::Verdict;
use sat_astro::body::EnvironmentModel;
use sat_astro::ephem::CentralBody;
use serde::{Deserialize, Serialize};

/// 中心天体の磁場で MTQ を評価できなかったときの判定のキー（service が [`MtqFieldGap`] からこの判定を作る）。
pub const MTQ_FIELD_MODEL_CHECK: &str = "mtq_field_model";

/// 中心天体の磁場で MTQ を評価できなかった理由（出力の `mtq_field_gap`）。**文は持たない**: service が判定
/// （key [`MTQ_FIELD_MODEL_CHECK`]、判定は [`MtqFieldGap::verdict`]）にし、語彙から日本語と英語の文を引く。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(rename_all = "snake_case")]
pub enum MtqFieldGap {
    /// 中心天体に全球の磁場が無い（月・金星・火星。MTQ では角運動量を捨てられない）
    Absent,
    /// 磁場はあるがモデルが無い（地球の磁場だけを扱う。MTQ を評価できない）
    NotModelled,
}

impl MtqFieldGap {
    /// 中心天体の磁場のモデルから（モデルがあれば `None`）。
    pub fn of(central: CentralBody) -> Option<Self> {
        match central.magnetic_field_model() {
            EnvironmentModel::Modelled => None,
            EnvironmentModel::Absent => Some(Self::Absent),
            EnvironmentModel::NotModelled => Some(Self::NotModelled),
        }
    }

    /// 判定: 磁場が無ければ MTQ では捨てられないので違反、モデルが無いだけなら評価できない注意。
    pub fn verdict(self) -> Verdict {
        match self {
            Self::Absent => Verdict::Violation,
            Self::NotModelled => Verdict::Warning,
        }
    }
}

/// 軌道から磁場を求めるときに、中心天体の磁場で MTQ を評価できない理由（評価できれば `None`）。
/// 磁場のモデルが無い天体で 0 として計算し「MTQ のトルクが 0」と誤解させないため、MTQ の評価を外す。
pub(super) fn field_gap(field: Option<&UnloadTimeField>) -> Option<MtqFieldGap> {
    match field? {
        UnloadTimeField::Orbit { sampling } => MtqFieldGap::of(sampling.orbit.central),
        UnloadTimeField::Constant { .. } => None,
    }
}
