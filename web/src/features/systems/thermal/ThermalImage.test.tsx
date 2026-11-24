import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { renderToStaticMarkup } from "react-dom/server";
import { describe, expect, it } from "vitest";
import type { PartHeat, ThermalSteadyOutput, ThermalTransientOutput } from "@/api/types";
import { getT } from "@/i18n";
import { fixture } from "@/test/fixture";
import { rangeChartOf } from "./rangeChart";
import { NodeList, sourceOptions, ThermalImage } from "./ThermalImage";
import { ThermalRangeChart } from "./ThermalRangeChart";

const node = (n: string, temperature: number, verdict: "ok" | "warning" | "violation") => ({
  node: n,
  temperature,
  absorbed: 0,
  internal: 0,
  verdict,
  ranges: [
    {
      kind: "operating",
      range: { min: 250, max: 330 },
      margin_low: 10,
      margin_high: 5,
      verdict,
      excluded: [],
    },
  ],
});

const steady = {
  cases: [
    { id: "hot", label: "High", nodes: [node("OBC", 320, "warning")] },
    { id: "cold", label: "Low", nodes: [node("OBC", 260, "violation")] },
  ],
} as unknown as ThermalSteadyOutput;

const transient = {
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
    ],
  },
  nodes: [{ node: "OBC", min: 250, max: 330, ranges: [], verdict: "ok" }],
} as unknown as ThermalTransientOutput;

const wrap = (el: React.ReactNode) =>
  renderToStaticMarkup(<QueryClientProvider client={new QueryClient()}>{el}</QueryClientProvider>);

describe("温度範囲の図（描画）", () => {
  const model = rangeChartOf(steady, transient)!;
  it("節点の行と、ケースごとの印・非定常の棒を出し、判定は記号と名前で示す", () => {
    const html = wrap(<ThermalRangeChart model={model} kase={null} />);
    expect(html).toContain("OBC");
    expect(html).toContain("OBC High:");
    expect(html).toContain("OBC Low:");
    expect(html).toContain(getT().systems.rangeChart.transient);
    expect(html).toContain("!");
    expect(html).toContain("×");
  });
  it("選んだケースは強調し、ほかは薄くする（ケースが図に無ければ薄くしない）", () => {
    expect(wrap(<ThermalRangeChart model={model} kase="hot" />)).toContain("opacity-40");
    expect(wrap(<ThermalRangeChart model={model} kase="unknown" />)).not.toContain("opacity-40");
    expect(wrap(<ThermalRangeChart model={model} kase={null} />)).not.toContain("opacity-40");
  });
});

describe("熱画像（描画と出どころの選択肢）", () => {
  const labels = { worst: "W", steady: (n: string) => `S:${n}`, transient: "T" };
  it("選択肢は最悪・ケースごと・非定常（結果があるときだけ）", () => {
    expect(sourceOptions(steady, true, labels).map((o) => o.option.label)).toEqual([
      "W",
      "S:High",
      "S:Low",
      "T",
    ]);
    expect(sourceOptions(steady, false, labels).map((o) => o.option.value)).toEqual([
      "worst",
      "case-0",
      "case-1",
    ]);
    expect(sourceOptions(null, false, labels)).toHaveLength(1);
  });
  it("結果が無ければ案内を出し、あれば 3D を出す（選択と凡例は 3D の枠に渡す。3D は遅れて読むので、ここでは読み込み中）", () => {
    const I = getT().systems.thermalImage;
    expect(
      wrap(<ThermalImage design={fixture} steady={null} transient={null} heat={[]} />),
    ).toContain(I.empty);
    const html = wrap(
      <ThermalImage design={fixture} steady={steady} transient={transient} heat={[]} />,
    );
    expect(html).not.toContain(I.empty);
    expect(html).toContain('role="status"');
  });
  it("節点の一覧は、既定の 1 部品 1 節点を「部品ごと」と示し、まとめた節点は入っている部品を並べる", () => {
    const I = getT().systems.thermalImage;
    const q = { si: 320, display: "", unit: "K" } as PartHeat["temperature"];
    const heat: PartHeat[] = [
      { part: "OBC", node: "OBC", per_part: true, temperature: q, case: "hot", verdict: "ok" },
      { part: "A", node: "LUMP", per_part: false, temperature: q, case: "hot", verdict: "ok" },
      { part: "B", node: "LUMP", per_part: false, temperature: q, case: "hot", verdict: "ok" },
    ];
    const temps = new Map([
      ["OBC", 320],
      ["LUMP", 300],
    ]);
    const html = wrap(<NodeList temps={temps} heat={heat} scale={{ min: 250, max: 330 }} />);
    expect(html).toContain(`(${I.nodePerPart})`);
    expect(html).toContain("A, B");
  });
});
