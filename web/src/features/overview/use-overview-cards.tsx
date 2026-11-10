import { useQuery } from "@tanstack/react-query";
import type { ReactNode } from "react";
import { q } from "@/api/queries";
import type { BudgetView, System } from "@/api/types";
import { FieldValue } from "@/features/systems/FieldValue";
import { kpiDimension, kpiValue } from "@/features/systems/kpi";
import { useAnalyses } from "@/features/systems/use-analyses";
import { type Dict, useT } from "@/i18n";
import type { TickTone } from "@/kit/TickBar";
import { verdictTone } from "@/kit/verdict-tone";
import { kpiOf, OVERVIEW_ANALYSES } from "./kpis";

export interface OverviewCard {
  label: string;
  value: ReactNode;
  ratio: number | null;
  tone: TickTone;
  footer: ReactNode;
  footerRight: ReactNode;
  to: string;
  loading: boolean;
}

/** 塗りの長さ（上限に対する割合）。表示のための比で、値は API の SI 値 */
function budgetRatio(b: BudgetView | undefined): number | null {
  return b?.total && b.limit && b.limit.si > 0 ? b.total.si / b.limit.si : null;
}

/** 解析の結果を KPI・判定として読む小さな道具 */
function useKpiReader() {
  const states = useAnalyses(OVERVIEW_ANALYSES, null);
  const analysisOf = (system: System, key: string) => kpiOf(system, key).analysis;
  const value = (system: System, key: string) => kpiValue(kpiOf(system, key), states);
  const text = (system: System, key: string): ReactNode => {
    const x = value(system, key);
    const spec = kpiOf(system, key);
    return x === null ? (
      "—"
    ) : (
      <FieldValue field={spec.field} value={x} dimension={kpiDimension(spec, states)} />
    );
  };
  const checks = (system: System, key: string, prefix?: string) => {
    const checks = states.find((s) => s.id === analysisOf(system, key))?.result?.checks ?? [];
    return checks.filter((c) => !prefix || c.key.startsWith(prefix));
  };
  const tone = (system: System, key: string): TickTone =>
    verdictTone[states.find((s) => s.id === analysisOf(system, key))?.result?.verdict ?? "ok"];
  const loading = (system: System, key: string) =>
    states.find((s) => s.id === analysisOf(system, key))?.loading ?? true;
  return { value, text, checks, tone, loading };
}

function analysisCards(o: Dict["overview"], k: ReturnType<typeof useKpiReader>): OverviewCard[] {
  const thermal = k.checks("thermal", "max");
  const bad = thermal.filter((c) => c.verdict !== "ok").length;
  const card = (
    system: System,
    key: string,
    rest: Omit<OverviewCard, "value" | "loading" | "to"> & { to?: string },
  ) => ({
    value: k.text(system, key),
    loading: k.loading(system, key),
    to: rest.to ?? `/systems/${system}`,
    ...rest,
  });
  return [
    card("power", "dod", {
      label: o.maxDod,
      ratio: k.value("power", "dod"),
      tone: k.tone("power", "dod"),
      footer: o.dodFooter,
      footerRight: "",
    }),
    card("attitude", "interval", {
      label: o.rwInterval,
      ratio: null,
      tone: k.tone("attitude", "interval"),
      footer: (
        <>
          {o.peakMomentum} {k.text("attitude", "peak")}
        </>
      ),
      footerRight: "",
    }),
    card("cdh", "margin", {
      label: o.storageMargin,
      ratio: k.value("cdh", "margin"),
      tone: k.tone("cdh", "margin"),
      footer: (
        <>
          {o.perDay} {k.text("cdh", "total")}
        </>
      ),
      footerRight: "",
    }),
    card("comm", "margin", {
      label: o.linkMargin,
      ratio: null,
      tone: k.tone("comm", "margin"),
      footer: "",
      footerRight: "",
    }),
    {
      ...card("thermal", "max", {
        label: o.temperature,
        ratio: thermal.length === 0 ? null : (thermal.length - bad) / thermal.length,
        tone: k.tone("thermal", "max"),
        footer: (
          <>
            {o.tempMax} {k.text("thermal", "max")} / {o.tempMin} {k.text("thermal", "min")}
          </>
        ),
        footerRight: "",
      }),
      value: thermal.length === 0 ? "—" : bad === 0 ? o.allInRange : o.nodesToCheck(bad),
    },
  ];
}

/** 概要の KPI カード（質量余裕・電力収支はバジェット、残りは系の解析の KPI） */
export function useOverviewCards(): OverviewCard[] {
  const o = useT().overview;
  const mass = useQuery(q.budget("mass"));
  const power = useQuery(q.budget("power"));
  const k = useKpiReader();
  return [
    {
      label: o.massMargin,
      value: mass.data?.margin?.display ?? "—",
      ratio: budgetRatio(mass.data),
      tone: mass.data?.verdict ? verdictTone[mass.data.verdict] : "accent",
      footer: mass.data?.total ? o.massTotal(mass.data.total.display) : o.massBudget,
      footerRight: mass.data?.limit ? o.limit(mass.data.limit.display) : "",
      to: "/systems/structure",
      loading: mass.isPending,
    },
    {
      label: o.powerBalance,
      value: k.text("power", "balance"),
      ratio: budgetRatio(power.data),
      tone: k.tone("power", "balance"),
      footer: power.data?.total ? o.consumption(power.data.total.display) : "",
      footerRight: power.data?.limit ? o.generationLimit(power.data.limit.display) : "",
      to: "/systems/power",
      loading: k.loading("power", "balance"),
    },
    ...analysisCards(o, k),
  ];
}
