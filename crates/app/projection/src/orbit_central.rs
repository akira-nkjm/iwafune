//! 設計の中心天体の読み替えと、設計の軌道の中心天体。**葉のモジュール**（`context` と `maneuver_dates` の両方が使うので、
//! どちらにも依存しない）。設計の綴り（`CentralBodyKind`）→ sat-astro の中心天体は [`central_body`] だけ。
//! 設計の軌道（`conditions/orbit.toml`）の設定を読むのは [`design_orbit`] と `Context::orbit_settings` だけ。

use fune_model::Design;
use fune_model::design::{CentralBodyKind, OrbitSettings, OrbitSource};
use sat_astro::ephem::CentralBody;

/// 設計の中心天体 → sat-astro の中心天体（μ・半径・暦・環境の値は sat-astro と解析が持つ）。
pub fn central_body(c: CentralBodyKind) -> CentralBody {
    match c {
        CentralBodyKind::Earth => CentralBody::Earth,
        CentralBodyKind::Moon => CentralBody::Moon,
        CentralBodyKind::Sun => CentralBody::Sun,
        CentralBodyKind::Mercury => CentralBody::Mercury,
        CentralBodyKind::Venus => CentralBody::Venus,
        CentralBodyKind::Mars => CentralBody::Mars,
        CentralBodyKind::Jupiter => CentralBody::Jupiter,
        CentralBodyKind::Saturn => CentralBody::Saturn,
        CentralBodyKind::Uranus => CentralBody::Uranus,
        CentralBodyKind::Neptune => CentralBody::Neptune,
    }
}

/// 設計の軌道の設定（`conditions/orbit.toml`）。ケースのフェーズを見ない。
pub(crate) fn design_orbit(design: &Design) -> Option<&OrbitSettings> {
    design.conditions.orbit.as_ref()
}

/// マヌーバの列の始めの軌道の中心天体（ケースを選ばない設計の軌道の中心。`maneuver_dates::central_before` の出発点）。
/// ケプラー要素の軌道と状態ベクトルの軌道は `central`（省略は地球）。日付つきの遷移の巡航の軌道（`transfer`）は、出発の天体を何も語らない
/// （巡航は親のまわりの軌道で、出発の惑星のまわりの軌道ではない。循環もさせない）ので既定の天体（地球）とし、
/// マヌーバの `after` の列が替える。
pub(crate) fn design_orbit_central(design: &Design) -> CentralBody {
    match design_orbit(design) {
        Some(o) if matches!(o.source_kind(), OrbitSource::Elements | OrbitSource::State) => {
            o.central.map(central_body).unwrap_or_default()
        }
        _ => CentralBody::default(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 中心天体の綴り（fune-model）と sat-astro の綴りが一致する。
    #[test]
    fn central_body_spellings_match_sat_astro() {
        for &kind in CentralBodyKind::ALL {
            let model = serde_json::to_value(kind).unwrap();
            assert_eq!(model, serde_json::to_value(central_body(kind)).unwrap());
        }
    }
}
