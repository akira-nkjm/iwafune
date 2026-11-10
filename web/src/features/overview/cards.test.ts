import { describe, expect, it } from "vitest";
import type { KpiSlot } from "@/analysis/kpi";
import type { BudgetView, KpiKey, KpiView, QuantityView, System } from "@/api/types";
import { dictionaries } from "@/i18n";
import { type OverviewSources, overviewCards } from "./cards";

const o = dictionaries.ja.overview;

const qty = (display: string): QuantityView => ({
  value: 0,
  unit: "",
  si: 0,
  dimension: "Dimensionless",
  display,
});

const kpi = (key: KpiKey, rest: Partial<KpiView>): KpiView => ({
  key,
  label: key,
  label_en: key,
  value: null,
  verdict: null,
  fill: null,
  tally: null,
  ...rest,
});

const slot = (system: System, k: KpiView, loading = false): KpiSlot => ({
  def: { key: k.key, label: k.label, label_en: k.label_en },
  analysis: `${system}.x`,
  system,
  kpi: loading ? null : k,
  loading,
});

const budget = (rest: Partial<BudgetView>): BudgetView => ({
  kind: "mass",
  system: "structure",
  label: "質量",
  label_en: "Mass",
  case: null,
  limit: qty("12.0 kg"),
  system_margin: qty("10.0 %"),
  total: qty("5.64 kg"),
  margin: qty("5.16 kg"),
  verdict: "ok",
  fill: { ratio: 0.47, basis: "used_over_limit" },
  rows: [],
  modes: [],
  analysis: "budget",
  mode: null,
  missing: [],
  notes: [],
  ...rest,
});

const slots: KpiSlot[] = [
  slot("power", kpi("orbit_balance", { value: qty("-5.09 W"), verdict: "violation" })),
  slot(
    "power",
    kpi("max_dod", {
      value: qty("100 %"),
      verdict: "violation",
      fill: { ratio: 1, basis: "value" },
    }),
  ),
  slot("attitude", kpi("unload_interval", { value: qty("16.4 h"), verdict: "ok" })),
  slot("attitude", kpi("peak_momentum", { value: qty("2.32 mN·m·s") })),
  slot("cdh", kpi("storage_margin", { value: qty("-3.61 %"), fill: { ratio: 0, basis: "value" } })),
  slot("cdh", kpi("data_per_day", { value: qty("2.77 GB") }), true),
  slot("comm", kpi("link_margin", { value: qty("3.05 dB"), verdict: "ok" })),
  slot(
    "thermal",
    kpi("nodes_in_range", {
      verdict: "violation",
      tally: { checked: 9, flagged: 4 },
      fill: { ratio: 5 / 9, basis: "ok_share" },
    }),
  ),
  slot("thermal", kpi("max_temperature", { value: qty("50.5 °C") })),
  slot("thermal", kpi("min_temperature", { value: qty("-32.2 °C") })),
];

const sources: OverviewSources = {
  kpi: (key) => slots.find((s) => s.def.key === key),
  mass: { data: budget({}), pending: false },
  power: {
    data: budget({
      kind: "power",
      system: "power",
      total: qty("17.4 W"),
      limit: qty("24.0 W"),
      fill: { ratio: 0.72, basis: "used_over_limit" },
    }),
    pending: false,
  },
};

describe("overviewCards", () => {
  const cards = overviewCards(o, sources);
  const card = (key: string) => cards.find((c) => c.key === key);

  it("値・判定・塗りの比は API のものを並べる（質量はバジェット、電力収支は KPI と電力のバジェット）", () => {
    expect(cards.map((c) => c.key)).toEqual([
      "mass",
      "orbit_balance",
      "max_dod",
      "unload_interval",
      "storage_margin",
      "link_margin",
      "nodes_in_range",
    ]);
    expect(card("mass")).toMatchObject({
      value: "5.16 kg",
      tone: "pos",
      fill: { ratio: 0.47 },
      footer: o.massTotal("5.64 kg"),
      footerRight: o.limit("12.0 kg"),
      to: "/systems/structure",
    });
    expect(card("orbit_balance")).toMatchObject({
      value: "-5.09 W",
      tone: "neg",
      fill: { ratio: 0.72, basis: "used_over_limit" },
      footer: o.consumption("17.4 W"),
      footerRight: o.generationLimit("24.0 W"),
      to: "/systems/power",
    });
    expect(card("max_dod")).toMatchObject({ value: "100 %", fill: { ratio: 1, basis: "value" } });
  });

  it("フッターに別の KPI の値を添え、判定の無い KPI は OK の色、届いていない値は「—」", () => {
    expect(card("unload_interval")).toMatchObject({
      footer: `${o.peakMomentum} 2.32 mN·m·s`,
      fill: null,
      tone: "pos",
      to: "/systems/attitude",
    });
    expect(card("storage_margin")?.footer).toBe(`${o.perDay} —`);
    expect(card("storage_margin")?.tone).toBe("pos");
  });

  it("温度は判定の数え方（API の tally）から文言を選び、塗りは OK の割合", () => {
    expect(card("nodes_in_range")).toMatchObject({
      value: o.nodesToCheck(4),
      fill: { ratio: 5 / 9, basis: "ok_share" },
      tone: "neg",
      footer: `${o.tempMax} 50.5 °C / ${o.tempMin} -32.2 °C`,
      to: "/systems/thermal",
    });
    const allOk = overviewCards(o, {
      ...sources,
      kpi: (key) =>
        key === "nodes_in_range"
          ? slot("thermal", kpi(key, { tally: { checked: 9, flagged: 0 } }))
          : sources.kpi(key),
    });
    expect(allOk.find((c) => c.key === "nodes_in_range")?.value).toBe(o.allInRange);
  });

  it("解析の一覧もバジェットも届く前は読み込み中で、開き先は概要", () => {
    const empty = overviewCards(o, {
      kpi: () => undefined,
      mass: { data: undefined, pending: true },
      power: { data: undefined, pending: true },
    });
    expect(empty.every((c) => c.loading)).toBe(true);
    expect(empty.every((c) => c.to === "/")).toBe(true);
    expect(empty[0]).toMatchObject({ value: "—", tone: "accent", footer: o.massBudget });
  });
});
