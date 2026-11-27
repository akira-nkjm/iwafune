/**
 * docs の画面写真の目印（`<!-- screenshot: <route> — <description> -->`、状態を指定するなら
 * `<!-- screenshot: <route> as <name> — <description> -->`）の読み取りと、ファイル名の決め方。
 * ビルド用の remark プラグインと scripts/capture-shots.ts が同じ規則を使う（正本はここだけ）。
 */

/** 目印 1 つ。 */
export interface ShotMarker {
  /** GUI のルート（例 `/design/components`）。 */
  route: string;
  /** 名前（`as <name>`）。同じルートの特定の状態を撮る写真に付ける。無ければ、そのルートをそのまま開いた写真。 */
  name?: string;
  /** 写真の説明（写真がまだないときに枠に表示する）。 */
  description: string;
}

const MARKER =
  /^\s*screenshot:\s*(\/\S*)(?:\s+as\s+([a-z0-9]+(?:-[a-z0-9]+)*))?\s+(?:—|–|--|-)\s+([\s\S]+?)\s*$/;

/** HTML コメントの中身（`<!--` と `-->` の間）から目印を読む。目印でなければ null。 */
export function parseMarker(comment: string): ShotMarker | null {
  const m = MARKER.exec(comment);
  if (!m?.[1] || !m[3]) return null;
  const description = m[3].replace(/\s+/g, " ");
  return m[2] ? { route: m[1], name: m[2], description } : { route: m[1], description };
}

/** Markdown の本文にある目印を全部読む（コードブロックと、行内のコード（バッククォート）の中は読まない）。 */
export function findMarkers(markdown: string): ShotMarker[] {
  const withoutFences = markdown.replace(/^(```|~~~)[\s\S]*?^\1/gm, "").replace(/`[^`\n]*`/g, "");
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

/** 写真の識別子。名前が無ければ slug のまま、あれば `<slug>--<name>`（名前の無い写真と取り違えない）。 */
export function shotId(route: string, name?: string): string {
  return name ? `${shotSlug(route)}--${name}` : shotSlug(route);
}

/** 画面写真のファイル名。日本語の画面は `<id>.ja.png`、それ以外は `<id>.png`。 */
export function shotFileName(route: string, lang: "en" | "ja", name?: string): string {
  return `${shotId(route, name)}${lang === "ja" ? ".ja" : ""}.png`;
}
