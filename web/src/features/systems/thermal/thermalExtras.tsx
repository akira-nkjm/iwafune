import { analysisOutput } from "@/api/analysis-output";
import type { AnalysisState } from "@/api/queries";
import type { AnalysisOutputs, DesignView } from "@/api/types";
import type { Extras } from "../dedicated";
import { rangeChartOf } from "./rangeChart";
import { ThermalImage } from "./ThermalImage";
import { ThermalRangeChart } from "./ThermalRangeChart";
import { ThermalNetworkTab } from "./tables/ThermalNetworkTab";

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
 * 熱のページの追加の表示（結果のタブの温度範囲の図・結合と表面のタブ・熱画像のタブの温度で塗った 3D）。
 * 登録表（`dedicated.tsx` の `SYSTEM_REGISTRY.thermal.extras`）から呼ばれる。
 */
export function thermalExtras(
  design: DesignView,
  states: readonly AnalysisState[],
  kase: string | null,
): Extras {
  const steady = outputOf(states, "thermal.steady");
  const transient = outputOf(states, "thermal.transient");
  const model = rangeChartOf(steady, transient);
  return {
    results: model ? <ThermalRangeChart model={model} kase={kase} /> : null,
    couplings: <ThermalNetworkTab kase={kase} />,
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
