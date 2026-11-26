//! マヌーバごとの前後の軌道と、方式ごとの理想の ΔV（質量によらない部分）。遷移の式は `sat_astro::transfer`。
//!
//! - 軌道は近地点・遠地点の半径と傾斜角で持つ（近点線は揃っているとみなす。RAAN の変化は扱わない）。
//! - 目標の軌道は、書いた要素だけを前の軌道から変える（`altitude` は近地点と遠地点の両方）。
//! - インパルス・有限噴射: 2 回噴射の遷移（中間の遠地点を書けばバイエリプティック）。平面変更は遠い方の噴射に合わせる。
//! - 低推力: Edelbaum の式（円軌道どうし。円でなければ長半径の円で近似したと注記する）。
//! - 目標に別の天体（`target.body`）を書いたとき: パッチドコニックの遷移（`sat_astro::transfer::patched_conic`。
//!   出発の軌道の近点で脱出し、目標の天体を回る軌道の近点で捕獲する）。軌道は中心天体ごと持つ。
//!   `transfer`（出発・到着の日）があれば、遷移は両端の天体のその日の位置からの Lambert（`sat_astro::transfer::dated_transfer`。
//!   噴射は同じく軌道の近点。結果は [`super::dated`]）。途中のスイングバイ（`transfer.flybys`）は、前後の脚をつなぎ、通過ごとの近点の噴射を
//!   出発と到着の噴射の間に足す（`sat_astro::transfer::dated_transfer_via`。動力なしと宣言した通過は数えない）。
//! - 数値伝播: 最初は低推力と同じ見積もり。伝播の結果（[`Ideal::Propagated`]）で `sequence` が置き換える。

use super::input::{
    DatedTransferInput, DepartureProvider, ManeuverInput, OrbitTarget, TransferMethod,
};
use crate::spec::AnalysisError;
use sat_astro::ephem::CentralBody;
use sat_astro::orbit::OrbitSpec;
use sat_astro::transfer::{
    Apsides, DatedTransfer, FlybyStop, ImpulsiveBurn, ImpulsiveTransfer, NEGLIGIBLE_DV_M_S,
    PatchedBurn, apse_transfer, bi_elliptic, dated_transfer_via, edelbaum, patched_conic,
};
use sat_units::typed::{Length, Qty};
use serde::{Deserialize, Serialize};

/// 離心率がこれを超える軌道は、低推力の円軌道の近似から外れるとみなす。
const CIRCULAR_ECCENTRICITY: f64 = 1e-3;

/// マヌーバの計算の注記。**文は持たない**（service の語彙が引く）。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(rename_all = "snake_case")]
pub enum ManeuverNote {
    /// 中間の遠地点を通るバイエリプティック遷移で計算した
    BiElliptic,
    /// 低推力では中間の遠地点を使わない
    ViaAltitudeIgnored,
    /// 低推力の Edelbaum の式は円軌道どうしの式なので、長半径の円軌道で近似した
    LowThrustCircularApproximation,
    /// スラスタが無いので ΔV だけを求め、推薬・噴射時間は求めない
    NoThruster,
    /// 有限噴射の噴射が軌道の半周を超えるので、損失を見込まずインパルスの ΔV を使った
    FiniteBurnTooLong,
    /// 別の天体へのパッチドコニックの遷移（天体の円軌道どうしのホーマン遷移。打ち上げの窓は見ない）で計算した
    PatchedConic,
    /// 出発の日・到着の日の天体の位置からの Lambert の遷移（日付つきのパッチドコニック）で計算した
    DatedLambert,
    /// 途中の惑星のスイングバイをパッチドコニックで計算した（通過は一瞬。影響圏の中の時間・双曲線の弧は扱わない）
    SwingBy,
    /// 出発の噴射を打ち上げ機が行うので、衛星の ΔV・推薬に数えない（C3・超過速度は打ち上げの要求として出す）
    DepartureByLauncher,
    /// 天体間の遷移では軌道面の変更（傾斜角）の ΔV を数えない
    PlaneChangeIgnored,
    /// 数値伝播が上限の時間までに目標に届かなかった（届いたところまでの値）
    PropagationNotReached,
    /// ΔV は軌道の計算でなく、設計データに直接書かれた値（軌道は変わらない）
    WrittenDeltaV,
}

/// 軌道（中心天体・近地点と遠地点の半径・傾斜角）。
#[derive(Debug, Clone, Copy, PartialEq)]
pub(super) struct Orbit {
    pub central: CentralBody,
    pub apsides: Apsides,
    pub inclination_rad: f64,
}

impl Orbit {
    /// 軌道の仕様の元期の要素から（`a(1 ∓ e)`）。
    pub fn of(spec: &OrbitSpec) -> Result<Self, AnalysisError> {
        let el = spec.kepler_elements()?;
        Ok(Self {
            central: spec.central,
            apsides: Apsides::new(el.a_m * (1.0 - el.e), el.a_m * (1.0 + el.e)),
            inclination_rad: el.i_rad,
        })
    }

    /// 中心天体と長半径・離心率・傾斜角から（伝播の終わりの平均の長半径など）。
    pub fn from_elements(central: CentralBody, a_m: f64, e: f64, inclination_rad: f64) -> Self {
        Self {
            central,
            apsides: Apsides::new(a_m * (1.0 - e), a_m * (1.0 + e)),
            inclination_rad,
        }
    }

    pub(super) fn eccentricity(&self) -> f64 {
        let (p, a) = (self.apsides.periapsis_m, self.apsides.apoapsis_m);
        (a - p) / (a + p)
    }

    /// 目標の要素を当てた軌道。
    fn with_target(self, t: &OrbitTarget) -> Self {
        let r = |h: Qty<Length>| self.central.radius_m() + h.value();
        let (mut rp, mut ra) = (self.apsides.periapsis_m, self.apsides.apoapsis_m);
        if let Some(h) = t.altitude {
            (rp, ra) = (r(h), r(h));
        }
        if let Some(h) = t.perigee_altitude {
            rp = r(h);
        }
        if let Some(h) = t.apogee_altitude {
            ra = r(h);
        }
        Self {
            central: self.central,
            apsides: Apsides::new(rp, ra),
            inclination_rad: t.inclination.map_or(self.inclination_rad, |i| i.value()),
        }
    }
}

/// 理想の ΔV（損失・質量によらない部分）。
#[derive(Debug, Clone, PartialEq)]
pub(super) enum Ideal {
    /// インパルスの噴射の列（有限噴射はこれに損失を足す）
    Impulsive(ImpulsiveTransfer),
    /// 低推力の連続噴射の ΔV [m/s]
    LowThrust(f64),
    /// 推力を入れた数値伝播で求めた ΔV [m/s] と遷移の時間 [s]
    Propagated { delta_v_m_s: f64, duration_s: f64 },
}

/// 噴射する場所（中心天体と、双曲線の超過速度）。
#[derive(Debug, Clone, Copy, PartialEq)]
pub(super) struct BurnSite {
    pub central: CentralBody,
    /// 双曲線超過速度 [m/s]（天体間の遷移の脱出・捕獲だけ）
    pub v_inf_m_s: Option<f64>,
}

/// 1 つのマヌーバの前後の軌道と理想の ΔV。
#[derive(Debug, Clone, PartialEq)]
pub(super) struct Planned {
    pub from: Orbit,
    pub to: Orbit,
    pub ideal: Ideal,
    /// 噴射ごとの場所（インパルスは `ideal` の噴射と同じ順、低推力は 1 つ）
    pub sites: Vec<BurnSite>,
    pub notes: Vec<ManeuverNote>,
    /// 日付つきの遷移の結果（`transfer` を書いたときだけ）
    pub dated: Option<DatedTransfer>,
    /// 出発の噴射を打ち上げ機が行う（`ideal` の噴射の列と `sites` から出発の噴射を除いてある。C3・超過速度は `dated` に残る）
    pub launcher_departure: bool,
    /// `ideal` の噴射の列のうち、途中のスイングバイの噴射の範囲（通過がなければ空。飛行の途中の噴射なので、有限噴射の遷移の時間に半分ずつ足さない）
    pub mid_burns: std::ops::Range<usize>,
}

impl Planned {
    /// `i` 番目の噴射の場所（無ければ出発の軌道の中心天体）。
    pub fn site(&self, i: usize) -> BurnSite {
        self.sites.get(i).copied().unwrap_or(BurnSite {
            central: self.from.central,
            v_inf_m_s: None,
        })
    }
}

/// 軌道 `from` からマヌーバ `m` を行う計画。
pub(super) fn plan(from: Orbit, m: &ManeuverInput) -> Result<Planned, AnalysisError> {
    if let Some(dv) = m.delta_v {
        return written(from, dv.value(), m);
    }
    if let Some(body) = m.target.body.filter(|b| *b != from.central) {
        return interbody(from, body, m);
    }
    if m.transfer.is_some() {
        return Err(AnalysisError::InvalidInput(format!(
            "Maneuver {}: a dated transfer needs a target body different from the current central body",
            m.id
        )));
    }
    let central = from.central;
    let to = from.with_target(&m.target);
    let angle = (to.inclination_rad - from.inclination_rad).abs();
    let mu = central.gm();
    let mut notes = Vec::new();
    let ideal = match m.method {
        TransferMethod::Impulsive | TransferMethod::FiniteBurn => match m.via_altitude {
            Some(h) => {
                notes.push(ManeuverNote::BiElliptic);
                let via = central.radius_m() + h.value();
                Ideal::Impulsive(bi_elliptic(mu, from.apsides, to.apsides, via, angle)?)
            }
            None => Ideal::Impulsive(apse_transfer(mu, from.apsides, to.apsides, angle)?),
        },
        // 数値伝播も最初は Edelbaum で見積もる（質量の見積もりに使い、伝播の結果で置き換える。`sequence`）
        TransferMethod::LowThrust | TransferMethod::Propagated => {
            if m.via_altitude.is_some() {
                notes.push(ManeuverNote::ViaAltitudeIgnored);
            }
            if from.eccentricity() > CIRCULAR_ECCENTRICITY
                || to.eccentricity() > CIRCULAR_ECCENTRICITY
            {
                notes.push(ManeuverNote::LowThrustCircularApproximation);
            }
            Ideal::LowThrust(edelbaum(
                mu,
                from.apsides.semi_major_axis_m(),
                to.apsides.semi_major_axis_m(),
                angle,
            )?)
        }
    };
    Ok(Planned {
        from,
        to,
        ideal,
        sites: Vec::new(),
        notes,
        dated: None,
        launcher_departure: false,
        mid_burns: 0..0,
    })
}

/// 直接書いた ΔV（余裕）。軌道は変わらず、ΔV はそのまま理想の ΔV にする（噴射の位置は今の軌道の長半径。
/// インパルスは 1 回の噴射、低推力は連続噴射）。軌道の計算（`target` ・`via_altitude`・`transfer`）とは一緒に使えない。
fn written(from: Orbit, delta_v_m_s: f64, m: &ManeuverInput) -> Result<Planned, AnalysisError> {
    let invalid = |why: &str| AnalysisError::InvalidInput(format!("Maneuver {}: {why}", m.id));
    if m.transfer.is_some() || m.via_altitude.is_some() || m.target.body.is_some() {
        return Err(invalid(
            "A written delta_v cannot be combined with a target body, via_altitude or a dated transfer",
        ));
    }
    if !(delta_v_m_s >= 0.0 && delta_v_m_s.is_finite()) {
        return Err(invalid(
            "A written delta_v must be a finite value of 0 or more",
        ));
    }
    let ideal = match m.method {
        TransferMethod::Impulsive | TransferMethod::FiniteBurn => {
            Ideal::Impulsive(ImpulsiveTransfer {
                burns: vec![ImpulsiveBurn {
                    radius_m: from.apsides.semi_major_axis_m(),
                    delta_v_m_s,
                }],
                coast_s: 0.0,
            })
        }
        TransferMethod::LowThrust => Ideal::LowThrust(delta_v_m_s),
        TransferMethod::Propagated => {
            return Err(invalid(
                "A written delta_v is used with impulsive or low_thrust (propagated needs an orbit change)",
            ));
        }
    };
    Ok(Planned {
        from,
        to: from,
        ideal,
        sites: Vec::new(),
        notes: vec![ManeuverNote::WrittenDeltaV],
        dated: None,
        launcher_departure: false,
        mid_burns: 0..0,
    })
}

/// 別の天体 `body` を回る軌道へのパッチドコニックの遷移。捕獲の軌道は `altitude`（円）か
/// `perigee_altitude`（と `apogee_altitude`）で、高度は `body` の半径から数える。
fn interbody(from: Orbit, body: CentralBody, m: &ManeuverInput) -> Result<Planned, AnalysisError> {
    let invalid = |why: &str| AnalysisError::InvalidInput(format!("Maneuver {}: {why}", m.id));
    if matches!(
        m.method,
        TransferMethod::LowThrust | TransferMethod::Propagated
    ) {
        return Err(invalid(
            "Transfers between bodies are computed with impulsive or finite_burn (low_thrust and propagated are not supported)",
        ));
    }
    let t = &m.target;
    let r = |h: Qty<Length>| body.radius_m() + h.value();
    let (rp, ra) = match (t.altitude, t.perigee_altitude) {
        (Some(h), _) => (r(h), r(h)),
        (None, Some(p)) => (r(p), t.apogee_altitude.map_or(r(p), r)),
        (None, None) => {
            return Err(invalid(
                "Specify an orbit around the target body (altitude or perigee_altitude)",
            ));
        }
    };
    let capture = Apsides::new(rp, ra);
    let (departure, arrival, coast_s, dated) = match &m.transfer {
        Some(t) => {
            let d = dated_transfer_via(
                from.central,
                from.apsides,
                body,
                Some(capture),
                t.departure,
                t.arrival,
                &flyby_stops(t),
            )?;
            (d.departure, d.arrival, d.transfer_time_s, Some(d))
        }
        None => {
            let pc = patched_conic(from.central, from.apsides, body, Some(capture))?;
            (pc.departure, pc.arrival, pc.transfer_time_s, None)
        }
    };
    let mut notes = vec![if dated.is_some() {
        ManeuverNote::DatedLambert
    } else {
        ManeuverNote::PatchedConic
    }];
    if dated.as_ref().is_some_and(|d| !d.flybys.is_empty()) {
        notes.push(ManeuverNote::SwingBy);
    }
    if m.via_altitude.is_some() {
        notes.push(ManeuverNote::ViaAltitudeIgnored);
    }
    if t.inclination.is_some() {
        notes.push(ManeuverNote::PlaneChangeIgnored);
    }
    let launcher_departure = dated.is_some()
        && m.transfer
            .as_ref()
            .is_some_and(|t| t.departure_by == DepartureProvider::Launcher);
    if launcher_departure {
        notes.push(ManeuverNote::DepartureByLauncher);
    }
    let first = (!launcher_departure).then(|| burn_of(departure));
    let mid = dated
        .as_ref()
        .map(|d| flyby_burns(d, m.transfer.as_ref()))
        .unwrap_or_default();
    let mid_start = usize::from(first.is_some());
    let mid_burns = mid_start..mid_start + mid.len();
    let (burns, sites): (Vec<_>, Vec<_>) = first
        .into_iter()
        .chain(mid)
        .chain(arrival.map(burn_of))
        .unzip();
    Ok(Planned {
        from,
        to: Orbit {
            central: body,
            apsides: capture,
            inclination_rad: t.inclination.map_or(from.inclination_rad, |i| i.value()),
        },
        ideal: Ideal::Impulsive(ImpulsiveTransfer { burns, coast_s }),
        sites,
        notes,
        dated,
        launcher_departure,
        mid_burns,
    })
}

/// 噴射 1 回の（噴射・場所）。
fn burn_of(b: PatchedBurn) -> (ImpulsiveBurn, BurnSite) {
    let burn = ImpulsiveBurn {
        radius_m: b.radius_m,
        delta_v_m_s: b.delta_v_m_s,
    };
    let site = BurnSite {
        central: b.central,
        v_inf_m_s: b.v_inf_m_s,
    };
    (burn, site)
}

/// 日付つきの遷移の入力の通過 → `sat_astro` の通過（日付の順のまま）。
fn flyby_stops(t: &DatedTransferInput) -> Vec<FlybyStop> {
    t.flybys
        .iter()
        .map(|f| FlybyStop {
            body: f.body,
            date: f.date,
            min_altitude_m: f.min_altitude.map(|h| h.value()),
        })
        .collect()
}

/// 通過ごとの近点の噴射（動力ありと書かれ、ΔV が無視できる量を超えるものだけ。噴射の位置は近点の半径、近点が無限大なら下限の半径）。
/// 噴射する場所は通過する天体で、超過速度は入りの大きさ。
fn flyby_burns(
    d: &DatedTransfer,
    t: Option<&DatedTransferInput>,
) -> Vec<(ImpulsiveBurn, BurnSite)> {
    let powered = |i: usize| t.and_then(|t| t.flybys.get(i)).is_none_or(|f| f.powered);
    d.flybys
        .iter()
        .enumerate()
        .filter(|(i, f)| powered(*i) && f.total_dv_m_s() > NEGLIGIBLE_DV_M_S)
        .map(|(_, f)| {
            let burn = ImpulsiveBurn {
                radius_m: f.periapsis_radius_m.unwrap_or(f.min_radius_m),
                delta_v_m_s: f.total_dv_m_s(),
            };
            let site = BurnSite {
                central: f.body,
                v_inf_m_s: Some(f.v_inf_in_m_s.norm()),
            };
            (burn, site)
        })
        .collect()
}
