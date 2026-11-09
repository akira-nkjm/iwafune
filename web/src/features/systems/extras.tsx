import type { ReactNode } from "react";
import { analysisOutput } from "@/api/analysis-output";
import type { AnalysisOutputs, DesignView, System } from "@/api/types";
import { LazyLayout3D } from "@/features/layout3d/LazyLayout3D";
import type { V3 } from "@/features/layout3d/scene";
import { getT } from "@/i18n";
import { toneHex } from "@/lib/tone";
import { heatHex, heatKey, heatMapOf, heatRatio } from "./heat";
import { dimOf, EACH } from "./output";
import type { SystemTab } from "./SystemTabs";
import { ThermalHeatmap } from "./ThermalHeatmap";
import type { AnalysisState } from "./use-analyses";

function resultOf(states: readonly AnalysisState[], id: string) {
  return states.find((s) => s.id === id)?.result;
}

function outputOf<K extends keyof AnalysisOutputs>(
  states: readonly AnalysisState[],
  id: K,
): AnalysisOutputs[K] | null {
  return analysisOutput(states.find((s) => s.id === id)?.result, id);
}

/** 系ごとの追加の表示（構造: 3D 配置と重心、熱: 節点温度の表と温度で塗った 3D） */
export function systemExtras(
  system: System,
  design: DesignView | undefined,
  states: readonly AnalysisState[],
): Partial<Record<SystemTab, ReactNode>> {
  if (!design) return {};
  if (system === "structure") {
    const mp = outputOf(states, "structure.mass_properties");
    const c = mp?.summary.center_of_mass;
    const com: V3 | null = c ? [c.x, c.y, c.z] : null;
    return { results: <LazyLayout3D design={design} com={com} /> };
  }
  if (system === "thermal") {
    const map = heatMapOf(outputOf(states, "thermal.steady"));
    if (!map) return {};
    const hottest = (node: string) => {
      const ts = map.cases.flatMap((c) => {
        const cell = map.cells.get(heatKey(c.id, node));
        return cell ? [cell.temperature] : [];
      });
      return ts.length > 0 ? Math.max(...ts) : null;
    };
    const colorOf = (id: string) => {
      const t = hottest(id);
      return t === null ? toneHex.gray : heatHex(heatRatio(t, map));
    };
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
          <LazyLayout3D
            design={design}
            colorOf={colorOf}
            editable={false}
            title={getT().systems.thermal3d}
            legend={getT().systems.thermal3dLegend}
          />
        </>
      ),
    };
  }
  return {};
}
