import { FieldValue } from "@/analysis/FieldValue";
import { useT } from "@/i18n";
import { Panel } from "@/kit/Panel";
import { PanelHeader } from "@/kit/PanelHeader";
import { cn } from "@/lib/cn";
import { heatHex } from "@/lib/tone";
import { Formatted } from "@/quantity/Formatted";
import { type HeatMap, heatKey, heatRatio } from "./heat";

const ring = {
  ok: "",
  warning: "ring-2 ring-warn ring-inset",
  violation: "ring-2 ring-neg ring-inset",
} as const;

/** 節点温度のヒートマップ（ケース × 節点）。枠の色が判定（注意・違反）、塗りが表の中での温度の高低 */
export function ThermalHeatmap({ map, dimension }: { map: HeatMap; dimension: string | null }) {
  const t = useT();
  return (
    <Panel>
      <PanelHeader
        title={t.systems.heatTitle}
        caption={
          <>
            <FieldValue field="temperature" value={map.min} dimension={dimension} />
            {t.systems.rangeSep}
            <FieldValue field="temperature" value={map.max} dimension={dimension} /> ·{" "}
            {t.systems.heatLegendBefore}{" "}
            {map.band !== null && (
              <Formatted
                si={map.band}
                dimension="Temperature"
                unit="K"
                fallback={String(map.band)}
              />
            )}{" "}
            {t.systems.heatLegendAfter}
          </>
        }
      />
      <div className="overflow-x-auto">
        <table className="border-separate border-spacing-0.5 text-[12px]">
          <thead>
            <tr>
              <th className="px-2 text-left font-medium text-[11px] text-ink-3">
                {t.systems.node}
              </th>
              {map.cases.map((c) => (
                <th key={c.id} className="px-2 text-left font-medium text-[11px] text-ink-3">
                  {c.label}
                </th>
              ))}
            </tr>
          </thead>
          <tbody>
            {map.nodes.map((n) => (
              <tr key={n}>
                <td className="pr-3 font-mono text-ink-2">{n}</td>
                {map.cases.map((c) => {
                  const cell = map.cells.get(heatKey(c.id, n));
                  if (!cell)
                    return (
                      <td key={c.id} className="rounded-[6px] bg-panel-2 px-2 py-1 text-ink-4">
                        —
                      </td>
                    );
                  return (
                    <td
                      key={c.id}
                      className={cn(
                        "num min-w-24 rounded-[6px] px-2 py-1 text-right text-inverse",
                        cell.verdict && ring[cell.verdict],
                      )}
                      style={{ background: heatHex(heatRatio(cell.temperature, map)) }}
                      title={cell.verdict ?? undefined}
                    >
                      <FieldValue
                        field="temperature"
                        value={cell.temperature}
                        dimension={dimension}
                      />
                    </td>
                  );
                })}
              </tr>
            ))}
          </tbody>
        </table>
      </div>
    </Panel>
  );
}
