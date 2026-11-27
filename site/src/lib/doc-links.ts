/**
 * ランディングページと docs の入口から docs/ のページへ張るリンク。
 * ページがまだ docs/ に無いときは、リンク切れにせず docs の入口（/docs/）に向ける（ビルドのログに出す）。
 */
import fs from "node:fs";
import path from "node:path";
import { routeOfDoc } from "./routes.ts";
import { BASE, docsDir, REPO_URL, repoRoot, withBase } from "./site.ts";

/** 言語。英語がルート、日本語が `/ja/`。 */
export type Lang = "en" | "ja";

/** 言語ごとのルートの接頭辞（base なし）。 */
export function langPrefix(lang: Lang): string {
  return lang === "ja" ? "/ja" : "";
}

/** docs の入口のルート（base 付き）。 */
export function docsEntryHref(lang: Lang): string {
  return withBase(`${langPrefix(lang)}/docs/`);
}

/** ランディングページのルート（base 付き）。 */
export function landingHref(lang: Lang): string {
  return withBase(`${langPrefix(lang)}/`);
}

const missing = new Set<string>();

/**
 * docs/ のページ（例 `guide/quickstart`）のルート（base 付き）。英語版が無ければ docs の入口。
 * 日本語版が無いページも、英語版があれば `/ja/…`（Starlight が英語の内容で出す）を返す。
 */
export function docHref(slug: string, lang: Lang): string {
  const file = path.join(docsDir(), `${slug}.md`);
  if (!fs.existsSync(file)) {
    if (!missing.has(slug)) {
      missing.add(slug);
      console.warn(`[site] docs/${slug}.md does not exist yet; linking to the docs entry instead`);
    }
    return docsEntryHref(lang);
  }
  return withBase(`${langPrefix(lang)}${routeOfDoc(`${slug}.md`)}`);
}

/** GitHub のリリース一覧。 */
export const RELEASES_URL = `${REPO_URL}/releases`;

/** ワークスペースのバージョン（Cargo.toml の `[workspace.package] version`）。読めなければ null。 */
export function workspaceVersion(): string | null {
  try {
    const text = fs.readFileSync(path.join(repoRoot(), "Cargo.toml"), "utf-8");
    const block = /\[workspace\.package\]([\s\S]*?)(?:\n\[|$)/.exec(text)?.[1] ?? "";
    return /^version\s*=\s*"([^"]+)"/m.exec(block)?.[1] ?? null;
  } catch {
    return null;
  }
}

export { BASE };
