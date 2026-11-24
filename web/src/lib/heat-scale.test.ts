import { describe, expect, it } from "vitest";
import { heatColor, heatRatio, heatScaleOfValues } from "./heat-scale";
import { heatHex } from "./tone";

describe("熱の色の範囲", () => {
  it("値の最低〜最高。NaN は数えず、値が無ければ null", () => {
    expect(heatScaleOfValues([300, 280, Number.NaN, 340])).toEqual({ min: 280, max: 340 });
    expect(heatScaleOfValues([])).toBeNull();
    expect(heatScaleOfValues([Number.NaN])).toBeNull();
  });

  it("温度は範囲の中の位置になり、幅が 0 なら真ん中", () => {
    expect(heatRatio(290, { min: 260, max: 320 })).toBeCloseTo(0.5);
    expect(heatRatio(290, { min: 290, max: 290 })).toBe(0.5);
    expect(heatColor(320, { min: 260, max: 320 })).toBe(heatHex(1));
  });
});
