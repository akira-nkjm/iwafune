import { describe, expect, it } from "vitest";
import { crossingsOn, jumpPath, labelPoint, segmentsOf } from "./jumps";

describe("線の飛び越し", () => {
  const vertical = segmentsOf([
    { x: 50, y: -20 },
    { x: 50, y: 20 },
  ]);
  it("水平の線分と垂直の線分の交点を見つける", () => {
    expect(crossingsOn({ a: { x: 0, y: 0 }, b: { x: 100, y: 0 } }, vertical, 4)).toEqual([50]);
    expect(crossingsOn({ a: { x: 0, y: 30 }, b: { x: 100, y: 30 } }, vertical, 4)).toEqual([]);
  });
  it("交点に半円を入れた path を作る", () => {
    const d = jumpPath(
      [
        { x: 0, y: 0 },
        { x: 100, y: 0 },
      ],
      vertical,
    );
    expect(d).toBe("M 0 0 L 46 0 A 4 4 0 0 1 54 0 L 100 0");
  });
  it("ラベルは一番長い線分の中点", () => {
    expect(
      labelPoint([
        { x: 0, y: 0 },
        { x: 10, y: 0 },
        { x: 10, y: 100 },
      ]),
    ).toEqual({ x: 10, y: 50 });
  });

  it("同じ所で交わる複数の線には、飛び越しを 1 つだけ描く", () => {
    const h = { a: { x: 100, y: 0 }, b: { x: 0, y: 0 } };
    const v = { a: { x: 50, y: -10 }, b: { x: 50, y: 10 } };
    expect(crossingsOn(h, [v, v, { a: { x: 52, y: -10 }, b: { x: 52, y: 10 } }], 4)).toEqual([52]);
  });
});
