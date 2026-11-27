/**
 * ビルドしたサイト（dist/）の検査。`pnpm build` のあとに `pnpm check:dist` で動かす。
 * - 内部のリンク・画像・スクリプト・CSS の参照が、`/iwafune/` の下で実在するファイル（と、ページ内の見出し）を指す。
 * - 外部への参照がない: HTML の src・stylesheet・preload・srcset と CSS の url()・@import は、サイトの中だけ。
 *   本文の `<a href>` の外部リンク（文献など）は取得されないので許す。
 * - 検索の索引（pagefind）がある。
 * 問題があれば一覧して終了コード 1 で終わる。
 */
import fs from "node:fs";
import path from "node:path";
import { BASE, SITE_ORIGIN } from "../src/lib/site.ts";

/** 参照 1 つ。 */
export interface Ref {
  /** 参照の種類（`a` はリンク、`meta` は canonical・alternate の自分のサイトの URL、`resource` は取得される資源）。 */
  kind: "a" | "meta" | "resource";
  url: string;
}

/** HTML から参照を集める。 */
export function extractRefs(html: string): Ref[] {
  const refs: Ref[] = [];
  const body = html.replace(/<script\b[^>]*>[\s\S]*?<\/script>/gi, (m) => {
    // インラインのスクリプトの中身は読まず、src だけ見る。
    const src = /\bsrc=(["'])(.*?)\1/i.exec(m)?.[2];
    if (src) refs.push({ kind: "resource", url: src });
    return "";
  });
  for (const m of body.matchAll(/<([a-z][a-z0-9-]*)\b([^>]*)>/gi)) {
    const tag = (m[1] ?? "").toLowerCase();
    const attrs = m[2] ?? "";
    const get = (name: string) => new RegExp(`\\s${name}=(["'])(.*?)\\1`, "i").exec(attrs)?.[2];
    const href = get("href");
    const src = get("src");
    const srcset = get("srcset");
    if (tag === "a" && href !== undefined) refs.push({ kind: "a", url: href });
    else if (tag === "link" && href !== undefined) {
      const rel = (get("rel") ?? "").toLowerCase();
      // canonical と alternate は、取得されない自分のサイトの URL。
      if (rel === "canonical" || rel === "alternate") refs.push({ kind: "meta", url: href });
      else refs.push({ kind: "resource", url: href });
    } else if (href !== undefined && tag !== "base") refs.push({ kind: "resource", url: href });
    if (src !== undefined && tag !== "script") refs.push({ kind: "resource", url: src });
    if (srcset !== undefined) {
      for (const part of srcset.split(",")) {
        const u = part.trim().split(/\s+/)[0];
        if (u) refs.push({ kind: "resource", url: u });
      }
    }
  }
  return refs;
}

/** CSS の url()・@import の参照を集める（data: は除く）。 */
export function extractCssRefs(css: string): string[] {
  const out: string[] = [];
  for (const m of css.matchAll(/url\(\s*(["']?)(.*?)\1\s*\)/g)) out.push(m[2] ?? "");
  for (const m of css.matchAll(/@import\s+(["'])(.*?)\1/g)) out.push(m[2] ?? "");
  return out.filter((u) => u !== "" && !u.startsWith("data:"));
}

function walk(dir: string): string[] {
  return fs
    .readdirSync(dir, { withFileTypes: true })
    .flatMap((e) => (e.isDirectory() ? walk(path.join(dir, e.name)) : [path.join(dir, e.name)]));
}

/** dist の中でその URL のパスが指すファイル（無ければ null）。 */
function resolveInDist(dist: string, pathname: string): string | null {
  const rel = decodeURIComponent(pathname.slice(BASE.length));
  const candidates = [rel, path.join(rel, "index.html"), `${rel}.html`];
  for (const c of candidates) {
    const full = path.join(dist, c);
    if (full.startsWith(dist) && fs.existsSync(full) && fs.statSync(full).isFile()) return full;
  }
  return null;
}

const idCache = new Map<string, Set<string>>();
function idsOf(file: string): Set<string> {
  let ids = idCache.get(file);
  if (!ids) {
    ids = new Set(
      [...fs.readFileSync(file, "utf-8").matchAll(/\sid=(["'])(.*?)\1/g)].map((m) => m[2] ?? ""),
    );
    idCache.set(file, ids);
  }
  return ids;
}

/** dist を検査して、問題の一覧を返す。 */
/** HTML の中の、描けなかった数式の数（KaTeX が誤りのときに出す `katex-error` の要素）。 */
export function failedFormulas(html: string): number {
  return html.split('class="katex-error"').length - 1;
}

export function checkDist(dist: string): { problems: string[]; pages: number; links: number } {
  const problems: string[] = [];
  const files = walk(dist);
  const pages = files.filter((f) => f.endsWith(".html"));
  let links = 0;

  // 数式が描けなかったページ（KaTeX は誤りを `katex-error` の要素にして、元の式を赤い文字で出す）。
  for (const page of pages) {
    const failed = failedFormulas(fs.readFileSync(page, "utf-8"));
    if (failed > 0) {
      problems.push(`formula not rendered (${failed}) — ${path.relative(dist, page)}`);
    }
  }

  const check = (from: string, ref: Ref) => {
    const label = `${path.relative(dist, from)}: ${ref.url}`;
    if (ref.url === "" || /^(mailto:|tel:|data:|javascript:)/i.test(ref.url)) return;
    let url: URL;
    try {
      url = new URL(
        ref.url,
        `${SITE_ORIGIN}${BASE}/${path.relative(dist, from).replace(/index\.html$/, "")}`,
      );
    } catch {
      problems.push(`unparsable URL — ${label}`);
      return;
    }
    const external = url.origin !== SITE_ORIGIN;
    if (external) {
      if (ref.kind === "resource") problems.push(`external resource — ${label}`);
      return;
    }
    if (ref.kind === "meta") return;
    links += 1;
    if (!url.pathname.startsWith(`${BASE}/`) && url.pathname !== BASE) {
      problems.push(`outside the base path ${BASE} — ${label}`);
      return;
    }
    const target = resolveInDist(dist, url.pathname === BASE ? `${BASE}/` : url.pathname);
    if (!target) {
      problems.push(`missing target — ${label}`);
      return;
    }
    if (url.hash && target.endsWith(".html") && ref.kind === "a") {
      const id = decodeURIComponent(url.hash.slice(1));
      if (id !== "" && id !== "top" && !idsOf(target).has(id))
        problems.push(`missing anchor — ${label}`);
    }
  };

  for (const page of pages) {
    for (const ref of extractRefs(fs.readFileSync(page, "utf-8"))) check(page, ref);
  }
  for (const css of files.filter((f) => f.endsWith(".css"))) {
    for (const url of extractCssRefs(fs.readFileSync(css, "utf-8"))) {
      const where = path.relative(dist, css);
      const file = url.split(/[?#]/)[0] ?? "";
      if (/^(https?:)?\/\//.test(url)) {
        problems.push(`external resource in CSS — ${where}: ${url}`);
      } else if (
        url.startsWith("/")
          ? !resolveInDist(dist, file)
          : !fs.existsSync(path.resolve(path.dirname(css), file))
      ) {
        problems.push(`missing file in CSS — ${where}: ${url}`);
      }
    }
  }
  if (!fs.existsSync(path.join(dist, "pagefind", "pagefind-entry.json"))) {
    problems.push("search index missing — pagefind/pagefind-entry.json");
  }
  return { problems, pages: pages.length, links };
}

if (import.meta.url === `file://${process.argv[1]}`) {
  const dist = path.resolve(process.argv[2] ?? "dist");
  const { problems, pages, links } = checkDist(dist);
  console.log(`checked ${pages} pages, ${links} internal references`);
  if (problems.length > 0) {
    for (const p of problems) console.error(`  ${p}`);
    console.error(`${problems.length} problem(s)`);
    process.exit(1);
  }
  console.log("ok");
}
