//! 打ち上げ適合の固有振動数の判定。機体全体の一次の見積もりを **出どころ（[`LaunchFitSource`]）つきの 1 つの並び**で受け、
//! 向きごとに効く 1 つを選ぶ。**選ぶ規則はここだけ**（出どころを増やしても、並びに足すだけで規則は変わらない）。
//!
//! 向きごとの選び方（上から順に、最初に当てはまるもの）:
//! 1. 出どころの優先順（[`LaunchFitSource::PRIORITY`]: 有限要素 → 梁の見積もり）で、その向きの全体のモードがある出どころ。
//!    その出どころの最も低いものが効く（出どころが下限を下回っていれば違反）。
//! 2. 全体のモードが無く、**出どころがモードを探した範囲**（[`LaunchFitSearch`]。その振動数までのモードを全部求めて分類した）を持つとき:
//!    その向きの全体のモードは範囲より上にしか無い。範囲の上端が下限以上なら、一次はそれより上なので適合（見積もりは下界で、`estimated` は空）。
//!    範囲の上端が下限に届かなければ、下限と範囲の間に一次があるかもしれないので判定せず警告にする。
//! 3. どちらも無ければ判定せず警告（「確かめていない」を適合と読まれないため）。
//!
//! 2 を適合にしてよい理由: 有限要素の固有値は Sturm 列の判定で、返した最大の振動数より下の取りこぼしが無いと確かめている
//! （`sat_astro::math::lowest_modes`）。求めた全部のモードは有効質量の割合で分類済みなので、「範囲の中に全体のモードが無い」は
//! モデルの上で成り立つ。ただし全体のモードの定義は有効質量の割合（`GLOBAL_MODE_MIN_FRACTION`）による。
//!
//! 局所のモード（パネルの面外など。向きを持たない）は機体全体の条件とは比べない。下限を下回る最も低いものは、
//! 隠れないよう判定なしの情報として出力に残す。

use super::{FIT_MARGIN_WARNING_BAND, LaunchFitDirection, LaunchFitInput, positive};
use crate::spec::quantity_text;
use crate::spec::{AnalysisError, Check, Verdict, ratio_margin, ratio_margin_verdict};
use sat_units::typed::{Fraction, Frequency, Qty};
use serde::{Deserialize, Serialize};

/// 有限要素に求めてもらうモードの数（`structure.modal_fem` の `mode_count`。既定の 6 は、機体全体のモードが局所のモードより上にある機体では
/// 一次の機軸のモードに届かない）。下限を超える範囲まで探して全体のモードが無いことを言えるよう、既定の 2 倍にした。
/// 大きくするほど反復の計算が重くなる（デモで 6 → 12 でおよそ 0.1 s → 0.4 s）。
pub const FEM_MODE_COUNT: u32 = 12;

/// 固有振動数の見積もりの出どころ。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(rename_all = "snake_case")]
pub enum LaunchFitSource {
    /// 簡易固有振動数（`structure.modal_estimate`。梁とパネルの教科書の式）
    BeamEstimate,
    /// 粗い有限要素の固有値解析（`structure.modal_fem`）
    FiniteElement,
}

impl LaunchFitSource {
    /// 効かせる順（先のものが後のものに勝つ。有限要素は部品の置き方と質量を直接見るので、梁の式より先）。
    pub const PRIORITY: [Self; 2] = [Self::FiniteElement, Self::BeamEstimate];

    /// 判定の文に書く名前（`structure.modal_estimate`・`structure.modal_fem` の結果）。
    pub fn text(self) -> &'static str {
        match self {
            Self::BeamEstimate => "beam estimate",
            Self::FiniteElement => "finite-element model",
        }
    }
}

/// 固有振動数の見積もり 1 つ（機体全体の梁・全体のモード、またはパネルなどの局所のモード）。
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct LaunchFitMode {
    /// 要素の ID（梁なら部品の ID、有限要素なら `mode 3` のようなモードの番号）
    pub id: String,
    pub source: LaunchFitSource,
    /// 機体全体のモードの向き。`None` は局所のモード（機体全体の条件とは比べない）
    pub direction: Option<LaunchFitDirection>,
    pub frequency: Qty<Frequency>,
}

/// 出どころがモードを探した範囲: その振動数までのモードを、取りこぼし無く全部求めて分類した。
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct LaunchFitSearch {
    pub source: LaunchFitSource,
    /// 求めたモードの最も高い振動数
    pub up_to: Qty<Frequency>,
}

/// 見積もりに使えなかった出どころとその理由（`structure.modal_fem` が組み立てられなかったときなど）。
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct LaunchFitSourceNote {
    pub source: LaunchFitSource,
    pub reason: String,
}

/// 固有振動数の判定。
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct LaunchFitFrequency {
    pub direction: LaunchFitDirection,
    /// 効いた下限
    pub limit: Qty<Frequency>,
    /// 効いた下限の出所
    pub origin: String,
    /// 機体全体の一次の見積もり（無ければ `None`。判定は警告か、探した範囲による適合）
    pub estimated: Option<Qty<Frequency>>,
    /// 見積もりの要素の ID
    pub estimated_by: Option<String>,
    /// 見積もりの出どころ（見積もりがあるときと、探した範囲で判定したとき）
    pub source: Option<LaunchFitSource>,
    /// 出どころがこの向きの全体のモードを見つけずに探し終えた範囲の上端（そのときだけ。一次はこれより上。下限に届かないときは警告）
    pub searched_up_to: Option<Qty<Frequency>>,
    /// 余裕 = 見積もり / 下限 − 1（`searched_up_to` のときは、それを見積もりとした下界）
    pub margin: Option<Fraction>,
    /// 下限を下回る最も低い局所のモード（判定には使わない情報）
    pub lowest_local: Option<LaunchFitMode>,
    pub verdict: Verdict,
}

/// 向きの全体のモードのうち、出どころの優先順で最初の出どころの最も低いもの。
fn whole_body(input: &LaunchFitInput, direction: LaunchFitDirection) -> Option<&LaunchFitMode> {
    LaunchFitSource::PRIORITY.into_iter().find_map(|source| {
        input
            .modes
            .iter()
            .filter(|m| m.source == source && m.direction == Some(direction))
            .filter(|m| m.frequency.value().is_finite())
            .min_by(|a, b| a.frequency.value().total_cmp(&b.frequency.value()))
    })
}

/// 全体のモードが見つからなかった向きについて、優先順で最初の「探した範囲」。
fn searched(input: &LaunchFitInput) -> Option<&LaunchFitSearch> {
    LaunchFitSource::PRIORITY.into_iter().find_map(|source| {
        input
            .searched
            .iter()
            .find(|s| s.source == source && s.up_to.value().is_finite())
    })
}

/// 下限を下回る最も低い局所のモード。
fn lowest_local(input: &LaunchFitInput, limit: f64) -> Option<&LaunchFitMode> {
    input
        .modes
        .iter()
        .filter(|m| m.direction.is_none() && m.frequency.value() < limit)
        .filter(|m| m.frequency.value().is_finite())
        .min_by(|a, b| a.frequency.value().total_cmp(&b.frequency.value()))
}

/// 向きごとの判定（下限のある向きだけ。横 → 機軸の順）。
pub(super) fn judge(input: &LaunchFitInput) -> Result<Vec<LaunchFitFrequency>, AnalysisError> {
    let mut out = Vec::new();
    for (direction, limits) in [
        (LaunchFitDirection::Lateral, &input.min_lateral),
        (LaunchFitDirection::Axial, &input.min_axial),
    ] {
        let Some(limit) = super::tightest_min_frequency(limits) else {
            continue;
        };
        positive("Minimum natural frequency", limit.value.value())?;
        let hz = limit.value.value();
        let estimate = whole_body(input, direction);
        // 見積もりが無いときは、探した範囲を下界として使う。範囲が下限に届かなければ、違反とは言えない（下限との間に一次があるかもしれない）ので判定しない
        let bound = match estimate {
            Some(_) => None,
            None => searched(input).filter(|s| s.up_to.value() >= hz),
        };
        let evidence = estimate
            .map(|e| e.frequency.value())
            .or_else(|| bound.map(|s| s.up_to.value()));
        let margin = evidence.and_then(|f| ratio_margin(f, hz));
        let verdict = match evidence {
            Some(_) => ratio_margin_verdict(margin, FIT_MARGIN_WARNING_BAND),
            // 見積もりも十分な範囲も無いのは「確かめていない」。適合と読まれないよう警告にする
            None => Verdict::Warning,
        };
        // 文のために、届かなかった範囲も出力に残す
        let short = match (estimate, bound) {
            (None, None) => searched(input),
            _ => None,
        };
        out.push(LaunchFitFrequency {
            direction,
            limit: limit.value,
            origin: limit.origin.clone(),
            estimated: estimate.map(|e| e.frequency),
            estimated_by: estimate.map(|e| e.id.clone()),
            source: estimate
                .map(|e| e.source)
                .or(bound.or(short).map(|s| s.source)),
            searched_up_to: bound.or(short).map(|s| s.up_to),
            margin,
            lowest_local: lowest_local(input, hz).cloned(),
            verdict,
        });
    }
    Ok(out)
}

/// 出どころが使えなかった理由（判定の文の末尾に付ける。無ければ空）。
fn unused_text(notes: &[LaunchFitSourceNote]) -> String {
    notes
        .iter()
        .map(|n| format!("; {} not used: {}", n.source.text(), n.reason))
        .collect()
}

/// 下限を下回る局所のモードの情報（判定には使わない。無ければ空）。
fn local_text(f: &LaunchFitFrequency) -> String {
    match &f.lowest_local {
        Some(m) => format!(
            "; the lowest local mode ({} of the {}) is {}, below the limit, and is not judged against the whole-body requirement",
            m.id,
            m.source.text(),
            quantity_text::<Frequency>(m.frequency.value()),
        ),
        None => String::new(),
    }
}

/// 判定の行（文には使った出どころを書く）。
pub(super) fn check(f: &LaunchFitFrequency, notes: &[LaunchFitSourceNote]) -> Check {
    let name = f.direction.name();
    let required = format!(
        "required at least {} from {}",
        quantity_text::<Frequency>(f.limit.value()),
        f.origin
    );
    let hz = |q: Qty<Frequency>| quantity_text::<Frequency>(q.value());
    let message = match (f.estimated, f.searched_up_to, f.source) {
        (Some(e), _, Some(source)) => format!(
            "{} by the {}, {} ({required})",
            hz(e),
            source.text(),
            f.estimated_by.as_deref().unwrap_or("-")
        ),
        (None, Some(up_to), Some(source)) if f.verdict == Verdict::Warning => format!(
            "Not verified: the {} found no global {name} mode up to {}, which is below the required minimum, so a lower first mode cannot be ruled out ({required})",
            source.text(),
            hz(up_to)
        ),
        (None, Some(up_to), Some(source)) => format!(
            "The {} found no global {name} mode up to {}, so the first one is above {} ({required})",
            source.text(),
            hz(up_to),
            hz(up_to)
        ),
        _ => format!(
            "Not verified: there is no whole-body estimate of the {name} first natural frequency (structure.modal_estimate has no beam result) ({required})"
        ),
    };
    Check::new(
        &sat_checkkey::launch_frequency(name),
        &format!("First natural frequency ({name})"),
        f.verdict,
        format!("{message}{}{}", local_text(f), unused_text(notes)),
    )
}
