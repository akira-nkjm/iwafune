//! 未送信データの区分線形の時間発展（`comm.data_throughput` の中だけで使う）。

/// 未送信データの時間発展（区分的に線形なので、頂点だけを記録する）。
pub(super) struct Backlog {
    pub t: f64,
    pub value: f64,
    pub gen_bps: f64,
    pub points: Vec<(f64, f64)>,
}

impl Backlog {
    pub fn idle(&mut self, to: f64) {
        self.value += self.gen_bps * (to - self.t).max(0.0);
        self.t = to.max(self.t);
        self.points.push((self.t, self.value));
    }

    /// `to` まで送信する。送った量を返す。
    pub fn downlink(&mut self, to: f64, rate_bps: f64) -> f64 {
        let dt = (to - self.t).max(0.0);
        let net = rate_bps - self.gen_bps;
        if net > 0.0 && self.value / net < dt {
            // 途中で空になる点を頂点として残す。
            self.points.push((self.t + self.value / net, 0.0));
        }
        let sent = (rate_bps * dt).min(self.value + self.gen_bps * dt);
        self.value += self.gen_bps * dt - sent;
        self.t += dt;
        self.points.push((self.t, self.value));
        sent
    }
}
