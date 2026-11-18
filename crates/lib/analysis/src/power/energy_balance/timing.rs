//! 日照・食に対する時刻の指定: モードの使う時間帯（`when`）・開始の基準（`start`）・並びの基準（`anchor`）と、
//! 最長継続時間の評価でずらす開始の選び方。**開始時刻の候補を決める規則はここだけ**（[`super::duration`] は選ばれた開始を評価する）。
//!
//! 食の出入りは標本の刻みに丸める（食明け = 日陰の次の最初の日照の標本、食入り = 日照の次の最初の日陰の標本の時刻）。
//!
//! 最長継続時間の評価の開始の決め方（[`select_starts`]）:
//! - `start` があれば、その基準の時刻 + オフセットの標本だけ（[`DurationTiming::Fixed`]）。
//! - `when` だけなら、窓（最長継続時間の間）がその時間帯にまるごと入る開始の範囲でずらす（[`DurationTiming::Windowed`]）。
//! - どちらも無ければ 1 周の中で開始を標本ごとにずらして最悪を取る（[`DurationTiming::Unspecified`]）。
//!
//! 指定どおりの開始が作れないとき（食が無い・窓が時間帯に収まらない）は、指定を外した探索に戻して `note` に理由を返す。

use crate::spec::quantity_text;
use sat_units::typed::{Qty, Time};
use serde::{Deserialize, Serialize};

/// モードを使う時間帯。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(rename_all = "snake_case")]
pub enum TimeWindow {
    /// 時間帯の指定なし（日照・日陰のどちらにも置ける）
    #[default]
    Any,
    /// 日照中だけ
    Sunlit,
    /// 日陰中（食）だけ
    Eclipse,
}

/// 開始の基準にする食の出入り。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(rename_all = "snake_case")]
pub enum EclipseEvent {
    /// 食明け（日陰から日照に戻る時刻）
    EclipseExit,
    /// 食入り（日照から日陰に入る時刻）
    EclipseEntry,
}

/// 食の出入りからの開始の指定（基準の時刻 + `offset`。負なら前）。
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct EventStart {
    pub from: EclipseEvent,
    pub offset: Qty<Time>,
}

/// モードの時刻の指定（最長継続時間の評価が使う）。どちらも無ければ指定なし。
#[derive(Debug, Clone, Copy, PartialEq, Default, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct ModeTiming {
    #[serde(default)]
    pub when: TimeWindow,
    #[serde(default)]
    pub start: Option<EventStart>,
}

impl ModeTiming {
    /// 時刻の指定が何も無いか。
    pub fn is_unspecified(&self) -> bool {
        self.when == TimeWindow::Any && self.start.is_none()
    }
}

/// モードの並びの名前と基準（並びの先頭をどこに置くか）。
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct SequenceTiming {
    /// 使う並びの名前（出力に載せる。無名の並びは None）
    #[serde(default)]
    pub name: Option<String>,
    /// 並びの先頭の位置。無ければ解析の開始
    #[serde(default)]
    pub anchor: Option<EventStart>,
}

/// 最長継続時間の評価で開始をどう決めたか。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(rename_all = "snake_case")]
pub enum DurationTiming {
    /// 開始の基準が決まっていて、そこだけで評価した
    Fixed,
    /// 使う時間帯（`when`）だけが決まっていて、許された範囲の中でずらした
    Windowed,
    /// タイミングの指定が無いので、1 周の中でずらして最悪を取った
    #[default]
    Unspecified,
}

/// 食の出入りの標本の番号（時刻の順）。
pub(super) fn event_samples(lit: &[bool], from: EclipseEvent) -> Vec<usize> {
    (1..lit.len())
        .filter(|&k| match from {
            EclipseEvent::EclipseExit => lit[k] && !lit[k - 1],
            EclipseEvent::EclipseEntry => !lit[k] && lit[k - 1],
        })
        .collect()
}

/// 基準の時刻 + オフセット（標本の列の外でもよい）。基準になる出入りが無ければ None。
/// 複数あるときは解析の始めに最も近い（最初の）出入り。
pub(super) fn anchor_time(t: &[f64], lit: &[bool], r: &EventStart) -> Option<f64> {
    let k = *event_samples(lit, r.from).first()?;
    Some(t[k] + r.offset.value())
}

/// 並びの先頭の時刻。基準が無い（None）なら解析の始め。基準の出入りが標本に無いときは、解析の始めにして理由を返す。
pub(super) fn sequence_origin(
    t: &[f64],
    lit: &[bool],
    anchor: Option<&EventStart>,
) -> (f64, Option<String>) {
    let t0 = t.first().copied().unwrap_or(0.0);
    let Some(r) = anchor else {
        return (t0, None);
    };
    match anchor_time(t, lit, r) {
        Some(origin) => (origin, None),
        None => (
            t0,
            Some(format!(
                "The sequence anchor ({}) does not occur in the analysis span, so the sequence starts at the beginning of the span",
                anchor_name(r.from)
            )),
        ),
    }
}

/// 基準の名前（判定の文に入れる）。
pub(super) fn anchor_name(from: EclipseEvent) -> &'static str {
    match from {
        EclipseEvent::EclipseExit => "eclipse exit",
        EclipseEvent::EclipseEntry => "eclipse entry",
    }
}

/// ずらす開始の標本（1 周分）。最初の標本は窓の前の刻みが無く窓が半刻み短くなるので、標本が 2 つ以上あれば開始にしない。
/// 解析期間の終わりから「周期 + 最長継続時間 + 周期」前から始まる 1 周（前は基本のモードで定常に近づき、後ろは 1 周の回復を見る）。
/// 解析期間が足りなければ期間の始めから。
pub(super) fn start_samples(t: &[f64], period_s: f64, duration: f64) -> Vec<usize> {
    let (t0, end) = (t[0], t[t.len() - 1]);
    let valid = period_s.is_finite() && period_s > 0.0;
    let from = if valid {
        (end - duration - 2.0 * period_s).max(t0)
    } else {
        t0
    };
    let tol = 1e-9 * period_s.abs().max(1.0);
    let first = usize::from(t.len() > 1);
    (first..t.len())
        .filter(|&k| t[k] >= from - tol && (!valid || t[k] < from + period_s - tol))
        .collect()
}

/// 選んだ開始と、その決め方。
pub(super) struct Selection {
    pub starts: Vec<usize>,
    pub timing: DurationTiming,
    /// 指定どおりにできなかった理由（判定の警告にする）
    pub note: Option<String>,
}

/// 窓（開始 `k` から `duration`）の標本がすべて時間帯 `when` に入るか。
fn window_fits(t: &[f64], lit: &[bool], when: TimeWindow, k: usize, duration: f64) -> bool {
    let want = when == TimeWindow::Sunlit;
    (k..t.len())
        .take_while(|&j| t[j] < t[k] + duration)
        .all(|j| lit[j] == want)
}

/// 最長継続時間の評価でずらす開始を、モードの時刻の指定に従って選ぶ。
pub(super) fn select_starts(
    t: &[f64],
    lit: &[bool],
    period_s: f64,
    duration: f64,
    timing: &ModeTiming,
) -> Selection {
    let all = start_samples(t, period_s, duration);
    let unspecified = |note: Option<String>| Selection {
        starts: all.clone(),
        timing: DurationTiming::Unspecified,
        note,
    };
    if let Some(r) = &timing.start {
        return fixed(t, lit, &all, r, timing.when).unwrap_or_else(|| {
            unspecified(Some(format!(
                "The start reference ({}) does not occur in the swept orbit, so the start is searched over the whole orbit",
                anchor_name(r.from)
            )))
        });
    }
    if timing.when == TimeWindow::Any {
        return unspecified(None);
    }
    let fits: Vec<usize> = all
        .iter()
        .copied()
        .filter(|&k| window_fits(t, lit, timing.when, k, duration))
        .collect();
    if fits.is_empty() {
        let note = format!(
            "The longest duration ({}) does not fit in any {} window of the orbit, so the start is searched over the whole orbit",
            quantity_text::<Time>(duration),
            if timing.when == TimeWindow::Sunlit {
                "sunlit"
            } else {
                "eclipse"
            }
        );
        return unspecified(Some(note));
    }
    Selection {
        starts: fits,
        timing: DurationTiming::Windowed,
        note: None,
    }
}

/// 開始の基準が決まっているときの候補（基準の時刻 + オフセット以後の最初の標本）。
/// 時間帯（`when`）も指定されていて、その開始が合わなければ警告を付ける。基準の出入りが 1 周に無ければ None。
fn fixed(
    t: &[f64],
    lit: &[bool],
    all: &[usize],
    r: &EventStart,
    when: TimeWindow,
) -> Option<Selection> {
    let tol = 1e-9 * t[t.len() - 1].abs().max(1.0);
    let mut starts: Vec<usize> = event_samples(lit, r.from)
        .into_iter()
        .filter_map(|e| {
            // 基準の時刻 + オフセット以後の最初の標本。ずらす 1 周の外に丸められるものは含めない
            let target = t[e] + r.offset.value();
            let k = t.partition_point(|&x| x < target - tol);
            all.contains(&k).then_some(k)
        })
        .collect();
    starts.dedup();
    if starts.is_empty() {
        return None;
    }
    let note = (when != TimeWindow::Any
        && !starts
            .iter()
            .all(|&k| lit[k] == (when == TimeWindow::Sunlit)))
    .then(|| {
        format!(
            "The fixed start is not inside the {} window of the mode",
            if when == TimeWindow::Sunlit {
                "sunlit"
            } else {
                "eclipse"
            }
        )
    });
    Some(Selection {
        starts,
        timing: DurationTiming::Fixed,
        note,
    })
}

#[cfg(test)]
mod tests;
