//! 判定の基準（限度・しきい値）の置き場所。**基準の値は要求（`requirements/<ID>.toml` の `constraint`）だけが持ち**、
//! バジェットと解析は、確かめる先（`verify`）がそれを指す要求から基準を引く。
//!
//! - バジェット: `verify.budget = <種類>` の要求の `at_most` が、その種類の上限になる（[`Design::budget_limit`]）。
//! - 解析の判定: `verify.analysis = <解析 ID>` と `verify.check = <判定のキー>` が、解析が実行のたびに作る判定のキーに当たる要求が、
//!   その判定の基準になる（[`Design::criterion`]）。どの判定が基準を要求から取るか・向き・次元は宣言の表 [`CRITERIA`] が持つ。
//!   解析の判定のキーは解析が作る文字列なので、表の写しは fune-projection のテストが解析の出力と突き合わせる。
//! - ミッション期間は基準ではなく解析の入力なので、別の欄 `input = "mission_duration"` の要求が与える（[`Design::mission_duration`]）。
//! - 同じ基準を持つ要求が複数あれば、最も厳しいもの（`at_most` は最小、`at_least` は最大）が効く。
//!   向きか次元が表と合わない要求は基準にならない（`fune check` が `requirement.criterion_*` で知らせる）。
//! - 基準を持つ要求が無ければ `None`。そのときの振る舞い（判定を作らない・解析の既定を使う・入力不足にする）は、
//!   基準を読む射影（`fune_projection`）と解析が、設定が無かったときと同じに決める。

use super::{BudgetKind, ConstraintKind, Design, Requirement, RequirementInput};
use crate::value::Quantity;

/// 基準を要求から取る判定 1 つの宣言。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CriterionDecl {
    /// 解析の ID
    pub analysis: &'static str,
    /// 判定のキー、または判定のキーの族（`margin` は `margin:<回線>@<局>` のすべて。[`key_covers`]）
    pub check: &'static str,
    /// 基準を書く制約の向き（`at_most` か `at_least`）
    pub kind: ConstraintKind,
    /// 基準の次元名（要求の `quantity`）
    pub quantity: &'static str,
    /// 基準が入る解析の入力（説明用。`fune_projection` が入れる欄）
    pub feeds: &'static str,
}

/// 基準を要求から取る判定の宣言の表。**ここが基準の置き場所の宣言のただ 1 つの表**。
pub const CRITERIA: &[CriterionDecl] = &[
    CriterionDecl {
        analysis: "power.energy_balance",
        check: "dod",
        kind: ConstraintKind::AtMost,
        quantity: "Dimensionless",
        feeds: "EnergyBalanceBattery.dod_limit (power.energy_balance and power.sizing). Required: without it the analyses report a missing input",
    },
    CriterionDecl {
        analysis: "comm.link_budget",
        check: "margin",
        kind: ConstraintKind::AtLeast,
        quantity: "Dimensionless",
        feeds: "LinkBudgetLink.required_margin_db per link and station; comm.data_throughput takes the tightest of its downlink's stations. Without it the analysis default applies",
    },
    CriterionDecl {
        analysis: "comm.link_budget",
        check: "pfd",
        kind: ConstraintKind::AtMost,
        quantity: "Dimensionless",
        feeds: "LinkBudgetLink.pfd_limit_db (downlinks only). Without it no PFD check is made",
    },
    CriterionDecl {
        analysis: "aocs.slew_time",
        check: "slew_time",
        kind: ConstraintKind::AtMost,
        quantity: "Time",
        feeds: "SlewTimeInput.required_time. Without it no check is made",
    },
    CriterionDecl {
        analysis: "orbit.decay",
        check: "max_lifetime",
        kind: ConstraintKind::AtMost,
        quantity: "Time",
        feeds: "OrbitDecayInput.max_lifetime. Without it no check is made",
    },
    CriterionDecl {
        analysis: "thermal.steady",
        check: "range_low",
        kind: ConstraintKind::AtLeast,
        quantity: "Temperature",
        feeds: "ThermalNode.required (lower limit per node and range kind). It only tightens the part range: a looser requirement has no effect",
    },
    CriterionDecl {
        analysis: "thermal.steady",
        check: "range_high",
        kind: ConstraintKind::AtMost,
        quantity: "Temperature",
        feeds: "ThermalNode.required (upper limit per node and range kind). It only tightens the part range: a looser requirement has no effect",
    },
    CriterionDecl {
        analysis: "thermal.transient",
        check: "range_low",
        kind: ConstraintKind::AtLeast,
        quantity: "Temperature",
        feeds: "ThermalNode.required (lower limit per node and range kind). It only tightens the part range: a looser requirement has no effect",
    },
    CriterionDecl {
        analysis: "thermal.transient",
        check: "range_high",
        kind: ConstraintKind::AtMost,
        quantity: "Temperature",
        feeds: "ThermalNode.required (upper limit per node and range kind). It only tightens the part range: a looser requirement has no effect",
    },
    CriterionDecl {
        analysis: "structure.modal_estimate",
        check: "lateral",
        kind: ConstraintKind::AtLeast,
        quantity: "Frequency",
        feeds: "ModalEstimateRequirements.lateral and the limit candidates of structure.launch_fit. It only tightens the minimum of the selected launcher and dispenser. Without any of them no check is made",
    },
    CriterionDecl {
        analysis: "structure.modal_estimate",
        check: "axial",
        kind: ConstraintKind::AtLeast,
        quantity: "Frequency",
        feeds: "ModalEstimateRequirements.axial and the limit candidates of structure.launch_fit. It only tightens the minimum of the selected launcher and dispenser. Without any of them no check is made",
    },
];

/// 要求の判定のキー `requirement_key` が、解析が作った判定のキー `produced` に当たるか。
/// 一致するか、`requirement_key` の次の文字が区切り（`:` か `@`）の接頭辞。`margin` は `margin:s_down@TOKYO_GS` に、
/// `margin:s_down` は `margin:s_down@TOKYO_GS` に当たり、`margin:s` は `margin:s_down@…` に当たらない。
pub fn key_covers(requirement_key: &str, produced: &str) -> bool {
    produced == requirement_key
        || produced
            .strip_prefix(requirement_key)
            .is_some_and(|rest| rest.starts_with([':', '@']))
}

/// 解析 `analysis` の判定 `key` が基準を要求から取るなら、その宣言。
pub fn declared(analysis: &str, key: &str) -> Option<&'static CriterionDecl> {
    CRITERIA
        .iter()
        .find(|d| d.analysis == analysis && key_covers(d.check, key))
}

/// 要求の `verify.check` が当たる宣言（要求の側から引く。キーの族でも個別のキーでもよい）。
pub fn declared_for_requirement(r: &Requirement) -> Option<&'static CriterionDecl> {
    let (analysis, key) = (r.verify.analysis.as_deref()?, r.verify.check.as_deref()?);
    CRITERIA
        .iter()
        .find(|d| d.analysis == analysis && key_covers(d.check, key))
}

/// 引き当てた基準 1 つ。
#[derive(Debug, Clone, PartialEq)]
pub struct Criterion {
    /// 基準を決めた要求の ID
    pub requirement: String,
    /// 要求に書かれた値（単位のまま）
    pub value: Quantity,
    /// 制約の向き
    pub kind: ConstraintKind,
    /// 次元の基準の SI 値（厳しさの比較に使った値）
    pub si: f64,
}

impl Criterion {
    fn of(r: &Requirement, kind: ConstraintKind, quantity: &str) -> Option<Self> {
        if r.constraint.kind != kind {
            return None;
        }
        let value = r.constraint.value.clone()?;
        let si = value.si_as(quantity).ok()?;
        Some(Self {
            requirement: r.id.to_string(),
            value,
            kind,
            si,
        })
    }
}

/// 最も厳しい基準（`at_most` は最小、`at_least` は最大）。
fn tightest(
    candidates: impl Iterator<Item = Criterion>,
    kind: ConstraintKind,
) -> Option<Criterion> {
    let order = |a: &Criterion, b: &Criterion| a.si.total_cmp(&b.si);
    match kind {
        ConstraintKind::AtLeast => candidates.max_by(order),
        _ => candidates.min_by(order),
    }
}

impl Design {
    /// バジェットの上限（`verify.budget = <種類>` の `at_most` の要求のうち最も小さいもの）。
    pub fn budget_limit(&self, kind: BudgetKind) -> Option<Criterion> {
        let candidates = self
            .requirements
            .values()
            .filter(|r| r.verify.budget == Some(kind))
            .filter_map(|r| Criterion::of(r, ConstraintKind::AtMost, kind.dimension()));
        tightest(candidates, ConstraintKind::AtMost)
    }

    /// 解析 `analysis` の判定 `key` の基準（宣言が無い判定、基準を持つ要求が無い判定は `None`）。
    pub fn criterion(&self, analysis: &str, key: &str) -> Option<Criterion> {
        let decl = declared(analysis, key)?;
        let candidates = self
            .requirements
            .values()
            .filter(|r| r.verify.analysis.as_deref() == Some(analysis))
            .filter(|r| {
                r.verify
                    .check
                    .as_deref()
                    .is_some_and(|c| key_covers(c, key))
            })
            .filter_map(|r| Criterion::of(r, decl.kind, decl.quantity));
        tightest(candidates, decl.kind)
    }

    /// 解析の入力として与えられた値（`input` がそれの要求の制約の値。要求が複数あれば ID 順の先頭。複数は `fune check` が誤りにする）。
    /// 向きか次元が合わない要求は入力にならない。
    pub fn requirement_input(&self, input: RequirementInput) -> Option<Criterion> {
        self.requirements
            .values()
            .filter(|r| r.input == Some(input))
            .find_map(|r| Criterion::of(r, input.kind(), input.quantity()))
    }

    /// ミッション期間（`input = "mission_duration"` の要求の `at_least` の値）。
    pub fn mission_duration(&self) -> Option<Criterion> {
        self.requirement_input(RequirementInput::MissionDuration)
    }

    /// 要求 `r` と同じ基準に効く、ほかの要求の ID（同じバジェットの種類、または同じ解析で判定のキーが重なるもの）。
    /// 要求適合表が、要求ごとに「その要求の制約だけ」で判定し直すときに外す対象。
    pub fn criterion_siblings(&self, r: &Requirement) -> Vec<String> {
        let same = |o: &&Requirement| {
            if let Some(kind) = r.verify.budget {
                return o.verify.budget == Some(kind);
            }
            match (&r.verify.analysis, &r.verify.check) {
                (Some(a), Some(c)) if o.verify.analysis.as_ref() == Some(a) => o
                    .verify
                    .check
                    .as_deref()
                    .is_some_and(|oc| key_covers(c, oc) || key_covers(oc, c)),
                _ => false,
            }
        };
        self.requirements
            .values()
            .filter(|o| o.id != r.id)
            .filter(same)
            .map(|o| o.id.to_string())
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::design::{Constraint, Verification};
    use crate::id::Id;

    fn req(
        id: &str,
        kind: ConstraintKind,
        v: f64,
        unit: &str,
        verify: Verification,
    ) -> Requirement {
        Requirement {
            id: Id::new(id),
            label: None,
            statement: "s".into(),
            statement_en: None,
            quantity: "Mass".into(),
            source: None,
            input: None,
            constraint: Constraint {
                kind,
                value: Some(Quantity::new(v, unit)),
                tolerance: None,
                min: None,
                max: None,
            },
            quantify: Default::default(),
            verify,
        }
    }

    fn budget_req(id: &str, v: f64, unit: &str) -> Requirement {
        let verify = Verification {
            budget: Some(BudgetKind::Mass),
            ..Default::default()
        };
        req(id, ConstraintKind::AtMost, v, unit, verify)
    }

    #[test]
    fn key_covers_matches_at_separators_only() {
        assert!(key_covers("margin", "margin:s_down@TOKYO_GS"));
        assert!(key_covers("margin:s_down", "margin:s_down@TOKYO_GS"));
        assert!(key_covers(
            "margin:s_down@TOKYO_GS",
            "margin:s_down@TOKYO_GS"
        ));
        assert!(!key_covers("margin:s", "margin:s_down@TOKYO_GS"));
        assert!(!key_covers("margin", "marginal"));
        assert!(!key_covers("margin:s_down@TOKYO", "margin:s_down@TOKYO_GS"));
    }

    /// 同じ種類のバジェットに要求が複数あれば最も小さい上限が効き、単位が違っても SI で比べる。
    #[test]
    fn the_tightest_budget_limit_wins() {
        let mut d = Design::default();
        for r in [budget_req("A", 12.0, "kg"), budget_req("B", 11_000.0, "g")] {
            d.requirements.insert(r.id.clone(), r);
        }
        let limit = d.budget_limit(BudgetKind::Mass).expect("limit");
        assert_eq!(limit.requirement, "B");
        assert!((limit.si - 11.0).abs() < 1e-9);
        assert_eq!(d.budget_limit(BudgetKind::Power), None);
        let a = d.requirements.get("A").expect("A");
        assert_eq!(d.criterion_siblings(a), vec!["B".to_owned()]);
    }

    /// 向きが `at_most` でない要求は上限にならない。
    #[test]
    fn a_requirement_of_the_wrong_direction_is_not_a_limit() {
        let mut d = Design::default();
        let verify = Verification {
            budget: Some(BudgetKind::Mass),
            ..Default::default()
        };
        let r = req("A", ConstraintKind::AtLeast, 1.0, "kg", verify);
        d.requirements.insert(r.id.clone(), r);
        assert_eq!(d.budget_limit(BudgetKind::Mass), None);
    }
}
