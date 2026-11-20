import { describe, expect, it } from "vitest";
import type { PartHeat, ThermalSteadyOutput } from "@/api/types";
import { heatHex, toneHex } from "@/lib/tone";
import { heatKey, heatMapOf, heatRatio, partHeatColors } from "./heat";

describe("熱の温度表", () => {
  const environment = {
    preset: null,
    solar_flux: null,
    albedo: 0.3,
    planet_ir: 237,
    space_temperature: 3,
  };
  const out: ThermalSteadyOutput = {
    envelope: [],
    margin: { uncertainty: 0, warning_band: 5, heater_duty_warning: 0.8 },
    couplings: { conductive: [], radiative: [], enclosure: null },
    checks: [],
    cases: [
      {
        id: "hot",
        label: "高温",
        environment,
        heaters: [],
        iterations: 1,
        residual: 0,
        nodes: [
          {
            node: "OBC",
            temperature: 320,
            absorbed: 0,
            internal: 0,
            ranges: [],
            verdict: "warning",
          },
          {
            node: "BAT",
            temperature: 300,
            absorbed: 0,
            internal: 0,
            ranges: [],
            verdict: "ok",
          },
        ],
      },
      {
        id: "cold",
        label: "",
        environment,
        heaters: [],
        iterations: 1,
        residual: 0,
        nodes: [
          {
            node: "OBC",
            temperature: 260,
            absorbed: 0,
            internal: 0,
            ranges: [],
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
    expect(m?.band).toBe(5);
  });
  it("形が違えば null", () => {
    expect(heatMapOf({ ...out, cases: [] })).toBeNull();
    expect(heatMapOf(null)).toBeNull();
  });
  it("3D の色は API の部品ごとの温度から引く（節点の名前と部品の ID は同じでなくてよい）", () => {
    const m = heatMapOf(out)!;
    const q = (si: number) => ({ si, display: "", unit: "K" }) as PartHeat["temperature"];
    const colorOf = partHeatColors(
      [
        { part: "obc_a", node: "OBC", temperature: q(320), case: "hot", verdict: "warning" },
        { part: "bat_1", node: "BAT", temperature: q(260), case: "cold", verdict: "ok" },
      ],
      m,
    );
    expect(colorOf("obc_a")).toBe(heatHex(1));
    expect(colorOf("bat_1")).toBe(heatHex(0));
    expect(colorOf("OBC")).toBe(toneHex.gray);
    expect(partHeatColors(undefined, m)("obc_a")).toBe(toneHex.gray);
  });
});
