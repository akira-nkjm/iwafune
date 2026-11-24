import { readFileSync, writeFileSync } from "node:fs";
import { describe, expect, it } from "vitest";
import { contrast, deltaE, mixOklab } from "@/lib/color";
import {
  CSS_TOKENS_BEGIN,
  CSS_TOKENS_DARK_BEGIN,
  CSS_TOKENS_DARK_END,
  CSS_TOKENS_END,
  cssColorTokens,
  cssColorTokensDark,
  fillOnly,
  heatHex,
  palette,
  paletteFor,
  type Scheme,
  schemes,
  toneHex,
  toneHexFor,
  toneInkHexFor,
  tones,
  toneTint,
  toneVar,
  uiHexFor,
} from "@/lib/tone";

const cssUrl = new URL("../styles/index.css", import.meta.url);
/** `pnpm -C web tokens` は UPDATE_TOKENS=1 で走らせ、index.css の色の区間を書き直す */
const update = process.env.UPDATE_TOKENS === "1";

/** 検査の対象にする配色。コントラストと見分けやすさは、明るい配色でも暗い配色でも同じ基準で確かめる */
const SCHEME_NAMES: Record<Scheme, string> = { light: "明るい配色", dark: "暗い配色" };

/** Chromium で `color-mix(in oklab, neg t%, accent)` を canvas に塗って読んだ画素（手で写した参照値）。`mixOklab` の確かめに使う */
const CHROMIUM_OKLAB_MIX = { 0.25: "#6b759e", 0.5: "#8b647a", 0.75: "#a44d54" } as const;

/** index.css の目印の区間を、書き出した内容と照らす（UPDATE_TOKENS=1 なら書き直す） */
function checkRegion(begin: string, end: string, expected: string) {
  const css = readFileSync(cssUrl, "utf8");
  const b = css.indexOf(begin);
  const e = css.indexOf(end);
  expect(b, `目印が無い: ${begin}`).toBeGreaterThan(0);
  // 目印の行の頭（字下げ）から、終わりの目印の行末まで
  const from = css.lastIndexOf("\n", b) + 1;
  const to = e + end.length;
  if (update) writeFileSync(cssUrl, css.slice(0, from) + expected + css.slice(to));
  else expect(css.slice(from, to)).toBe(expected);
}

describe("色の正本と CSS", () => {
  it("index.css の色と影の区間（明るい配色）は lib/tone.ts から書き出したものと一致する", () => {
    checkRegion(CSS_TOKENS_BEGIN, CSS_TOKENS_END, cssColorTokens());
  });

  it("index.css の暗い配色の区間は lib/tone.ts から書き出したものと一致し、同じ変数を置き換える", () => {
    checkRegion(CSS_TOKENS_DARK_BEGIN, CSS_TOKENS_DARK_END, cssColorTokensDark());
    const names = (css: string) =>
      [...css.matchAll(/(--(?:color|shadow)-[\w-]+):/g)].map((m) => m[1]);
    expect(names(cssColorTokensDark())).toEqual(names(cssColorTokens()));
  });

  it('暗い配色は `:root[data-theme="dark"]` の中にあり、`dark:` の variant は同じ属性に従う', () => {
    const css = readFileSync(cssUrl, "utf8");
    expect(css).toMatch(/:root\[data-theme="dark"\] \{\s*color-scheme: dark;/);
    expect(css).toContain(
      '@custom-variant dark (&:where([data-theme="dark"], [data-theme="dark"] *));',
    );
  });

  it("CSS 変数の参照は、その変数の名前と一致する", () => {
    expect(toneVar.blue).toBe("var(--color-cat-blue)");
    expect(cssColorTokens()).toContain("--color-cat-blue: #3382c2;");
  });
});

describe.each(schemes)("配色: %s", (scheme) => {
  const label = SCHEME_NAMES[scheme];
  const p = paletteFor(scheme);
  const fill = toneHexFor(scheme);
  const ink = toneInkHexFor(scheme);
  const PANEL = p.panel;
  const APP = p.app;

  it(`${label}: 文字用の ink は面とサイドバー地で 4.5:1 以上`, () => {
    for (const t of tones) {
      expect(contrast(ink[t], PANEL), `${t} on panel`).toBeGreaterThanOrEqual(4.5);
      expect(contrast(ink[t], APP), `${t} on app`).toBeGreaterThanOrEqual(4.5);
      expect(contrast(ink[t], p["panel-2"]), `${t} on panel-2`).toBeGreaterThanOrEqual(4.5);
    }
  });

  it(`${label}: 塗り専用でない色は面で 3:1 以上（非文字の基準）`, () => {
    for (const t of tones) {
      if (fillOnly.includes(t)) continue;
      expect(contrast(fill[t], PANEL), `${t} on panel`).toBeGreaterThanOrEqual(3);
      expect(contrast(fill[t], APP), `${t} on app`).toBeGreaterThanOrEqual(3);
    }
  });

  it(`${label}: 塗りの色どうしは見分けられる（OKLab ΔE ≥ 0.05）`, () => {
    const close: string[] = [];
    for (let i = 0; i < tones.length; i++) {
      for (let j = i + 1; j < tones.length; j++) {
        const a = tones[i]!;
        const b = tones[j]!;
        if (deltaE(fill[a], fill[b]) < 0.05) close.push(`${a}/${b}`);
      }
    }
    expect(close).toEqual([]);
  });

  it(`${label}: 文字に使う ink は面で 4.5:1 以上`, () => {
    for (const name of [
      "ink",
      "ink-2",
      "ink-3",
      "accent-ink",
      "pos-ink",
      "warn-ink",
      "neg-ink",
    ] as const) {
      expect(contrast(p[name], PANEL), `${name} on panel`).toBeGreaterThanOrEqual(4.5);
      expect(contrast(p[name], APP), `${name} on app`).toBeGreaterThanOrEqual(4.5);
    }
  });

  it(`${label}: 状態の色は、淡い地の上でも文字に使える（4.5:1 以上）`, () => {
    for (const s of ["pos", "warn", "neg"] as const) {
      expect(contrast(p[`${s}-ink`], p[`${s}-bg`]), s).toBeGreaterThanOrEqual(4.5);
    }
    expect(contrast(p["accent-ink"], p["accent-tint"])).toBeGreaterThanOrEqual(4.5);
  });

  it(`${label}: 主ボタン（字の色の地に inverse の字）は 14:1 以上`, () => {
    expect(contrast(p.ink, p.inverse)).toBeGreaterThanOrEqual(14);
  });

  it(`${label}: 面は app・panel・panel-2 の順に区別できる`, () => {
    expect(deltaE(p.app, p.panel)).toBeGreaterThan(0.005);
    expect(deltaE(p.panel, p["panel-2"])).toBeGreaterThan(0.005);
  });

  it(`${label}: 描画用の uiHex は palette と一致する`, () => {
    expect(uiHexFor(scheme).ink).toBe(p.ink);
    expect(uiHexFor(scheme).tick).toBe(p.tick);
  });
});

describe("配色の使い分け", () => {
  it("明るい配色の値は従来のまま（toneHex・palette は明るい配色）", () => {
    expect(toneHexFor("light")).toBe(toneHex);
    expect(paletteFor("light")).toBe(palette);
    expect(palette.panel).toBe("#ffffff");
    expect(toneHex.navy).toBe("#062654");
  });

  it("暗い配色は面が暗く字が明るい", () => {
    expect(contrast(paletteFor("dark").app, "#000000")).toBeLessThan(1.5);
    expect(contrast(paletteFor("dark").ink, "#000000")).toBeGreaterThan(14);
  });
});

describe("色の混ぜ方", () => {
  it("OKLab で混ぜる: 両端は元の色、範囲の外は端に寄せる、同じ色どうしは変わらない", () => {
    expect(mixOklab(palette.accent, palette.neg, 0)).toBe(palette.accent);
    expect(mixOklab(palette.accent, palette.neg, 1)).toBe(palette.neg);
    expect(mixOklab(palette.accent, palette.neg, -1)).toBe(palette.accent);
    expect(mixOklab(palette.accent, palette.neg, 2)).toBe(palette.neg);
    expect(mixOklab(toneHex.teal, toneHex.teal, 0.3)).toBe(toneHex.teal);
  });

  it("真ん中は両端から知覚の上で同じだけ離れる（OKLab の距離）", () => {
    const mid = mixOklab(palette.accent, palette.neg, 0.5);
    const a = deltaE(palette.accent, mid);
    const b = deltaE(mid, palette.neg);
    expect(Math.abs(a - b)).toBeLessThan(0.005);
    // sRGB の値をそのまま平均した色（以前の 3D の塗り）とは違う
    expect(mid).not.toBe("#765474");
  });

  it("CSS の color-mix(in oklab, …) と同じ値（Chromium で描いた画素を手で写した値）", () => {
    expect(mixOklab(palette.accent, palette.neg, 0.25)).toBe(CHROMIUM_OKLAB_MIX[0.25]);
    expect(mixOklab(palette.accent, palette.neg, 0.5)).toBe(CHROMIUM_OKLAB_MIX[0.5]);
    expect(mixOklab(palette.accent, palette.neg, 0.75)).toBe(CHROMIUM_OKLAB_MIX[0.75]);
  });

  it("熱画像の配色は低い温度が暗く高い温度が明るく、範囲外は端の色（凡例と 3D で同じ関数）", () => {
    expect(heatHex(0)).toBe("#10104a");
    expect(heatHex(1)).toBe("#fff9cf");
    expect(heatHex(-3)).toBe(heatHex(0));
    expect(heatHex(7)).toBe(heatHex(1));
    expect(heatHex(0.5)).toBe("#e8532f");
    // 暗い方から明るい方へ、明るさ（相対輝度）が単調に増える
    const lum = (hex: string) => contrast(hex, "#000000");
    const ls = [0, 0.2, 0.4, 0.6, 0.8, 1].map((r) => lum(heatHex(r)));
    for (let i = 1; i < ls.length; i++) expect(ls[i] ?? 0).toBeGreaterThan(ls[i - 1] ?? 0);
  });

  it("サブシステム色の淡い地は、面の色にカテゴリ色を混ぜ、濃さは枠 < 見出し < アイコン < カード", () => {
    expect(toneTint("blue", "header")).toBe(
      "color-mix(in srgb, var(--color-cat-blue) 10%, var(--color-panel))",
    );
    const percent = (s: string) => s.match(/ (\d+)%/)?.[1];
    const levels = (["frame", "header", "icon", "card"] as const).map((l) =>
      Number.parseInt(percent(toneTint("teal", l)) ?? "", 10),
    );
    expect(levels).toEqual([...levels].sort((a, b) => a - b));
    expect(new Set(levels).size).toBe(levels.length);
  });
});
