import { readFileSync, writeFileSync } from "node:fs";
import { describe, expect, it } from "vitest";
import { MARK, WORDMARK, wordmarkBox, wordmarkCenterY } from "./brand-art";
import { BRAND_FILES } from "./brand-svg";

const root = new URL("../../../", import.meta.url);
/** `pnpm -C web brand` は UPDATE_BRAND=1 で走らせ、ファイルを書き出す */
const update = process.env.UPDATE_BRAND === "1";

describe("ロゴの SVG（README・ファビコン）", () => {
  for (const file of BRAND_FILES) {
    it(`${file.path} は作図の正本から書き出したものと一致する`, () => {
      const url = new URL(file.path, root);
      if (update) writeFileSync(url, file.svg());
      expect(readFileSync(url, "utf8")).toBe(file.svg());
    });
  }
});

describe("名前の並べ方", () => {
  it("名前だけの viewBox は x ハイトの帯の中心で上下対称で、墨がはみ出さない", () => {
    const box = wordmarkBox();
    expect(box.y + box.h / 2).toBeCloseTo(wordmarkCenterY, 9);
    expect(box.y).toBeLessThanOrEqual(WORDMARK.ink.top);
    expect(box.y + box.h).toBeGreaterThanOrEqual(WORDMARK.ink.bottom);
  });

  it("マークの余白を除いた枠は、外形と線の太さの半分を含む", () => {
    // 外形（OUTLINE）の頂点の範囲は x 2.5〜21.5・y 9.5〜20.3。光学補正で動かし、線の太さの半分を足す
    const half = MARK.strokeWidth / 2;
    const { x, y, w, h } = MARK.tight;
    expect(x).toBeCloseTo(2.5 + MARK.shift.x - half, 9);
    expect(x + w).toBeCloseTo(21.5 + MARK.shift.x + half, 9);
    expect(y).toBeCloseTo(9.5 + MARK.shift.y - half, 9);
    expect(y + h).toBeCloseTo(20.3 + MARK.shift.y + half, 9);
  });
});
