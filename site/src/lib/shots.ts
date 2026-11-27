/**
 * docs の画面写真の目印（`<!-- screenshot: <route> — <description> -->`）の読み取りと、ファイル名の決め方。
 * ビルド用の remark プラグインと scripts/capture-shots.ts が同じ規則を使う（正本はここだけ）。
 */

/** 目印 1 つ。 */
export interface ShotMarker {
  /** GUI のルート（例 `/design/components`）。 */
  route: string;
  /** 写真の説明（写真がまだないときに枠に表示する）。 */
  description: string;
}

const MARKER = /^\s*screenshot:\s*(\S+)\s+(?:—|–|--|-)\s+([\s\S]+?)\s*$/;

/** HTML コメントの中身（`<!--` と `-->` の間）から目印を読む。目印でなければ null。 */
export function parseMarker(comment: string): ShotMarker | null {
  const m = MARKER.exec(comment);
  if (!m?.[1] || !m[2]) return null;
  return { route: m[1], description: m[2].replace(/\s+/g, " ") };
}

/** Markdown の本文にある目印を全部読む（コードブロックの中は読まない）。 */
export function findMarkers(markdown: string): ShotMarker[] {
  const withoutFences = markdown.replace(/^(```|~~~)[\s\S]*?^\1/gm, "");
  const out: ShotMarker[] = [];
  for (const m of withoutFences.matchAll(/<!--([\s\S]*?)-->/g)) {
    const marker = parseMarker(m[1] ?? "");
    if (marker) out.push(marker);
  }
  return out;
}

/**
 * ルートからファイル名の元（slug）を作る。小文字にし、英数字以外の連続を `-` にし、前後の `-` を除く。
 * `/` は `home`。`/design/power/budget` は `design-power-budget`。
 */
export function shotSlug(route: string): string {
  const clean = route.split(/[?#]/)[0] ?? "";
  const slug = clean
    .toLowerCase()
    .replace(/[^a-z0-9]+/g, "-")
    .replace(/^-+|-+$/g, "");
  return slug === "" ? "home" : slug;
}

/** 画面写真のファイル名。日本語の画面は `<slug>.ja.png`、それ以外は `<slug>.png`。 */
export function shotFileName(route: string, lang: "en" | "ja"): string {
  return `${shotSlug(route)}${lang === "ja" ? ".ja" : ""}.png`;
}
