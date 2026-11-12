//! モードごとの状態と時間割合。**電源（消費電力）・C&DH（データ発生量）・熱（内部発熱）がこれを使う。**
//!
//! モードは「部品 ID → 状態（＋デューティ比）」の割り当ての集まり。部品は文字列の ID で識別し、
//! 部品の中身（電力・データレートなど）はこのモジュールは知らない。値は呼び出し側が状態名から引く。
//! 部品の状態ごとの値の表は [`StateValues`]（単体値。員数は各解析の部品の `count` に持たせて解析が掛ける）。
//!
//! **方針（ここだけ）**: モードで状態が割り当てられていない部品と、使う状態の値が無い部品は、
//! 0 とみなさず**警告して合計から除く**（[`evaluate`]・[`ModeEvaluation::checks`]）。

use crate::spec::{AnalysisError, Check, Verdict};
use sat_units::typed::{Dimension, Qty};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

/// 割合の合計を 1 とみなす許容差（デューティ比・モードの時間割合）。**この許容差はここだけ。**
pub const DUTY_TOLERANCE: f64 = 1e-9;

/// 部品 1 台の状態ごとの値（消費電力・データレートなど、単体値）。`by_state` に無い状態は `default`
/// （それも無ければ値なし）。**状態ごとの値の表はこれを使う**（電源・C&DH・熱で共有）。
#[derive(Serialize, Deserialize)]
#[serde(bound = "")]
#[cfg_attr(
    feature = "ts",
    derive(ts_rs::TS),
    ts(concrete(D = sat_units::typed::Dimensionless), bound = "")
)]
pub struct StateValues<D: Dimension> {
    /// 状態によらない値（`by_state` に無い状態に使う）
    #[serde(default)]
    pub default: Option<Qty<D>>,
    /// 状態ごとの値（例: `{"idle": 0, "on": 1.2}`）
    #[serde(default)]
    pub by_state: BTreeMap<String, Qty<D>>,
}

impl<D: Dimension> StateValues<D> {
    /// 状態 `state` の値（`by_state` に無ければ `default`、それも無ければ `None`）。
    pub fn get(&self, state: &str) -> Option<Qty<D>> {
        self.by_state.get(state).copied().or(self.default)
    }

    /// 値が 1 つも無いか（`default` も `by_state` も無い）。
    pub fn is_empty(&self) -> bool {
        self.default.is_none() && self.by_state.is_empty()
    }
}

// 次元の型 `D` は印だけなので、`D` に何も求めずに実装する（derive だと `D: Clone` などが要る）。
impl<D: Dimension> Default for StateValues<D> {
    fn default() -> Self {
        Self {
            default: None,
            by_state: BTreeMap::new(),
        }
    }
}

impl<D: Dimension> Clone for StateValues<D> {
    fn clone(&self) -> Self {
        Self {
            default: self.default,
            by_state: self.by_state.clone(),
        }
    }
}

impl<D: Dimension> std::fmt::Debug for StateValues<D> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("StateValues")
            .field("default", &self.default)
            .field("by_state", &self.by_state)
            .finish()
    }
}

/// モードの割り当てのうち、[`evaluate`] に渡した部品の一覧に無い部品の扱い。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UnlistedParts {
    /// 警告する（`unknown_part`）。一覧が値を持ちうる部品のすべてのとき（電源の消費電力）
    Warn,
    /// 警告しない。一覧がその解析の対象の部品だけのとき（C&DH はレートを持つ部品、熱は節点に含まれる部品）
    Ignore,
}

/// 衛星のモード（部品ごとの状態の割り当て）。
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct ModeDef {
    pub id: String,
    pub label: String,
    pub assignments: Vec<StateAssignment>,
}

impl ModeDef {
    /// 部品 `part` の割り当て。無ければ `None`（呼び出し側は「0 とみなす」のではなく未割り当てとして扱う）。
    pub fn assignment(&self, part: &str) -> Option<&StateAssignment> {
        self.assignments.iter().find(|a| a.part == part)
    }

    /// 同じ部品が 2 回以上割り当てられていたらエラー。
    pub fn validate(&self) -> Result<(), AnalysisError> {
        for (i, a) in self.assignments.iter().enumerate() {
            if self.assignments[..i].iter().any(|b| b.part == a.part) {
                return Err(AnalysisError::InvalidInput(format!(
                    "モード {} で部品 {} が 2 回割り当てられています",
                    self.id, a.part
                )));
            }
        }
        Ok(())
    }
}

/// 1 部品の状態の割り当て。`duty` に書いた状態がその時間割合、残りが `state`。
///
/// 例: `state = "rx"`, `duty = {"tx": 0.1}` → rx 90 %・tx 10 %。
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct StateAssignment {
    pub part: String,
    pub state: String,
    #[serde(default)]
    pub duty: BTreeMap<String, f64>,
}

impl StateAssignment {
    /// (状態, 時間割合) の一覧。先頭が基本の `state`（残りの割合。0 のこともある）、続いて `duty` の状態（名前順）。
    /// 割合の合計は 1。`duty` の値が 0〜1 の外・有限でない・合計が 1 を超える・基本の状態を `duty` にも書いた場合はエラー。
    pub fn fractions(&self) -> Result<Vec<(String, f64)>, AnalysisError> {
        let invalid = |why: String| {
            AnalysisError::InvalidInput(format!("部品 {} の状態の割り当て: {why}", self.part))
        };
        if self.duty.contains_key(&self.state) {
            return Err(invalid(format!(
                "基本の状態 {} を duty にも書いています",
                self.state
            )));
        }
        let mut sum = 0.0;
        for (state, &f) in &self.duty {
            if !f.is_finite() || !(0.0..=1.0).contains(&f) {
                return Err(invalid(format!("状態 {state} の割合 {f} が 0〜1 の外です")));
            }
            sum += f;
        }
        if sum > 1.0 + DUTY_TOLERANCE {
            return Err(invalid(format!("duty の合計 {sum} が 1 を超えています")));
        }
        let mut out = Vec::with_capacity(self.duty.len() + 1);
        out.push((self.state.clone(), (1.0 - sum).max(0.0)));
        out.extend(self.duty.iter().map(|(s, &f)| (s.clone(), f)));
        Ok(out)
    }

    /// 状態ごとの値（電力やデータレート）を時間割合で重み付けした平均。
    /// 割合が 0 より大きい状態のどれかで `value_of` が `None` を返したら `Ok(None)`（値が足りない）。
    pub fn weighted_mean(
        &self,
        value_of: impl Fn(&str) -> Option<f64>,
    ) -> Result<Option<f64>, AnalysisError> {
        let mut mean = 0.0;
        for (state, f) in self.fractions()? {
            if f <= 0.0 {
                continue;
            }
            match value_of(&state) {
                Some(v) => mean += f * v,
                None => return Ok(None),
            }
        }
        Ok(Some(mean))
    }
}

/// ID でモードを引く。無ければ `InvalidInput`。
pub fn find_mode<'a>(modes: &'a [ModeDef], id: &str) -> Result<&'a ModeDef, AnalysisError> {
    modes
        .iter()
        .find(|m| m.id == id)
        .ok_or_else(|| AnalysisError::InvalidInput(format!("モード {id} の定義がありません")))
}

/// 1 モードの評価（部品ごとの時間平均と、合計から除いた部品）。
#[derive(Debug, Clone, Default)]
pub struct ModeEvaluation {
    pub mode: String,
    /// 部品ごとの平均（`evaluate` に渡した部品の順）。未割り当て・値の不足は `None`
    pub means: Vec<(String, Option<f64>)>,
    /// 状態が割り当てられていない部品
    pub unassigned: Vec<String>,
    /// 割合が正の状態に値が無い (部品, 状態)
    pub missing: Vec<(String, String)>,
    /// 割り当てはあるが部品の一覧に無い ID（[`UnlistedParts::Ignore`] で評価したときは空）
    pub unknown_parts: Vec<String>,
}

impl ModeEvaluation {
    /// 値が揃った部品の平均の合計（未割り当て・値の不足は除く）。
    pub fn total(&self) -> f64 {
        self.means.iter().filter_map(|(_, m)| *m).sum()
    }

    /// 合計から除いた部品の警告。`value_label` は値の名前（例: `"消費電力"`・`"データレート"`）。
    /// キーは `unassigned`・`missing_value`・`unknown_part`。
    pub fn checks(&self, value_label: &str) -> Vec<Check> {
        let mut out = Vec::new();
        let mut push = |key: &str, label: &str, what: &str, list: Vec<String>| {
            if !list.is_empty() {
                let msg = format!(
                    "モード {} で{what}（合計に入れていません）: {}",
                    self.mode,
                    list.join(", ")
                );
                out.push(Check::new(key, label, Verdict::Warning, msg));
            }
        };
        push(
            "unassigned",
            "状態の割り当てが無い部品",
            "状態が決まっていない部品があります",
            self.unassigned.clone(),
        );
        let missing = self
            .missing
            .iter()
            .map(|(p, s)| format!("{p}.{s}"))
            .collect();
        push(
            "missing_value",
            &format!("{value_label}が無い状態"),
            &format!("{value_label}が無い状態があります"),
            missing,
        );
        push(
            "unknown_part",
            "部品の一覧に無い割り当て",
            "一覧に無い部品が割り当てられています",
            self.unknown_parts.clone(),
        );
        out
    }
}

/// モード `mode` での部品 `parts`（ID）の時間平均を求める。`value_of(部品, 状態)` は状態の値（無ければ `None`）。
/// 一覧に無い部品の割り当てを警告するかは `unlisted` で選ぶ。
pub fn evaluate<'a>(
    mode: &ModeDef,
    parts: impl IntoIterator<Item = &'a str>,
    unlisted: UnlistedParts,
    value_of: impl Fn(&str, &str) -> Option<f64>,
) -> Result<ModeEvaluation, AnalysisError> {
    mode.validate()?;
    let mut ev = ModeEvaluation {
        mode: mode.id.clone(),
        ..Default::default()
    };
    let parts: Vec<&str> = parts.into_iter().collect();
    for &part in &parts {
        let Some(a) = mode.assignment(part) else {
            ev.unassigned.push(part.to_owned());
            ev.means.push((part.to_owned(), None));
            continue;
        };
        let lacking: Vec<String> = a
            .fractions()?
            .into_iter()
            .filter(|(s, f)| *f > 0.0 && value_of(part, s).is_none())
            .map(|(s, _)| s)
            .collect();
        let mean = a.weighted_mean(|s| value_of(part, s))?;
        ev.missing
            .extend(lacking.into_iter().map(|s| (part.to_owned(), s)));
        ev.means.push((part.to_owned(), mean));
    }
    if unlisted == UnlistedParts::Warn {
        ev.unknown_parts = mode
            .assignments
            .iter()
            .filter(|a| !parts.contains(&a.part.as_str()))
            .map(|a| a.part.clone())
            .collect();
    }
    Ok(ev)
}

#[cfg(test)]
mod tests;
