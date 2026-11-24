import { describe, expect, it } from "vitest";
import type { AnalysisRunResult } from "@/api/types";
import { findingStates, geometryFindings } from "./geometry-findings";

/** 結果の形（テストが使う欄だけ）。型は API のもの */
function result(id: string, output: unknown, extra: object = {}): AnalysisRunResult {
  return { id, status: "ok", output, part_findings: [], ...extra } as unknown as AnalysisRunResult;
}

const overlap = (first: string, second: string, extra: object = {}) => ({
  first,
  second,
  depth: { value: 1, unit: "mm" },
  acknowledged: false,
  ...extra,
});

describe("geometryFindings", () => {
  it("干渉は重なっている組を出力の順に写し、承知済みは理由と形態を持つ", () => {
    const r = result("structure.interference", {
      overlaps: [
        overlap("A", "B"),
        overlap("C", "D", {
          acknowledged: true,
          reason: "Hinge edge",
          acknowledged_configuration: "STOWED",
        }),
      ],
    });
    expect(geometryFindings(r)).toEqual([
      {
        kind: "interference",
        first: "A",
        second: "B",
        acknowledged: false,
        reason: null,
        acknowledgedConfiguration: null,
      },
      {
        kind: "interference",
        first: "C",
        second: "D",
        acknowledged: true,
        reason: "Hinge edge",
        acknowledgedConfiguration: "STOWED",
      },
    ]);
  });

  it("円錐は担い手と、円錐に入る部品ごとに 1 件（担い手が先）", () => {
    const r = result("structure.cone_clearance", {
      cones: [
        {
          id: "SS_PX",
          obstructions: [
            { part: "SAP_PY", acknowledged: true, reason: "Folded" },
            { part: "CELL_1", acknowledged: false },
          ],
        },
        { id: "STT", obstructions: [] },
      ],
    });
    expect(geometryFindings(r).map((f) => [f.kind, f.first, f.second, f.acknowledged])).toEqual([
      ["cone_clearance", "SS_PX", "SAP_PY", true],
      ["cone_clearance", "SS_PX", "CELL_1", false],
    ]);
  });

  it("ほかの解析・入力不足・結果なしは空", () => {
    expect(
      geometryFindings(result("power.energy_balance", { overlaps: [overlap("A", "B")] })),
    ).toEqual([]);
    expect(
      geometryFindings(result("structure.interference", null, { status: "missing_inputs" })),
    ).toEqual([]);
    expect(geometryFindings(null)).toEqual([]);
    expect(geometryFindings(undefined)).toEqual([]);
  });
});

describe("findingStates", () => {
  it("API の部品の状態を表にし、2 つの解析に載る部品は承知でない所見が残るほうを残す", () => {
    const a = result("structure.interference", null, {
      part_findings: [
        { part: "A", state: "acknowledged" },
        { part: "B", state: "acknowledged" },
      ],
    });
    const b = result("structure.cone_clearance", null, {
      part_findings: [
        { part: "B", state: "open" },
        { part: "C", state: "acknowledged" },
      ],
    });
    expect([...findingStates([a, b])]).toEqual([
      ["A", "acknowledged"],
      ["B", "open"],
      ["C", "acknowledged"],
    ]);
    // 順序を入れ替えても同じ
    expect(findingStates([b, a]).get("B")).toBe("open");
    expect(findingStates([]).size).toBe(0);
  });
});
