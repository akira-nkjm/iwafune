import { describe, expect, it } from "vitest";
import { kpiValue } from "@/features/systems/kpi";
import { systems } from "@/features/systems/systems";
import { analysisOutput } from "./analysis-output";
import type { AnalysisRunResult, ModeLoadMode, ModeLoadOutput } from "./types";

const mode = (id: string, total: number): ModeLoadMode => ({
  id,
  label: id,
  total,
  total_with_growth: total,
  peak_with_growth: total,
  groups: [],
  parts: [],
  unassigned: [],
  missing_power: [],
  checks: [],
});
const output: ModeLoadOutput = { modes: [mode("larger", 100), mode("budget_choice", 20)] };
const result: AnalysisRunResult = {
  id: "power.mode_load",
  status: "ok",
  case: null,
  input: null,
  output,
  checks: [],
  verdict: null,
  missing: [],
  error: null,
  dims: {},
};

describe("typed analysis outputs", () => {
  it("requires a matching ID and successful run", () => {
    expect(analysisOutput(result, "power.mode_load")).toBe(output);
    expect(analysisOutput(result, "thermal.steady")).toBeNull();
    expect(analysisOutput({ ...result, status: "failed" }, "power.mode_load")).toBeNull();
    expect(analysisOutput(undefined, "power.mode_load")).toBeNull();
  });
  it("uses the budget's chosen mode without picking the largest again", () => {
    const spec = systems.power.kpis.find((k) => k.key === "load");
    expect(spec).toBeDefined();
    if (!spec) return;
    const states = [{ id: result.id, result, error: null, loading: false, updating: false }];
    expect(kpiValue(spec, states, "budget_choice")).toBe(20);
    expect(kpiValue(spec, states, "absent")).toBeNull();
  });
});
