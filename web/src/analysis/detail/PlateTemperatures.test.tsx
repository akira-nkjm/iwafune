import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { renderToStaticMarkup } from "react-dom/server";
import { describe, expect, it } from "vitest";
import type { AnalysisRunResult } from "@/api/types";
import { hasDetailView } from "./AnalysisDetail";
import { CellList, FieldSelect } from "./PlateTemperatures";
import type { FieldChoice, PlateRows } from "./plate-temperatures";

const wrap = (node: React.ReactNode) =>
  renderToStaticMarkup(
    <QueryClientProvider client={new QueryClient()}>{node}</QueryClientProvider>,
  );

const choice = (value: string, label: string): FieldChoice => ({
  value,
  label,
  field: { quantity: "steady", temperature: [] },
});

describe("場の選択", () => {
  it("選択肢が 2 つ以上なら出し、選んだ値が選択状態", () => {
    const out = wrap(
      <FieldSelect
        choices={[choice("0", "Steady: Hot"), choice("1", "Steady: Cold")]}
        value="1"
        onChange={() => {}}
      />,
    );
    expect(out).toContain("Steady: Hot");
    expect(out).toContain("Steady: Cold");
    expect(out).toMatch(/<option value="1" selected/);
  });

  it("選択肢が 1 つなら選択は出さない", () => {
    expect(
      wrap(<FieldSelect choices={[choice("0", "Only")]} value="0" onChange={() => {}} />),
    ).toBe("");
  });
});

describe("升の一覧", () => {
  const rows: PlateRows[] = [
    {
      part: "PANEL_PX",
      size: [3, 5],
      cells: [
        { index: 0, node: "PANEL_PX[0,0]", temperature: 280 },
        { index: 1, node: "PANEL_PX[0,1]", temperature: 340 },
      ],
    },
  ];
  it("板の名前と升の数、升の名前を出し、重ねた升だけ強調する", () => {
    const out = wrap(
      <CellList rows={rows} scale={{ min: 280, max: 340 }} hover={1} onHover={() => {}} />,
    );
    expect(out).toContain("PANEL_PX（3 × 5）");
    expect(out).toContain("PANEL_PX[0,0]");
    expect(out.match(/bg-accent-tint/g)).toHaveLength(1);
    expect(out.indexOf("bg-accent-tint")).toBeGreaterThan(out.indexOf("PANEL_PX[0,0]"));
  });
});

describe("板の温度の詳細を出すか", () => {
  const result = (id: string, plates: unknown[] | undefined) =>
    ({
      id,
      status: "ok",
      output: { couplings: plates ? { plates } : {} },
    }) as unknown as AnalysisRunResult;

  it("升に分けた板がある熱の結果だけ（定常も非定常も）", () => {
    expect(hasDetailView("ThermalPlateTemperatures", result("thermal.steady", [{}]))).toBe(true);
    expect(hasDetailView("ThermalPlateTemperatures", result("thermal.transient", [{}]))).toBe(true);
    expect(hasDetailView("ThermalPlateTemperatures", result("thermal.steady", []))).toBe(false);
    expect(hasDetailView("ThermalPlateTemperatures", result("thermal.steady", undefined))).toBe(
      false,
    );
  });

  it("モードの形は結果によらず、未登録の型名と型名なしは出さない", () => {
    expect(hasDetailView("ModalFemShapes", result("structure.modal_fem", undefined))).toBe(true);
    expect(hasDetailView("Unknown", result("x", undefined))).toBe(false);
    expect(hasDetailView(undefined, result("x", undefined))).toBe(false);
  });
});
