/**
 * サイト全体で共有する定数と、docs/ の場所の解決（正本はここだけ）。
 * docs/ はリポジトリ直下のものを読む。開発用に環境変数 IWAFUNE_DOCS_DIR で別の docs ディレクトリを指せる（コミットする設定には使わない）。
 */
import path from "node:path";

/** 公開 URL（GitHub Pages のプロジェクトサイト）。 */
export const SITE_ORIGIN = "https://akira-nkjm.github.io";
/** サイトの base パス（先頭にスラッシュ、末尾にスラッシュなし）。 */
export const BASE = "/iwafune";
/** GitHub のリポジトリ。 */
export const REPO_URL = "https://github.com/akira-nkjm/iwafune";
/** ソースへのリンクが指すブランチ。 */
export const SOURCE_BRANCH = "main";
/** 「このページを編集」が指すブランチ。 */
export const EDIT_BRANCH = "develop";

/**
 * site/ ディレクトリ（絶対パス、末尾に区切りなし）。`pnpm -C site …` のように site/ を作業ディレクトリにして動かす前提
 * （ビルドされたページの中では import.meta.url がソースを指さないため、作業ディレクトリを使う）。
 */
export const SITE_ROOT = process.cwd();

/** docs/ ディレクトリ（絶対パス）。 */
export function docsDir(env: NodeJS.ProcessEnv = process.env): string {
  const override = env.IWAFUNE_DOCS_DIR;
  return override ? path.resolve(override) : path.resolve(SITE_ROOT, "..", "docs");
}

/** リポジトリのルート（docs/ の親）。 */
export function repoRoot(docs: string = docsDir()): string {
  return path.dirname(docs);
}

/** スクリーンショットの置き場。 */
export const SHOTS_DIR = path.join(SITE_ROOT, "src", "assets", "shots");

/** BASE を付けたパス（`/guide/` → `/iwafune/guide/`）。 */
export function withBase(route: string, base: string = BASE): string {
  return `${base}${route.startsWith("/") ? route : `/${route}`}`;
}
