import { describe, expect, it } from "vitest";
import type { ThermalRangeMargin, ThermalSteadyOutput, ThermalTransientOutput } from "@/api/types";
import { axisPosition, axisTicks, rangeChartOf } from "./rangeChart";

const range = (kind: ThermalRangeMargin["kind"], min: number, max: number): ThermalRangeMargin => ({
  kind,
  range: { min, max },
  margin_low: 1,
  margin_high: 2,
  verdict: "ok",
  excluded: [],
});

const node = (n: string, temperature: number, ranges: ThermalRangeMargin[]) => ({
  node: n,
  temperature,
  absorbed: 0,
  internal: 0,
  ranges,
  verdict: "warning" as const,
});

const steady = {
  cases: [
    {
      id: "hot",
      label: "Hot",
      nodes: [node("OBC", 320, [range("operating", 250, 330)]), node("FREE", 300, [])],
    },
    {
      id: "cold",
      label: "",
      nodes: [node("OBC", 260, [range("operating", 250, 330), range("storage", 230, 350)])],
    },
  ],
} as unknown as ThermalSteadyOutput;

const transient = {
  nodes: [
    {
      node: "OBC",
      min: 255,
      max: 335,
      ranges: [range("operating", 250, 330)],
      verdict: "violation",
    },
    { node: "ONLY_TR", min: 280, max: 290, ranges: [], verdict: "ok" },
  ],
} as unknown as ThermalTransientOutput;

describe("温度範囲の図の元データ", () => {
  it("行は定常の節点の順、非定常だけの節点は後ろ。範囲が無い節点にも行がある", () => {
    const m = rangeChartOf(steady, transient)!;
    expect(m.rows.map((r) => r.node)).toEqual(["OBC", "FREE", "ONLY_TR"]);
    expect(m.caseIds).toEqual(["hot", "cold"]);
    expect(m.rows[1]?.bands).toEqual([]);
  });
  it("印は API の判定をそのまま持ち、ケースの名前が無ければ ID", () => {
    const obc = rangeChartOf(steady, null)?.rows[0];
    expect(obc?.markers.map((x) => [x.label, x.temperature, x.verdict])).toEqual([
      ["Hot", 320, "warning"],
      ["cold", 260, "warning"],
    ]);
    expect(obc?.span).toBeNull();
  });
  it("帯は保管・動作・性能の順で、ケースをまたいで種類ごとに 1 つ", () => {
    const obc = rangeChartOf(steady, transient)?.rows[0];
    expect(obc?.bands.map((b) => b.kind)).toEqual(["storage", "operating"]);
    expect(obc?.span).toMatchObject({ min: 255, max: 335, verdict: "violation" });
  });
  it("横軸の端はデータと帯を含み、少し余白を足す", () => {
    const m = rangeChartOf(steady, transient)!;
    expect(m.lo).toBeLessThan(230);
    expect(m.hi).toBeGreaterThan(350);
    expect(m.lo).toBeGreaterThan(200);
  });
  it("どちらの出力も無ければ null", () => {
    expect(rangeChartOf(null, null)).toBeNull();
  });
  it("値の位置は端に丸め、目盛りは端を含めて等間隔", () => {
    expect(axisPosition(15, 10, 20)).toBeCloseTo(0.5);
    expect(axisPosition(5, 10, 20)).toBe(0);
    expect(axisPosition(25, 10, 20)).toBe(1);
    expect(axisPosition(1, 3, 3)).toBe(0.5);
    expect(axisTicks(0, 100, 5)).toEqual([0, 25, 50, 75, 100]);
  });
});
