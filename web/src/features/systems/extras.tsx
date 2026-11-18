import type { ReactNode } from "react";
import { dimOf, EACH } from "@/analysis/output";
import { analysisOutput } from "@/api/analysis-output";
import type { AnalysisState } from "@/api/queries";
import type { AnalysisOutputs, DesignView, System } from "@/api/types";
import { LazyLayout3D } from "@/features/layout3d/LazyLayout3D";
import { hasExtras } from "./preload";
import type { SystemTab } from "./SystemTabs";
import { heatMapOf, partHeatColors } from "./thermal/heat";
import { ThermalHeatmap } from "./thermal/ThermalHeatmap";

function resultOf(states: readonly AnalysisState[], id: string) {
  return states.find((s) => s.id === id)?.result;
}

function outputOf<K extends keyof AnalysisOutputs>(
  states: readonly AnalysisState[],
  id: K,
): AnalysisOutputs[K] | null {
  return analysisOutput(states.find((s) => s.id === id)?.result, id);
}

/**
 * 系ごとの追加の表示（熱: 節点温度の表と温度で塗った 3D）。大きなチャンクは `preload.ts` の
 * `EXTRA_CHUNKS`（ナビの先読みと共通）で、下の分岐はその系をちょうど覆う（型で確かめる）。
 */
export function systemExtras(
  system: System,
  design: DesignView | undefined,
  states: readonly AnalysisState[],
): Partial<Record<SystemTab, ReactNode>> {
  if (!design) return {};
  if (!hasExtras(system)) return {};
  if (system === "thermal") {
    const map = heatMapOf(outputOf(states, "thermal.steady"));
    if (!map) return {};
    const colorOf = partHeatColors(resultOf(states, "thermal.steady")?.part_heat, map);
    return {
      results: (
        <>
          <ThermalHeatmap
            map={map}
            dimension={dimOf(resultOf(states, "thermal.steady"), [
              "cases",
              EACH,
              "nodes",
              EACH,
              "temperature",
            ])}
          />
          <LazyLayout3D variant="heat" design={design} colorOf={colorOf} />
        </>
      ),
    };
  }
  // 上の分岐が EXTRA_CHUNKS の系をすべて覆っていなければ、ここで型が合わない
  return system satisfies never;
}
