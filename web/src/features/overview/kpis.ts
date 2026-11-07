import type { System } from "@/api/types";
import { systems } from "@/features/systems/systems";

/** 概要に出す系の KPI（どの系の、どの KPI か）。解析 ID は各系の KPI の定義から引く */
export const OVERVIEW_KPIS: readonly { system: System; key: string }[] = [
  { system: "power", key: "balance" },
  { system: "power", key: "dod" },
  { system: "attitude", key: "interval" },
  { system: "attitude", key: "peak" },
  { system: "cdh", key: "margin" },
  { system: "cdh", key: "total" },
  { system: "comm", key: "margin" },
  { system: "thermal", key: "max" },
  { system: "thermal", key: "min" },
];

export function kpiOf(system: System, key: string) {
  const spec = systems[system].kpis.find((k) => k.key === key);
  if (!spec) throw new Error(`KPI ${system}.${key} not found`);
  return spec;
}

/** 概要で実行する解析（KPI が指す解析だけ） */
export const OVERVIEW_ANALYSES: readonly string[] = [
  ...new Set(OVERVIEW_KPIS.map((k) => kpiOf(k.system, k.key).analysis)),
];
