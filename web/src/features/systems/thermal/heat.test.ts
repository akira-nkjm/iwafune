import { describe, expect, it } from "vitest";
import type { PartHeat, ThermalSteadyOutput, ThermalTransientOutput } from "@/api/types";
import { heatRatio } from "@/lib/heat-scale";
import { heatHex, toneHex } from "@/lib/tone";
import { heatScaleOf, nodeTemperatures, partHeatColors, transientTemperatures } from "./heat";

describe("熱画像の色", () => {
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
  const q = (si: number) => ({ si, display: "", unit: "K" }) as PartHeat["temperature"];
  const heat: PartHeat[] = [
    {
      part: "obc_a",
      node: "OBC",
      per_part: false,
      temperature: q(320),
      case: "hot",
      verdict: "warning",
    },
  ];
  const tr = {
    series: {
      t: [0, 60],
      channels: [
        {
          key: "OBC",
          kind: "node_temperature",
          subject: "OBC",
          dimension: "Temperature",
          values: [250, 330],
        },
        {
          key: "absorbed",
          kind: "absorbed_heat",
          subject: null,
          dimension: "Power",
          values: [1, 2],
        },
      ],
    },
  } as unknown as ThermalTransientOutput;
  const transient = transientTemperatures(tr);
  it("色の範囲は全ケースの定常と非定常の全時刻を通した最低・最高", () => {
    expect(heatScaleOf(out, null)).toEqual({ min: 260, max: 320 });
    expect(heatScaleOf(out, transient)).toEqual({ min: 250, max: 330 });
    expect(heatScaleOf(null, null)).toBeNull();
    expect(heatRatio(290, { min: 260, max: 320 })).toBeCloseTo(0.5);
    expect(heatRatio(290, { min: 290, max: 290 })).toBe(0.5);
  });
  it("非定常の系列は節点の温度の系列だけ取り出す", () => {
    expect(transientTemperatures(null)).toBeNull();
    expect(transientTemperatures(tr)?.nodes.get("OBC")).toEqual([250, 330]);
    expect(transientTemperatures(tr)?.nodes.has("absorbed")).toBe(false);
  });
  it("出どころごとの節点の温度（最悪は part_heat、ケースは定常、時刻は非定常の系列）", () => {
    expect(nodeTemperatures({ kind: "worst" }, out, transient, heat).get("OBC")).toBe(320);
    expect(nodeTemperatures({ kind: "case", id: "cold" }, out, transient, heat).get("OBC")).toBe(
      260,
    );
    expect(nodeTemperatures({ kind: "transient", index: 1 }, out, transient, heat).get("OBC")).toBe(
      330,
    );
    // 範囲外の時刻は端に丸める
    expect(
      nodeTemperatures({ kind: "transient", index: 99 }, out, transient, heat).get("OBC"),
    ).toBe(330);
    expect(nodeTemperatures({ kind: "case", id: "none" }, out, transient, heat).size).toBe(0);
  });
  it("3D の色は部品の節点（part_heat）の温度から引く。出どころを変えても範囲は同じ", () => {
    const scale = heatScaleOf(out, transient)!;
    const worst = partHeatColors(
      heat,
      nodeTemperatures({ kind: "worst" }, out, transient, heat),
      scale,
    );
    expect(worst("obc_a")).toBe(heatHex(heatRatio(320, scale)));
    const cold = partHeatColors(
      heat,
      nodeTemperatures({ kind: "case", id: "cold" }, out, transient, heat),
      scale,
    );
    expect(cold("obc_a")).toBe(heatHex(heatRatio(260, scale)));
    expect(cold("obc_a")).not.toBe(worst("obc_a"));
    // 節点の名前と部品の ID は同じでなくてよい。対応の無い部品・温度の無い節点は灰色
    expect(worst("OBC")).toBe(toneHex.gray);
    expect(cold("bat_1")).toBe(toneHex.gray);
    expect(partHeatColors(undefined, new Map(), scale)("obc_a")).toBe(toneHex.gray);
  });
});
