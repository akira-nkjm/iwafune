/**
 * サイトの色の値が、GUI の色の正本（web/src/lib/tone.ts から書き出された web/src/styles/index.css）と
 * 食い違っていないことを検査する。
 */
import fs from "node:fs";
import { describe, expect, it } from "vitest";

const read = (rel: string) => fs.readFileSync(new URL(rel, import.meta.url), "utf-8");

/** セレクタのブロックから `--prefix-name: value;` を集める。 */
function vars(css: string, selector: RegExp, prefix: string): Map<string, string> {
  const out = new Map<string, string>();
  const block = selector.exec(css);
  const body = block?.[1] ?? "";
  for (const m of body.matchAll(new RegExp(`--${prefix}-([a-z0-9-]+):\\s*([^;]+);`, "g"))) {
    out.set(m[1] ?? "", (m[2] ?? "").trim());
  }
  return out;
}

const siteCss = read("../styles/tokens.css");
const webCss = read("../../../web/src/styles/index.css");

const SURFACE_KEYS = [
  "app",
  "panel",
  "panel-2",
  "sunken",
  "chip",
  "line-soft",
  "line",
  "line-strong",
  "tick",
  "ink",
  "ink-2",
  "ink-3",
  "ink-4",
  "inverse",
  "accent",
  "accent-ink",
  "accent-tint",
  "pos",
  "pos-bg",
  "pos-ink",
  "warn",
  "warn-bg",
  "warn-ink",
  "neg",
  "neg-bg",
  "neg-ink",
];

describe("サイトの色は GUI の正本と同じ", () => {
  const lightWeb = vars(webCss, /@theme[^{]*\{([\s\S]*?)\n\}/, "color");
  const darkWeb = vars(webCss, /:root\[data-theme="dark"\]\s*\{([\s\S]*?)\n\}/, "color");
  const lightSite = vars(siteCss, /^:root\s*\{([\s\S]*?)\n\}/m, "iw");
  const darkSite = vars(siteCss, /:root\[data-theme="dark"\]\s*\{([\s\S]*?)\n\}/, "iw");
  const mediaSite = vars(
    siteCss,
    /@media \(prefers-color-scheme: dark\) \{\s*:root:not\(\[data-theme\]\) \{([\s\S]*?)\n {2}\}/,
    "iw",
  );

  it.each(SURFACE_KEYS)("明るい配色の %s", (key) => {
    expect(lightWeb.get(key), `web に ${key} が無い`).toBeDefined();
    expect(lightSite.get(key)).toBe(lightWeb.get(key));
  });

  it.each(SURFACE_KEYS)("暗い配色の %s", (key) => {
    expect(darkWeb.get(key), `web に ${key} が無い`).toBeDefined();
    expect(darkSite.get(key)).toBe(darkWeb.get(key));
    // OS の設定に従う暗い配色も同じ値。
    expect(mediaSite.get(key)).toBe(darkWeb.get(key));
  });

  it("端末ブロックの色は暗い配色の値", () => {
    const term = vars(siteCss, /([\s\S]*)/, "iw");
    expect(term.get("term-bg")).toBe(darkWeb.get("app"));
    expect(term.get("term-ink")).toBe(darkWeb.get("ink"));
    expect(term.get("term-prompt")).toBe(darkWeb.get("accent-ink"));
    expect(term.get("term-ok")).toBe(darkWeb.get("pos-ink"));
    expect(term.get("term-muted")).toBe(darkWeb.get("ink-3"));
  });
});
