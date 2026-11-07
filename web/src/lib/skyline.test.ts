import { describe, expect, it } from "vitest";
import { type Packed, packRects, type RectSize } from "./skyline";

/** 2 つの長方形が重なるか（間隔 gap を含めて） */
function overlaps(sizes: RectSize[], p: Packed, gap: number): boolean {
  for (let i = 0; i < sizes.length; i++) {
    for (let j = i + 1; j < sizes.length; j++) {
      const a = { ...p.spots[i]!, ...sizes[i]! };
      const b = { ...p.spots[j]!, ...sizes[j]! };
      const apart =
        a.x + a.width + gap <= b.x + 1e-9 ||
        b.x + b.width + gap <= a.x + 1e-9 ||
        a.y + a.height + gap <= b.y + 1e-9 ||
        b.y + b.height + gap <= a.y + 1e-9;
      if (!apart) return true;
    }
  }
  return false;
}

describe("packRects", () => {
  it("背の高いものの横の空きに、低いものを縦に重ねて入れる", () => {
    // 幅 100: 60×100 の横の 40 の幅に 40×50 を 2 つ重ねれば、全体は 100 の高さに収まる
    const sizes = [
      { width: 60, height: 100 },
      { width: 40, height: 50 },
      { width: 40, height: 50 },
    ];
    const p = packRects(sizes, 100, 0);
    expect(p.height).toBe(100);
    expect(overlaps(sizes, p, 0)).toBe(false);
  });

  it("間隔をあけ、幅の中に収める", () => {
    const sizes = [
      { width: 30, height: 20 },
      { width: 30, height: 40 },
      { width: 30, height: 20 },
      { width: 50, height: 10 },
    ];
    const gap = 10;
    const p = packRects(sizes, 100, gap);
    expect(overlaps(sizes, p, gap)).toBe(false);
    expect(p.width).toBeLessThanOrEqual(100);
    // 1 段目に 30 + 10 + 30 + 10 + 30 = 110 は入らないので 2 つ、残りはその下の空きへ
    expect(p.height).toBeLessThan(40 + gap + 20 + gap + 10);
  });

  it("幅より広いものは左端に置き、全体の幅はそれに合わせる", () => {
    const p = packRects([{ width: 150, height: 20 }], 100, 8);
    expect(p.spots[0]).toEqual({ x: 0, y: 0 });
    expect(p.width).toBe(150);
  });

  it("どの順で詰めても重ならない（乱数の大きさ）", () => {
    let seed = 7;
    const rand = () => {
      seed = (seed * 16807) % 2147483647;
      return seed / 2147483647;
    };
    for (let k = 0; k < 20; k++) {
      const sizes = Array.from({ length: 8 }, () => ({
        width: 40 + Math.floor(rand() * 200),
        height: 30 + Math.floor(rand() * 200),
      }));
      const p = packRects(sizes, 500, 20);
      expect(overlaps(sizes, p, 20)).toBe(false);
    }
  });

  it("空なら大きさ 0", () => {
    expect(packRects([], 100, 10)).toEqual({ spots: [], width: 0, height: 0 });
  });
});
