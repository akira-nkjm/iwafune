import { describe, expect, it } from "vitest";
import type { ThermalSteadyOutput } from "@/api/types";
import { heatKey, heatMapOf, heatRatio } from "./heat";

describe("熱の温度表", () => {
  const out: ThermalSteadyOutput = {
    envelope: [],
    range_warning_band: 5,
    checks: [],
    cases: [
      {
        id: "hot",
        label: "高温",
        iterations: 1,
        residual: 0,
        nodes: [
          {
            node: "OBC",
            temperature: 320,
            absorbed: 0,
            internal: 0,
            margin_low: null,
            margin_high: null,
            verdict: "warning",
          },
          {
            node: "BAT",
            temperature: 300,
            absorbed: 0,
            internal: 0,
            margin_low: null,
            margin_high: null,
            verdict: "ok",
          },
        ],
      },
      {
        id: "cold",
        label: "",
        iterations: 1,
        residual: 0,
        nodes: [
          {
            node: "OBC",
            temperature: 260,
            absorbed: 0,
            internal: 0,
            margin_low: null,
            margin_high: null,
            verdict: "ok",
          },
        ],
      },
    ],
  };
  it("ケース × 節点に並べ、温度の範囲を持つ", () => {
    const m = heatMapOf(out);
    expect(m?.cases.map((c) => c.label)).toEqual(["高温", "cold"]);
    expect(m?.nodes).toEqual(["OBC", "BAT"]);
    expect(m?.cells.get(heatKey("hot", "OBC"))).toEqual({ temperature: 320, verdict: "warning" });
    expect(m?.min).toBe(260);
    expect(m?.max).toBe(320);
    expect(heatRatio(290, m!)).toBeCloseTo(0.5);
  });
  it("形が違えば null", () => {
    expect(heatMapOf({ ...out, cases: [] })).toBeNull();
    expect(heatMapOf(null)).toBeNull();
  });
});

describe("3D の塗り", () => {
  it("両端は accent と neg", async () => {
    const { heatHex } = await import("./heat");
    expect(heatHex(0)).toBe("#3382c2");
    expect(heatHex(1)).toBe("#b92626");
  });
});
