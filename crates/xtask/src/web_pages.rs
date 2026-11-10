//! Web のページの URL と、ページの中身（`features/`）への依存の検査。
//!
//! - ページの URL（`/design/…`・`/systems/…`・`/mission/…`・`/project/…`）を文字列で組み立てるのは
//!   `web/src/lib/nav.ts` の `paths` だけ（ID の encode と、ルートの形との一致を 1 か所にする）。引用符の直後に
//!   これらの接頭辞が来る書き方を違反にする。API の道筋（`/design`・`/parts/…` など）は区間が続かないか接頭辞が違うので見ない。
//! - `@/features/` を import してよいのは、ページの中身（`features/`）どうしと、ページの登録（`routes/`）だけ。
//!   画面の枠（`shell/`）や共通の部品（`kit/`・`lib/`・`api/` など）からページの中身へは依存しない（先読みの計画などは
//!   `routes/pages.ts` から渡す）。
//!
//! どちらもテスト以外の `web/src/` のコード（生成物 `api/gen/` とテストの補助 `test/` を除く）だけを見る。

use std::path::Path;

use crate::web_strings::is_web_production;

/// ページの URL の接頭辞（`routes.ts` の区分。概要の `/` は区間が無いので見ない）。
const PAGE_PREFIXES: &[&str] = &["/design/", "/systems/", "/mission/", "/project/"];

/// ページの URL を組み立ててよいファイル。
const URL_HOME: &str = "web/src/lib/nav.ts";

/// しばらく URL の直書きを残すファイル（理由付き）。
const URL_PENDING: &[&str] = &[
    // 概要の KPI カードの行き先。KPI の集め方を service に移すとき（#23 の後）に `paths.system` にする
    "web/src/features/overview/use-overview-cards.tsx",
];

/// `@/features/` を import してよい場所。
const FEATURE_IMPORTERS: &[&str] = &["web/src/features/", "web/src/routes/"];

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
    let urls_allowed = rel == URL_HOME || URL_PENDING.contains(&rel);
    let features_allowed = FEATURE_IMPORTERS.iter().any(|d| rel.starts_with(d));
    let mut out = Vec::new();
    for (n, line) in text.lines().enumerate() {
        if !urls_allowed && let Some(prefix) = page_url(line) {
            out.push(format!(
                "{rel}:{}: ページの URL（`{prefix}…`）を直書きしています。URL は {URL_HOME} の `paths` で作ります（ID の encode もそこ）",
                n + 1
            ));
        }
        if !features_allowed && imports_feature(line) {
            out.push(format!(
                "{rel}:{}: `@/features/` を import しています。ページの中身に依存してよいのは features/ と routes/ だけです（shell へはページの登録 routes/pages.ts から渡す）",
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
        assert_eq!(problems("web/src/shell/CommandSearch.tsx", url).len(), 1);
        assert!(problems("web/src/lib/nav.ts", url).is_empty());
        let import = "import { isSystem } from \"@/features/systems/systems\";";
        assert_eq!(problems("web/src/shell/preload.ts", import).len(), 1);
        assert_eq!(problems("web/src/kit/Tree.tsx", import).len(), 1);
        assert!(problems("web/src/routes/pages.ts", import).is_empty());
        assert!(problems("web/src/features/modes/ModesTable.tsx", import).is_empty());
        let lazy = "() => import(\"@/features/overview/OverviewPage\"),";
        assert_eq!(problems("web/src/shell/pages.ts", lazy).len(), 1);
    }
}
