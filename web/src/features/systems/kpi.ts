import { analysisOutput } from "@/api/analysis-output";
import { dimOf, pickNumbers } from "./output";
import type { KpiSpec } from "./systems";
import type { AnalysisState } from "./use-analyses";

/** KPI の値を出力から引く（集め方は spec の pick。値の計算はしない） */
export function kpiValue(
  spec: KpiSpec,
  states: readonly AnalysisState[],
  budgetMode?: string | null,
): number | null {
  const result = states.find((s) => s.id === spec.analysis)?.result;
  const out = result?.output;
  if (spec.analysis === "power.mode_load" && spec.key === "load") {
    return (
      analysisOutput(result, "power.mode_load")?.modes.find((m) => m.id === budgetMode)
        ?.total_with_growth ?? null
    );
  }
  if (!out) return null;
  const xs = (Array.isArray(spec.path) ? spec.path : [spec.path]).flatMap((p) =>
    pickNumbers(out, p),
  );
  if (xs.length === 0) return null;
  return spec.pick === "max"
    ? Math.max(...xs)
    : spec.pick === "min"
      ? Math.min(...xs)
      : (xs[0] ?? null);
}

/** KPI の値の次元（解析の API の dims から。最初のパスで引く） */
export function kpiDimension(spec: KpiSpec, states: readonly AnalysisState[]): string | null {
  const dims = states.find((s) => s.id === spec.analysis)?.result?.dims;
  const path = Array.isArray(spec.path) ? spec.path[0] : spec.path;
  return path ? dimOf(dims, path) : null;
}
