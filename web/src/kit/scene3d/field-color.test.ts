import { Color } from "three";
import { describe, expect, it } from "vitest";
import { heatHex } from "@/lib/tone";
import { fieldRgb, LUT_STEPS } from "./field-color";

describe("3D の場の配色", () => {
  it("表の段では `heatHex`（熱画像・凡例の配色）の色そのもの", () => {
    for (const i of [0, 1, 17, 32, 63, LUT_STEPS]) {
      const expected = new Color(heatHex(i / LUT_STEPS)).toArray();
      expect([...fieldRgb(i / LUT_STEPS)]).toEqual(expected);
    }
  });

  it("範囲外は端の色、段の間は近い段", () => {
    expect([...fieldRgb(-3)]).toEqual([...fieldRgb(0)]);
    expect([...fieldRgb(9)]).toEqual([...fieldRgb(1)]);
    expect([...fieldRgb(0.5 / LUT_STEPS - 1e-9)]).toEqual([...fieldRgb(0)]);
  });
});
