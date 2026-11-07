import { useQuery } from "@tanstack/react-query";
import { q } from "@/api/queries";
import type { System } from "@/api/types";
import { StatStrip } from "@/kit/StatStrip";
import { FieldValue } from "./FieldValue";
import { kpiDimension, kpiValue } from "./kpi";
import { type KpiSpec, kpiLabel } from "./systems";
import type { AnalysisState } from "./use-analyses";

/** 系のページ上部の主要 KPI */
export function SystemKpis({
  system,
  kpis,
  states,
}: {
  system: System;
  kpis: KpiSpec[];
  states: AnalysisState[];
}) {
  const budget = useQuery({ ...q.budget("power"), enabled: system === "power" });
  return (
    <StatStrip
      items={kpis.map((k) => {
        const v = kpiValue(k, states, budget.data?.mode);
        const loading = states.find((s) => s.id === k.analysis)?.loading;
        return {
          key: k.key,
          label: kpiLabel(system, k.key),
          value: loading ? (
            "…"
          ) : v === null ? (
            "—"
          ) : (
            <FieldValue field={k.field} value={v} dimension={kpiDimension(k, states)} />
          ),
        };
      })}
    />
  );
}
