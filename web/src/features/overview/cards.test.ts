import { describe, expect, it } from "vitest";
import type { KpiSlot } from "@/analysis/kpi";
import type { BudgetView, KpiKey, KpiView, OverviewKpi, QuantityView, System } from "@/api/types";
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

/** 概要の見出しにする KPI（API の `KpiDef.overview`）。フッターに添える KPI は `footer` に */
const HEADS: Partial<Record<KpiKey, OverviewKpi>> = {
  orbit_balance: { order: 0, footer: [], budget: "power" },
  max_dod: { order: 1, footer: [], budget: null },
  unload_interval: { order: 2, footer: ["peak_momentum"], budget: null },
  storage_margin: { order: 3, footer: ["data_per_day"], budget: null },
  link_margin: { order: 4, footer: [], budget: null },
  nodes_in_range: { order: 5, footer: ["max_temperature", "min_temperature"], budget: null },
  dv_with_margin: { order: 6, footer: ["loaded_propellant"], budget: null },
};

const slot = (system: System, k: KpiView, loading = false): KpiSlot => ({
  def: {
    key: k.key,
    label: k.label,
    label_en: k.label_en,
    when_measured: false,
    overview: HEADS[k.key] ?? null,
  },
  analysis: `${system}.x`,
  area: { kind: "system", value: system },
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
  limit_requirement: "R_MASS",
  system_margin: qty("10.0 %"),
  total: qty("5.64 kg"),
  margin: qty("5.16 kg"),
  verdict: "ok",
  fill: { ratio: 0.47, basis: "used_over_limit" },
  rows: [],
  modes: [],
  propellant: null,
  analysis: "budget",
  mode: null,
  missing: [],
  uncounted: [],
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
  slot("aocs", kpi("unload_interval", { value: qty("16.4 h"), verdict: "ok" })),
  slot("aocs", kpi("peak_momentum", { value: qty("2.32 mN·m·s") })),
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

const budgets = {
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

const sources: OverviewSources = {
  slots,
  budget: (kind) => budgets[kind as "mass" | "power"] ?? { data: undefined, pending: false },
};

describe("overviewCards", () => {
  const cards = overviewCards(o, sources);
  const card = (key: string) => cards.find((c) => c.key === key);

  it("値・判定・塗りの比は API のものを、解析の一覧の順に並べる（質量はバジェット、電力収支は KPI と電力のバジェット）", () => {
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

  it("並びは API の `order`（解析の一覧の順ではない）", () => {
    const reversed = overviewCards(o, { ...sources, slots: [...slots].reverse() });
    expect(reversed.map((c) => c.key)).toEqual(cards.map((c) => c.key));
  });

  it("見出しは API の KPI の名前、フッターには同じ解析の別の KPI を「名前 値」で添える（届いていない値は「—」）", () => {
    expect(card("unload_interval")).toMatchObject({
      label: "unload_interval",
      footer: "peak_momentum 2.32 mN·m·s",
      fill: null,
      tone: "pos",
      to: "/systems/aocs",
    });
    expect(card("storage_margin")?.footer).toBe("data_per_day —");
    expect(card("storage_margin")?.tone).toBe("pos");
  });

  it("温度は判定の数え方（API の tally）から文言を選び、塗りは OK の割合", () => {
    expect(card("nodes_in_range")).toMatchObject({
      value: o.nodesToCheck(4),
      fill: { ratio: 5 / 9, basis: "ok_share" },
      tone: "neg",
      footer: "max_temperature 50.5 °C / min_temperature -32.2 °C",
      to: "/systems/thermal",
    });
    const allOk = overviewCards(o, {
      ...sources,
      slots: slots.map((s) =>
        s.def.key === "nodes_in_range"
          ? slot("thermal", kpi("nodes_in_range", { tally: { checked: 9, flagged: 0 } }))
          : s,
      ),
    });
    expect(allOk.find((c) => c.key === "nodes_in_range")?.value).toBe(o.allInRange);
  });

  it("推進の KPI は API が見出しにしていれば並ぶ（キーを書き足さなくてよい）", () => {
    const withPropulsion = overviewCards(o, {
      ...sources,
      slots: [
        ...slots,
        slot("propulsion", kpi("dv_with_margin", { value: qty("212 m/s") })),
        slot("propulsion", kpi("loaded_propellant", { value: qty("67.2 g") })),
      ],
    });
    expect(withPropulsion.find((c) => c.key === "dv_with_margin")).toMatchObject({
      value: "212 m/s",
      footer: "loaded_propellant 67.2 g",
      to: "/systems/propulsion",
    });
    // フッターだけの KPI（見出しでない）はカードにならない
    expect(withPropulsion.some((c) => c.key === "loaded_propellant")).toBe(false);
  });

  it("解析が走り終えて見せるものが無い KPI（電池・ホイールの無い設計）のカードは出さない", () => {
    const bare = overviewCards(o, {
      ...sources,
      slots: slots.map((s) =>
        ["orbit_balance", "max_dod", "unload_interval"].includes(s.def.key)
          ? { ...s, kpi: kpi(s.def.key, {}) }
          : s,
      ),
    });
    expect(bare.map((c) => c.key)).toEqual([
      "mass",
      "storage_margin",
      "link_margin",
      "nodes_in_range",
    ]);
    // 結果を待っている間は出す（読み込み中のカード）
    const waiting = overviewCards(o, {
      ...sources,
      slots: slots.map((s) => (s.def.key === "max_dod" ? { ...s, kpi: null, loading: true } : s)),
    });
    expect(waiting.find((c) => c.key === "max_dod")?.loading).toBe(true);
  });

  it("推薬を数えた質量は推薬込みの余裕・判定・塗りを出し、フッターに乾燥と推薬込みの両方", () => {
    const wet = overviewCards(o, {
      ...sources,
      budget: (kind) =>
        kind === "mass"
          ? {
              data: budget({
                propellant: {
                  status: "counted",
                  analysis: "propulsion.propellant",
                  loaded: qty("67.2 g"),
                  wet: {
                    total: qty("5.71 kg"),
                    margin: qty("5.09 kg"),
                    verdict: "warning",
                    fill: { ratio: 0.48, basis: "used_over_limit" },
                  },
                },
              }),
              pending: false,
            }
          : sources.budget(kind),
    });
    expect(wet[0]).toMatchObject({
      value: "5.09 kg",
      tone: "warn",
      fill: { ratio: 0.48 },
      footer: o.massDryWet("5.64 kg", "5.71 kg"),
    });
    const none = overviewCards(o, {
      ...sources,
      budget: (kind) =>
        kind === "mass"
          ? { data: budget({ propellant: { status: "no_propulsion" } }), pending: false }
          : sources.budget(kind),
    });
    expect(none[0]).toMatchObject({ value: "5.16 kg", footer: o.massTotal("5.64 kg") });
  });

  it("バジェットの届く前は質量のカードが読み込み中で、開き先は概要。バジェットの無い質量のカードは出さない", () => {
    const empty = overviewCards(o, {
      slots: [],
      budget: () => ({ data: undefined, pending: true }),
    });
    expect(empty).toHaveLength(1);
    expect(empty[0]).toMatchObject({ loading: true, to: "/", value: "—", tone: "accent" });
    expect(empty[0]?.footer).toBe(o.massBudget);
    expect(
      overviewCards(o, { slots: [], budget: () => ({ data: undefined, pending: false }) }),
    ).toEqual([]);
  });
});
