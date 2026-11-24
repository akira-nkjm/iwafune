//! 古くなった承知（`acknowledged.stale`）の診断（`Snapshot::of` が診断に足す。`analysis` に依存しない葉のモジュール: 依存すると workspace と循環する）。承知済みの組（`analysis/structure.toml` の `[[acknowledged]]`）が、今のどの所見にも当たらないとき、
//! `fune check` が警告で知らせる（形状を変えて重ならなくなった組の承知が残るのを防ぐ）。
//!
//! **所見は解析を回さないと分からない**ので、モデルの検査（`fune_model::check`。解析に依存できない）ではなく、`structure.interference` と
//! `structure.cone_clearance` の両方を見られるここで調べる（`Snapshot::of` が診断に足す）。形態を指した承知は、その形態の所見だけと比べ、
//! 形態を指さない承知は、基準の配置とすべての形態のどれかの所見に当たればよい（形態ごとに所見は違うので、1 つの配置だけで古いとは言えない）。
//! 解析が回せない（寸法が足りないなど）配置しか比べられない承知は、古いとは言わない。

use fune_model::design::{AcknowledgedFinding, AnalysisSystem, Design, FindingKind};
use fune_model::resolve::Resolved;
use fune_model::{Diagnostic, Target};
use fune_projection::{Context, project_json_with_configuration};
use sat_analysis::spec::Analysis;
use sat_analysis::structure::{
    ConeClearance, ConeClearanceOutput, Interference, InterferenceOutput,
};
use std::collections::BTreeMap;

/// 配置 1 つ（`None` は解析が既定で選ぶ配置、`Some` は形態を明示）で出た所見（種類ごとの組の並び）。
type Found = BTreeMap<FindingKind, Vec<(String, String)>>;

/// 古い承知の警告（承知が無ければ空。解析も回さない）。
pub fn stale(design: &Design, resolved: &Resolved) -> Vec<Diagnostic> {
    let Some(structure) = &design.analysis.structure else {
        return Vec::new();
    };
    if structure.acknowledged.is_empty() {
        return Vec::new();
    }
    let Ok(ctx) = Context::new(design, resolved, None) else {
        return Vec::new();
    };
    let placements: Vec<Option<&str>> = std::iter::once(None)
        .chain(design.configurations.keys().map(|k| Some(k.as_str())))
        .collect();
    // 配置 → 出た所見（解析が回らなかった配置は入れない）
    let found: Vec<(Option<&str>, Found)> = placements
        .iter()
        .filter_map(|p| findings(&ctx, *p).map(|f| (*p, f)))
        .collect();
    structure
        .acknowledged
        .iter()
        .enumerate()
        .filter(|(_, a)| is_stale(a, &found))
        .map(|(i, a)| diagnostic(design, i, a))
        .collect()
}

/// 承知が、比べられる配置のどれかの所見に当たらないか（比べられる配置が無ければ偽）。
fn is_stale(a: &AcknowledgedFinding, found: &[(Option<&str>, Found)]) -> bool {
    let mut comparable = false;
    for (placement, f) in found {
        // 形態を指した承知は、その形態を明示して回した配置とだけ比べる。指さない承知はすべての配置と比べる
        if a.configuration.is_some() && a.configuration.as_deref() != *placement {
            continue;
        }
        comparable = true;
        let hit = f
            .get(&a.finding)
            .is_some_and(|pairs| pairs.iter().any(|(x, y)| a.same_pair(a.finding, x, y)));
        if hit {
            return false;
        }
    }
    comparable
}

/// 配置 `placement` で 2 つの解析を回し、出た所見を集める（どちらも回らなければ None）。
fn findings(ctx: &Context<'_>, placement: Option<&str>) -> Option<Found> {
    let mut out = Found::new();
    let mut ran = false;
    if let Some(o) = run::<Interference, InterferenceOutput>(ctx, placement) {
        ran = true;
        out.entry(FindingKind::Interference)
            .or_default()
            .extend(o.overlaps.into_iter().map(|p| (p.first, p.second)));
    }
    if let Some(o) = run::<ConeClearance, ConeClearanceOutput>(ctx, placement) {
        ran = true;
        out.entry(FindingKind::ConeClearance)
            .or_default()
            .extend(o.cones.into_iter().flat_map(|c| {
                let id = c.id;
                c.obstructions
                    .into_iter()
                    .map(move |x| (id.clone(), x.part))
            }));
    }
    ran.then_some(out)
}

/// 解析 `A` を配置 `placement` で回して型付きの出力にする（入力が足りない・解析が失敗するときは None）。
fn run<'a, A: Analysis, O: serde::de::DeserializeOwned>(
    ctx: &Context<'a>,
    placement: Option<&'a str>,
) -> Option<O> {
    let input = project_json_with_configuration(A::ID, ctx, placement)
        .0
        .ok()?;
    let output = sat_analysis::catalog::run_json(A::ID, input).ok()?;
    serde_json::from_value(output).ok()
}

/// 古い承知の警告 1 件（位置は `[[acknowledged]]` の該当の行）。
fn diagnostic(design: &Design, index: usize, a: &AcknowledgedFinding) -> Diagnostic {
    let path = AnalysisSystem::Structure
        .setting_path()
        .child(&["acknowledged", &index.to_string()]);
    let scope = match &a.configuration {
        Some(c) => format!(" in configuration `{c}`"),
        None => String::new(),
    };
    let message = format!(
        "The acknowledged {} pair `{}` / `{}` matches no current finding{scope}. If the finding is gone for good, withdraw it with `fune acknowledged rm {} {} {}{}`",
        a.finding,
        a.first,
        a.second,
        a.finding,
        a.first,
        a.second,
        a.configuration
            .as_deref()
            .map(|c| format!(" --configuration {c}"))
            .unwrap_or_default()
    );
    let entity = fune_model::design::EntityKey::Setting(AnalysisSystem::Structure.setting_path());
    let loc = design
        .source_map
        .locate(&entity, &["acknowledged", &index.to_string()]);
    let mut d = Diagnostic::warning("acknowledged.stale", message).at(loc);
    d.target = Some(Target::from(path));
    d
}

#[cfg(test)]
mod tests {
    use super::*;
    use fune_ops::Op;
    use fune_store::{Snapshot, load};

    fn demo() -> Snapshot {
        load(&std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../../fixtures/demo-6u"))
            .unwrap()
    }

    fn ack(finding: FindingKind, first: &str, second: &str, config: Option<&str>) -> Op {
        Op::Acknowledge {
            finding,
            first: first.into(),
            second: second.into(),
            configuration: config.map(str::to_owned),
            reason: "known".into(),
        }
    }

    /// 承知を足した設計の、古い承知の警告（組の文）。
    fn stale_of(ops: &[Op]) -> Vec<String> {
        let plan = fune_ops::plan(&demo(), ops).unwrap();
        let resolved = fune_model::resolve(&plan.design);
        stale(&plan.design, &resolved)
            .into_iter()
            .map(|d| {
                assert_eq!(d.code, "acknowledged.stale");
                d.message
            })
            .collect()
    }

    /// 今の所見に当たる承知は古くない。形状に当たらない組（取付の祖先と子孫など、判定されない組）は古い。
    /// 形態を指した承知は、その形態の所見だけと比べる（`SAP_PY`–`SP_PY` の重なりは基準にあり、`STOWED` には無い）。
    #[test]
    fn an_acknowledgement_that_matches_no_current_finding_is_reported() {
        use FindingKind::{ConeClearance, Interference};
        assert!(stale_of(&[]).is_empty());
        // 基準の重なり（承知は並びを問わない）・円錐の組は当たる
        let live = [
            ack(Interference, "SP_PY", "SAP_PY", None),
            ack(ConeClearance, "SS_PY", "SAP_PY", None),
            // 形態を指さない承知は、どれかの配置の所見に当たればよい（`SS_PX` の視野の `SAP_PY` は STOWED だけにある）
            ack(ConeClearance, "SS_PX", "SAP_PY", None),
        ];
        assert!(stale_of(&live).is_empty());
        // どの配置にも無い組は古い
        let none = stale_of(&[ack(Interference, "CAM", "RW_X", None)]);
        assert_eq!(none.len(), 1);
        assert!(
            none[0].contains("`CAM` / `RW_X`") && none[0].contains("matches no current finding")
        );
        // 円錐は順がある: 担い手と部品を入れ替えると当たらない
        assert_eq!(
            stale_of(&[ack(ConeClearance, "SAP_PY", "SS_PY", None)]).len(),
            1
        );
        // 形態を指した承知: STOWED には重なりが無いので古い。基準の重なりを指すだけの承知は古くない
        let stowed = stale_of(&[ack(Interference, "SP_PY", "SAP_PY", Some("STOWED"))]);
        assert_eq!(stowed.len(), 1);
        assert!(
            stowed[0].contains("in configuration `STOWED`")
                && stowed[0].contains("--configuration STOWED")
        );
        assert!(stale_of(&[ack(Interference, "SP_PX", "SS_PX", Some("STOWED"))]).is_empty());
    }
}
