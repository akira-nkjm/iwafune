import { describe, expect, it } from "vitest";
import { analysisOutput } from "./analysis-output";
import type { AnalysisRunResult, ModeLoadMode, ModeLoadOutput } from "./types";

const mode = (id: string, total: number): ModeLoadMode => ({
  id,
  label: id,
  total,
  total_eclipse: total,
  total_eclipse_with_growth: total,
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
  case_source: null,
  input: null,
  output,
  checks: [],
  verdict: null,
  kpis: [],
  missing: [],
  error: null,
  overview: { main: [], checks: [], chart: null, hidden_tables: [] },
  dims: {},
  labels: {},
};

describe("typed analysis outputs", () => {
  it("requires a matching ID and successful run", () => {
    expect(analysisOutput(result, "power.mode_load")).toBe(output);
    expect(analysisOutput(result, "thermal.steady")).toBeNull();
    expect(analysisOutput({ ...result, status: "failed" }, "power.mode_load")).toBeNull();
    expect(analysisOutput(undefined, "power.mode_load")).toBeNull();
  });
});
