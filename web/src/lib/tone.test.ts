import { readFileSync, writeFileSync } from "node:fs";
import { describe, expect, it } from "vitest";
import { contrast, deltaE, mixOklab } from "@/lib/color";
import {
  CSS_TOKENS_BEGIN,
  CSS_TOKENS_END,
  cssColorTokens,
  fillOnly,
  heatHex,
  palette,
  toneHex,
  toneInkHex,
  tones,
  toneTint,
} from "@/lib/tone";

const cssUrl = new URL("../styles/index.css", import.meta.url);
/** `pnpm -C web tokens` は UPDATE_TOKENS=1 で走らせ、index.css の色の区間を書き直す */
const update = process.env.UPDATE_TOKENS === "1";

const PANEL = palette.panel;
const APP = palette.app;

/**
 * Chromium で `color-mix(in oklab, neg t%, accent)` を canvas に塗って読んだ画素（手で写した参照値）。
 * 温度表がかつて CSS で混ぜていた色と、いまの `heatHex` が同じであることの確かめに使う
 */
const CHROMIUM_OKLAB_MIX = { 0.25: "#6b759e", 0.5: "#8b647a", 0.75: "#a44d54" } as const;

describe("色の正本と CSS", () => {
  it("index.css の色と影の区間は lib/tone.ts から書き出したものと一致する", () => {
    const css = readFileSync(cssUrl, "utf8");
    const begin = css.indexOf(CSS_TOKENS_BEGIN);
    const end = css.indexOf(CSS_TOKENS_END);
    expect(begin, "目印が無い").toBeGreaterThan(0);
    // 目印の行の頭（字下げ）から、終わりの目印の行末まで
    const from = css.lastIndexOf("\n", begin) + 1;
    const to = end + CSS_TOKENS_END.length;
    const now = css.slice(from, to);
    if (update) writeFileSync(cssUrl, css.slice(0, from) + cssColorTokens() + css.slice(to));
    else expect(now).toBe(cssColorTokens());
  });
});

describe("カテゴリ色", () => {
  it("文字用の ink は白地とサイドバー地で 4.5:1 以上", () => {
    for (const t of tones) {
      expect(contrast(toneInkHex[t], PANEL), `${t} on panel`).toBeGreaterThanOrEqual(4.5);
      expect(contrast(toneInkHex[t], APP), `${t} on app`).toBeGreaterThanOrEqual(4.5);
    }
  });

  it("塗り専用でない色は白地で 3:1 以上（非文字の基準）", () => {
    for (const t of tones) {
      if (fillOnly.includes(t)) continue;
      expect(contrast(toneHex[t], PANEL), t).toBeGreaterThanOrEqual(3);
    }
  });

  it("塗りの色どうしは見分けられる（OKLab ΔE ≥ 0.05）", () => {
    const close: string[] = [];
    for (let i = 0; i < tones.length; i++) {
      for (let j = i + 1; j < tones.length; j++) {
        const a = tones[i]!;
        const b = tones[j]!;
        if (deltaE(toneHex[a], toneHex[b]) < 0.05) close.push(`${a}/${b}`);
      }
    }
    expect(close).toEqual([]);
  });
});

describe("状態の色", () => {
  it("文字に使う ink は白地で 4.5:1 以上", () => {
    for (const name of [
      "ink",
      "ink-2",
      "ink-3",
      "accent-ink",
      "pos-ink",
      "warn-ink",
      "neg-ink",
    ] as const) {
      expect(contrast(palette[name], PANEL), name).toBeGreaterThanOrEqual(4.5);
    }
  });

  it("主ボタン（紺地に白字）は 14:1 以上", () => {
    expect(contrast(palette.ink, PANEL)).toBeGreaterThanOrEqual(14);
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

  it("熱の配色は低い温度が accent、高い温度が neg（温度表と 3D で同じ関数）", () => {
    expect(heatHex(0)).toBe(palette.accent);
    expect(heatHex(1)).toBe(palette.neg);
    expect(heatHex(0.5)).toBe(mixOklab(palette.accent, palette.neg, 0.5));
  });

  it("サブシステム色の淡い地は、面の色にカテゴリ色を混ぜ、濃さは枠 < 見出し < アイコン < カード", () => {
    expect(toneTint("blue", "header")).toBe(
      `color-mix(in srgb, ${toneHex.blue} 10%, var(--color-panel))`,
    );
    const percent = (s: string) => s.match(/ (\d+)%/)?.[1];
    const levels = (["frame", "header", "icon", "card"] as const).map((l) =>
      Number.parseInt(percent(toneTint("teal", l)) ?? "", 10),
    );
    expect(levels).toEqual([...levels].sort((a, b) => a - b));
    expect(new Set(levels).size).toBe(levels.length);
  });
});
