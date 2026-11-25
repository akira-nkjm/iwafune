//! 日付の探索の格子の値（出発 × 到着の順）と、解いた点の集計（最小の点・解けた数・暦の出所）。

use super::cell::CellValues;
use super::output::{LaunchWindowLayer, LaunchWindowQuantity};
use sat_astro::ephem::EphemerisSource;
use sat_astro::time::Epoch;
use sat_units::typed::{Angle, NamedKind, SpecificEnergy, Velocity};

/// 格子の値（出発 × 到着の順）と、解いた点の集計。
pub(super) struct Grid {
    pub n_arr: usize,
    pub c3: Vec<Option<f64>>,
    pub v_dep: Vec<Option<f64>>,
    pub v_arr: Vec<Option<f64>>,
    pub declination: Vec<Option<f64>>,
    pub delta_v: Vec<Option<f64>>,
    /// 点ごとの通過の日（通過のある探索だけ。直接の遷移は空の列）
    via: Vec<Vec<Epoch>>,
    /// 点ごとの選ぶ基準（[`CellValues::objective_m_s`]。最小の点と極小の選び方に使う）
    objective: Vec<Option<f64>>,
    pub evaluated: u32,
    pub solved: u32,
    pub all_de440: bool,
    /// ΔV 最小・C3 最小の点（添字と値。同じ値なら先に解いた点）
    pub best_dv: Option<((usize, usize), CellValues)>,
    pub best_c3: Option<((usize, usize), CellValues)>,
}

impl Grid {
    pub fn new(n_dep: usize, n_arr: usize) -> Self {
        let blank = || vec![None; n_dep * n_arr];
        Self {
            n_arr,
            c3: blank(),
            v_dep: blank(),
            v_arr: blank(),
            declination: blank(),
            delta_v: blank(),
            via: vec![Vec::new(); n_dep * n_arr],
            objective: blank(),
            evaluated: 0,
            solved: 0,
            all_de440: true,
            best_dv: None,
            best_c3: None,
        }
    }

    pub fn record(&mut self, (i, j): (usize, usize), v: CellValues) {
        let k = i * self.n_arr + j;
        self.c3[k] = Some(v.c3_m2_s2);
        self.v_dep[k] = Some(v.v_inf_departure_m_s);
        self.v_arr[k] = Some(v.v_inf_arrival_m_s);
        self.declination[k] = Some(v.declination_rad);
        self.delta_v[k] = Some(v.delta_v_m_s);
        self.objective[k] = Some(v.objective_m_s);
        self.via[k].clone_from(&v.flyby_dates);
        self.solved += 1;
        self.all_de440 &= v.ephemeris == EphemerisSource::De440;
        if self
            .best_dv
            .as_ref()
            .is_none_or(|(_, b)| v.objective_m_s < b.objective_m_s)
        {
            self.best_dv = Some(((i, j), v.clone()));
        }
        if self
            .best_c3
            .as_ref()
            .is_none_or(|(_, b)| v.c3_m2_s2 < b.c3_m2_s2)
        {
            self.best_c3 = Some(((i, j), v));
        }
    }

    /// 出発 `i`・到着 `j` の 4 近傍のどれかが格子の外か、解けていない点か。
    pub fn on_edge(&self, (i, j): (usize, usize), n_dep: usize) -> bool {
        let solved = |i: Option<usize>, j: Option<usize>| match (i, j) {
            (Some(i), Some(j)) if i < n_dep && j < self.n_arr => {
                self.delta_v[i * self.n_arr + j].is_some()
            }
            _ => false,
        };
        !(solved(i.checked_sub(1), Some(j))
            && solved(Some(i + 1), Some(j))
            && solved(Some(i), j.checked_sub(1))
            && solved(Some(i), Some(j + 1)))
    }

    pub fn layers(&self) -> Vec<LaunchWindowLayer> {
        use LaunchWindowQuantity as Q;
        let layer = |kind, dimension: &str, values: &Vec<Option<f64>>| LaunchWindowLayer {
            kind,
            dimension: dimension.to_owned(),
            values: values.clone(),
        };
        vec![
            layer(Q::DeltaV, Velocity::name(), &self.delta_v),
            layer(Q::C3, SpecificEnergy::name(), &self.c3),
            layer(Q::VInfDeparture, Velocity::name(), &self.v_dep),
            layer(Q::VInfArrival, Velocity::name(), &self.v_arr),
            layer(Q::DepartureDeclination, Angle::name(), &self.declination),
        ]
    }

    /// ΔV の極小の点（8 近傍のどれよりも小さいか等しい。解けていない近傍は数えない）を、ΔV の小さい順に `n` 個。
    /// 通過の日と合わせて返す（日付は軸の添字から引く側が決める）。
    pub fn local_minima(&self, n: usize, n_dep: usize) -> Vec<((usize, usize), Vec<Epoch>)> {
        let at = |i: usize, j: usize| self.objective[i * self.n_arr + j];
        let mut found: Vec<(f64, (usize, usize))> = Vec::new();
        for i in 0..n_dep {
            for j in 0..self.n_arr {
                let Some(v) = at(i, j) else { continue };
                let is_min = (i.saturating_sub(1)..=(i + 1).min(n_dep - 1))
                    .flat_map(|a| {
                        (j.saturating_sub(1)..=(j + 1).min(self.n_arr - 1)).map(move |b| (a, b))
                    })
                    .filter(|&p| p != (i, j))
                    .all(|(a, b)| at(a, b).is_none_or(|w| v <= w));
                if is_min {
                    found.push((v, (i, j)));
                }
            }
        }
        found.sort_by(|a, b| a.0.total_cmp(&b.0).then(a.1.cmp(&b.1)));
        found
            .into_iter()
            .take(n)
            .map(|(_, (i, j))| ((i, j), self.via[i * self.n_arr + j].clone()))
            .collect()
    }
}
