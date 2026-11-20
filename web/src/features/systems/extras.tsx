import type { ReactNode } from "react";
import { analysisOutput } from "@/api/analysis-output";
import type { AnalysisState } from "@/api/queries";
import type { AnalysisOutputs, DesignView, System } from "@/api/types";
import { hasExtras } from "./preload";
import type { SystemTab } from "./SystemTabs";
import { rangeChartOf } from "./thermal/rangeChart";
import { ThermalImage } from "./thermal/ThermalImage";
import { ThermalRangeChart } from "./thermal/ThermalRangeChart";

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
 * 系ごとの追加の表示（熱: 結果のタブの温度範囲の図と、熱画像のタブの温度で塗った 3D）。大きなチャンクは `preload.ts` の
 * `EXTRA_CHUNKS`（ナビの先読みと共通）、追加のタブは `EXTRA_TABS`。下の分岐はその系をちょうど覆う（型で確かめる）。
 * `kase` はページの上で選んだケース（図で強調する）。
 */
export function systemExtras(
  system: System,
  design: DesignView | undefined,
  states: readonly AnalysisState[],
  kase: string | null = null,
): Partial<Record<SystemTab, ReactNode>> {
  if (!design) return {};
  if (!hasExtras(system)) return {};
  if (system === "thermal") {
    const steady = outputOf(states, "thermal.steady");
    const transient = outputOf(states, "thermal.transient");
    const model = rangeChartOf(steady, transient);
    return {
      results: model ? <ThermalRangeChart model={model} kase={kase} /> : null,
      heat: (
        <ThermalImage
          design={design}
          steady={steady}
          transient={transient}
          heat={resultOf(states, "thermal.steady")?.part_heat}
        />
      ),
    };
  }
  // 上の分岐が EXTRA_CHUNKS の系をすべて覆っていなければ、ここで型が合わない
  return system satisfies never;
}
