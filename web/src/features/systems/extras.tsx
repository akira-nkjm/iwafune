import type { ReactNode } from "react";
import { analysisOutput } from "@/api/analysis-output";
import type { AnalysisState } from "@/api/queries";
import type { AnalysisOutputs, DesignView, System } from "@/api/types";
import { type ExtraSystem, hasExtras } from "./preload";
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

type Extras = Partial<Record<SystemTab, ReactNode>>;

/** 追加の表示の作り方。`EXTRA_CHUNKS` の系をちょうど覆う（`Record<ExtraSystem, …>` の型で確かめる） */
const EXTRAS: Record<
  ExtraSystem,
  (design: DesignView, states: readonly AnalysisState[], kase: string | null) => Extras
> = {
  thermal: (design, states, kase) => {
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
  },
};

/**
 * 系ごとの追加の表示（熱: 結果のタブの温度範囲の図と、熱画像のタブの温度で塗った 3D）。大きなチャンクは `preload.ts` の
 * `EXTRA_CHUNKS`（ナビの先読みと共通）、追加のタブは `EXTRA_TABS`、作り方は上の `EXTRAS`（系の名前で比べずに表で引く）。
 * `kase` はページの上で選んだケース（図で強調する）。
 */
export function systemExtras(
  system: System,
  design: DesignView | undefined,
  states: readonly AnalysisState[],
  kase: string | null = null,
): Extras {
  if (!design || !hasExtras(system)) return {};
  return EXTRAS[system](design, states, kase);
}
