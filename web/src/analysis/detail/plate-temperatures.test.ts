import { describe, expect, it } from "vitest";
import type { ThermalPlateTemperatures } from "@/api/types";
import { cellRatios, detailScale, fieldChoices, plateRows } from "./plate-temperatures";

const labels = {
  steady: (n: string) => `Steady: ${n}`,
  quantity: { steady: "Steady", min: "Min", max: "Max", end: "End" },
};

/** 2 枚の板（A は 2 × 1、B は 1 × 2）の升 4 つと、定常の 2 ケース */
const detail: ThermalPlateTemperatures = {
  mesh: { nodes: [], groups: [] },
  cells: [
    { node: "A[0,0]", cell: { part: "A", i: 0, j: 0 } },
    { node: "A[1,0]", cell: { part: "A", i: 1, j: 0 } },
    { node: "B[0,0]", cell: { part: "B", i: 0, j: 0 } },
    { node: "B[0,1]", cell: { part: "B", i: 0, j: 1 } },
  ],
  fields: [
    { quantity: "steady", case: "hot", label: "Hot", temperature: [300, 310, 320, 330] },
    { quantity: "steady", case: "cold", label: "", temperature: [250, 260, 270, 280] },
  ],
};

describe("板の温度の分布の選択と色", () => {
  it("場の選択肢は API の並びのまま、定常はケースの名前（ラベルが空ならケースの ID）", () => {
    const c = fieldChoices(detail.fields, labels);
    expect(c.map((x) => x.label)).toEqual(["Steady: Hot", "Steady: cold"]);
    expect(c.map((x) => x.value)).toEqual(["0", "1"]);
  });

  it("非定常は量の名前", () => {
    const c = fieldChoices(
      (["min", "max", "end"] as const).map((quantity) => ({ quantity, temperature: [1] })),
      labels,
    );
    expect(c.map((x) => x.label)).toEqual(["Min", "Max", "End"]);
  });

  it("色の範囲は全部の場を通した最低〜最高なので、場を切り替えても同じ温度は同じ色", () => {
    const scale = detailScale(detail);
    expect(scale).toEqual({ min: 250, max: 330 });
    if (!scale) return;
    const [hot, cold] = detail.fields;
    if (!hot || !cold) return;
    expect(cellRatios(hot, scale)).toEqual([0.625, 0.75, 0.875, 1]);
    expect(cellRatios(cold, scale)).toEqual([0, 0.125, 0.25, 0.375]);
  });

  it("一覧は板ごと、升の通し番号（四角形の番号）と温度つき", () => {
    const [hot] = detail.fields;
    if (!hot) return;
    const rows = plateRows(detail, hot);
    expect(rows.map((r) => [r.part, r.size])).toEqual([
      ["A", [2, 1]],
      ["B", [1, 2]],
    ]);
    expect(rows[1]?.cells).toEqual([
      { index: 2, node: "B[0,0]", temperature: 320 },
      { index: 3, node: "B[0,1]", temperature: 330 },
    ]);
  });
});
