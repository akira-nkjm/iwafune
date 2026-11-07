import { readFileSync, writeFileSync } from "node:fs";
import { describe, expect, it } from "vitest";
import { contrast, deltaE } from "@/lib/color";
import {
  CSS_TOKENS_BEGIN,
  CSS_TOKENS_END,
  cssColorTokens,
  fillOnly,
  palette,
  toneHex,
  toneInkHex,
  tones,
} from "@/lib/tone";

const cssUrl = new URL("../styles/index.css", import.meta.url);
/** `pnpm -C web tokens` は UPDATE_TOKENS=1 で走らせ、index.css の色の区間を書き直す */
const update = process.env.UPDATE_TOKENS === "1";

const PANEL = palette.panel;
const APP = palette.app;

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
