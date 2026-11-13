//! Web のページの URL と、ページの中身（`features/`）への依存の検査。
//!
//! - ページの URL（`/design/…`・`/systems/…`・`/conditions/…`・`/project/…`）を文字列で組み立てるのは
//!   `web/src/lib/nav.ts` の `paths` だけ（ID の encode と、ルートの形との一致を 1 か所にする）。引用符の直後に
//!   これらの接頭辞が来る書き方を違反にする。API の道筋（`/design`・`/parts/…` など）は区間が続かないか接頭辞が違うので見ない。
//! - `@/features/` を import してよいのは、ページの中身（`features/`）どうしと、ページの登録（`routes/`）だけ。
//!   画面の枠（`shell/`）や共通の部品（`kit/`・`lib/`・`api/`・`analysis/` など）からページの中身へは依存しない
//!   （先読みの計画などは `routes/pages.ts` から渡す）。
//! - ページの中身どうしの import（`features/<A>` → `features/<B>/…`）は、`FEATURE_EDGES` に理由付きで書いたものだけ。
//!   いくつものページで使う部品は共通の場所（解析の表示は `analysis/`、取得は `api/`）に置く。
//!
//! どれもテスト以外の `web/src/` のコード（生成物 `api/gen/` とテストの補助 `test/` を除く）だけを見る。

use std::path::Path;

use crate::web_strings::is_web_production;

/// ページの URL の接頭辞（`routes.ts` の区分。概要の `/` は区間が無いので見ない）。
const PAGE_PREFIXES: &[&str] = &["/design/", "/systems/", "/conditions/", "/project/"];

/// ページの URL を組み立ててよいファイル。
const URL_HOME: &str = "web/src/lib/nav.ts";

/// `@/features/` を import してよい場所。
const FEATURE_IMPORTERS: &[&str] = &["web/src/features/", "web/src/routes/"];

/// ページの中身どうしの import で認めるもの（読む側の feature、読まれる側の `<feature>/<モジュール>`、理由）。
/// ほかに要るときは、共通の場所へ移すか、ここに理由付きで足す。
const FEATURE_EDGES: &[(&str, &str, &str)] = &[
    (
        "components",
        "modes/matrix/matrix",
        "部品の詳細のモードの欄が、モード表のセルの文字と同じ読み方を使う",
    ),
    (
        "defs",
        "components/parameters/NewAttributeFields",
        "属性の作成の欄を、部品のパラメーター追加と共有する",
    ),
    (
        "defs",
        "components/parameters/param-ops",
        "属性の作成の引数（`attrAddCommand`）を、部品のパラメーター追加と共有する",
    ),
    (
        "diagram",
        "components/detail/PartDetail",
        "系統図の詳細に部品の詳細を出す",
    ),
    (
        "layout3d",
        "components/detail/PlacementSection",
        "3D 配置で選んだ部品の配置の欄を出す",
    ),
    (
        "conditions",
        "project/SettingsTable",
        "前提条件の設定を、設定のページと同じ表で出す",
    ),
    (
        "systems",
        "project/SettingsTable",
        "系の設定を、設定のページと同じ表で出す",
    ),
    (
        "systems",
        "layout3d/LazyLayout3D",
        "構造と熱の追加の表示（3D 配置）",
    ),
    ("systems", "layout3d/scene", "3D 配置の座標の型（重心の点）"),
];

/// 行の中の、引用符の直後に来るページの URL の接頭辞を探す。
fn page_url(line: &str) -> Option<&'static str> {
    PAGE_PREFIXES.iter().copied().find(|prefix| {
        ['"', '\'', '`']
            .iter()
            .any(|quote| line.contains(&format!("{quote}{prefix}")))
    })
}

/// ページの中身を import している行か。
fn imports_feature(line: &str) -> bool {
    line.contains("\"@/features/") || line.contains("'@/features/")
}

/// 行の中の import 先（`@/features/<feature>/<モジュール>` の `<feature>/<モジュール>`）。
fn feature_target(line: &str) -> Option<&str> {
    let start = line.find("@/features/")? + "@/features/".len();
    let rest = &line[start..];
    Some(&rest[..rest.find(['"', '\''])?])
}

/// ファイルが属するページの中身（`web/src/features/<feature>/…` の `<feature>`）。
fn feature_of(rel: &str) -> Option<&str> {
    rel.strip_prefix("web/src/features/")?.split('/').next()
}

/// ページの中身どうしの import が `FEATURE_EDGES` に無ければ、その組（読む側、読まれる側）。
fn unlisted_edge<'a>(rel: &'a str, line: &'a str) -> Option<(&'a str, &'a str)> {
    let from = feature_of(rel)?;
    let target = feature_target(line)?;
    let to = target.split('/').next()?;
    let listed = FEATURE_EDGES
        .iter()
        .any(|(f, t, _)| *f == from && *t == target);
    (from != to && !listed).then_some((from, target))
}

/// 違反を `errors` に積む。`files` はワークスペースのファイル一覧。
pub fn check(root: &Path, files: &[std::path::PathBuf], errors: &mut Vec<String>) {
    for file in files {
        let rel = file.strip_prefix(root).unwrap_or(file);
        let rel = rel.to_string_lossy().replace('\\', "/");
        if !is_web_production(&rel) {
            continue;
        }
        let Ok(text) = std::fs::read_to_string(file) else {
            continue;
        };
        errors.extend(problems(&rel, &text));
    }
}

/// 1 ファイルの違反の文言。
fn problems(rel: &str, text: &str) -> Vec<String> {
    let urls_allowed = rel == URL_HOME;
    let features_allowed = FEATURE_IMPORTERS.iter().any(|d| rel.starts_with(d));
    let mut out = Vec::new();
    for (n, line) in text.lines().enumerate() {
        if !urls_allowed && let Some(prefix) = page_url(line) {
            out.push(format!(
                "{rel}:{}: ページの URL（`{prefix}…`）を直書きしています。URL は {URL_HOME} の `paths` で作ります（ID の encode もそこ）",
                n + 1
            ));
        }
        if !imports_feature(line) {
            continue;
        }
        if !features_allowed {
            out.push(format!(
                "{rel}:{}: `@/features/` を import しています。ページの中身に依存してよいのは features/ と routes/ だけです（shell へはページの登録 routes/pages.ts から渡す）",
                n + 1
            ));
        } else if let Some((from, target)) = unlisted_edge(rel, line) {
            out.push(format!(
                "{rel}:{}: ページの中身どうしの import（features/{from} → features/{target}）です。いくつものページで使うものは共通の場所（analysis/・kit/・lib/・api/ など）に移すか、xtask の web_pages.rs の FEATURE_EDGES に理由付きで足します",
                n + 1
            ));
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn page_urls_are_found_but_api_paths_are_not() {
        assert_eq!(
            page_url("<ChipLink to={`/design/components/${u}`}>"),
            Some("/design/")
        );
        assert_eq!(page_url("to=\"/project/diagnostics\""), Some("/project/"));
        assert_eq!(page_url("navigate('/systems/power')"), Some("/systems/"));
        assert_eq!(page_url("api<DesignView>(\"/design\")"), None);
        assert_eq!(page_url("api<ProjectInfo>(\"/project\")"), None);
        assert_eq!(page_url("api<PartView>(`/parts/${seg(id)}`)"), None);
        assert_eq!(page_url("to={paths.component(id)}"), None);
    }

    #[test]
    fn only_nav_builds_urls_and_only_pages_import_features() {
        let url = "const to = `/design/components/${id}`;";
        assert_eq!(
            problems("web/src/shell/sidebar/CommandSearch.tsx", url).len(),
            1
        );
        assert!(problems("web/src/lib/nav.ts", url).is_empty());
        let import = "import { preloadSystem } from \"@/features/systems/preload\";";
        assert_eq!(problems("web/src/shell/preload.ts", import).len(), 1);
        assert_eq!(problems("web/src/kit/tree/Tree.tsx", import).len(), 1);
        assert!(problems("web/src/routes/pages.ts", import).is_empty());
        assert!(problems("web/src/features/systems/SystemPage.tsx", import).is_empty());
        let lazy = "() => import(\"@/features/overview/OverviewPage\"),";
        assert_eq!(problems("web/src/shell/pages.ts", lazy).len(), 1);
        // 解析の表示の共通の場所（analysis/）もページの中身に依存しない
        assert_eq!(
            problems("web/src/analysis/AnalysisCard.tsx", import).len(),
            1
        );
        // 概要の KPI カードも URL は paths で作る（一時的な許可は無い）
        let card = "to: \"/systems/power\",";
        assert_eq!(
            problems("web/src/features/overview/cards.ts", card).len(),
            1
        );
    }

    /// ページの中身どうしの import は FEATURE_EDGES に書いたものだけ（同じ feature の中は自由）。
    #[test]
    fn cross_feature_imports_must_be_listed() {
        let shared = "import { AnalysisCard } from \"@/features/systems/AnalysisCard\";";
        let errs = problems("web/src/features/conditions/OrbitPage.tsx", shared);
        assert_eq!(errs.len(), 1);
        assert!(errs[0].contains("features/conditions → features/systems/AnalysisCard"));
        let listed = "import { SettingsTable } from \"@/features/project/SettingsTable\";";
        assert!(problems("web/src/features/conditions/OrbitPage.tsx", listed).is_empty());
        let own = "import { heatMapOf } from \"@/features/systems/thermal/heat\";";
        assert!(problems("web/src/features/systems/extras.tsx", own).is_empty());
        assert!(problems("web/src/routes/pages.ts", shared).is_empty());
        assert_eq!(
            feature_target("import(\"@/features/systems/SystemPage\"),"),
            Some("systems/SystemPage")
        );
        assert_eq!(
            feature_of("web/src/features/overview/cards.ts"),
            Some("overview")
        );
        assert_eq!(feature_of("web/src/kit/tree/Tree.tsx"), None);
        for (from, target, why) in FEATURE_EDGES {
            assert!(
                !why.is_empty() && !target.starts_with(from),
                "{from} → {target}"
            );
        }
    }
}
