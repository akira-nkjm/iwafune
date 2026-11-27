/**
 * docs/ のファイルとサイトのルートの対応（正本はここだけ）。
 * `docs/guide/index.md` → `/guide/`、`docs/ja/guide/install.md` → `/ja/guide/install/`。
 */
import path from "node:path";
import { REPO_URL, SOURCE_BRANCH } from "./site.ts";

/** docs/ からの相対パス（`guide/install.md`）をルート（`/guide/install/`、base なし）にする。 */
export function routeOfDoc(rel: string): string {
  const noExt = rel.replace(/\.md$/i, "");
  const slug = noExt === "index" ? "" : noExt.replace(/(^|\/)index$/, "");
  return slug === "" ? "/" : `/${slug}/`;
}

/** リポジトリのファイルへの GitHub のリンク（ディレクトリは tree、ファイルは blob）。 */
export function githubSourceUrl(repoRel: string, hash = "", isDir = false): string {
  return `${REPO_URL}/${isDir ? "tree" : "blob"}/${SOURCE_BRANCH}/${repoRel}${hash}`;
}

/** 書き換えの前提（どこのファイルを、どの base で書き換えるか）。 */
export interface LinkContext {
  /** 書いてあるファイルの、リポジトリのルートからの相対パス（`docs/guide/x.md`）。 */
  fileRel: string;
  /** サイトの base（`/iwafune`）。 */
  base: string;
}

const SCHEME = /^[a-z][a-z0-9+.-]*:/i;

/**
 * Markdown のリンク先を書き換える。
 * - 外部 URL・`#見出し`・メール: そのまま。
 * - docs/ の `.md`: サイトのルート（base 付き、見出しの `#` は保つ）。
 * - docs/ の外のファイル（`../../crates/…`）と docs/ の中の `.md` 以外: GitHub の `main` のファイルへ。
 * - `/` で始まる相対は GitHub と同じくリポジトリのルートからとして読む。
 * リポジトリの外に出る相対パスはそのまま返す。
 */
export function rewriteLink(url: string, ctx: LinkContext): string {
  if (url === "" || url.startsWith("#") || url.startsWith("//") || SCHEME.test(url)) return url;
  const match = /^([^?#]*)((?:\?[^#]*)?)(#.*)?$/.exec(url);
  if (!match) return url;
  const [, target = "", , hash = ""] = match;
  if (target === "") return url;
  const joined = target.startsWith("/")
    ? target.slice(1)
    : path.posix.join(path.posix.dirname(ctx.fileRel), target);
  const resolved = path.posix.normalize(joined);
  if (resolved.startsWith("../") || resolved === "..") return url;
  if (resolved.startsWith("docs/") && /\.md$/i.test(resolved)) {
    const route = routeOfDoc(resolved.slice("docs/".length));
    return `${ctx.base}${route}${hash}`;
  }
  const isDir = target.endsWith("/");
  return githubSourceUrl(resolved.replace(/\/$/, ""), hash, isDir);
}

/** docs/ の画像などの置き場（サイトのルート直下。base は付けない）。 */
export const DOCS_FILES_ROUTE = "/docs-files/";

/** 画像として配るファイルの拡張子。 */
export const IMAGE_EXTENSIONS = [".svg", ".png", ".jpg", ".jpeg", ".gif", ".webp", ".avif"];

/**
 * Markdown の画像の参照先を書き換える。docs/ の中のファイルは `<base>/docs-files/<docs からの相対パス>` にする
 * （integrations/docs-files.ts がそこへ配る）。外部 URL・docs/ の外・リポジトリの外はそのまま返す。
 */
export function rewriteImage(url: string, ctx: LinkContext): string {
  if (url === "" || url.startsWith("#") || url.startsWith("//") || SCHEME.test(url)) return url;
  const match = /^([^?#]*)((?:\?[^#]*)?)(#.*)?$/.exec(url);
  const target = match?.[1] ?? "";
  if (target === "") return url;
  const joined = target.startsWith("/")
    ? target.slice(1)
    : path.posix.join(path.posix.dirname(ctx.fileRel), target);
  const resolved = path.posix.normalize(joined);
  if (!resolved.startsWith("docs/")) return url;
  return `${ctx.base}${DOCS_FILES_ROUTE}${resolved.slice("docs/".length)}${match?.[3] ?? ""}`;
}
