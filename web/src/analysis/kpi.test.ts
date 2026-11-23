import { describe, expect, it } from "vitest";
import type { AnalysisState } from "@/api/queries";
import type { AnalysisEntry, AnalysisRunResult, KpiKey, KpiView, System } from "@/api/types";
import { analysesFor, hasContent, kpiSlots, kpiText, overviewKeys } from "./kpi";

const def = (key: KpiKey, when_measured = false) => ({
  key,
  label: `名前 ${key}`,
  label_en: `Name ${key}`,
  when_measured,
  overview: null,
});

const entry = (id: string, system: System, keys: KpiKey[]): AnalysisEntry => ({
  info: {
    id,
    title: id,
    title_en: id,
    area: { kind: "system", value: system },
    kind: "steady",
    description: "",
    description_en: "",
  },
  input_type: "",
  output_type: "",
  projectable: true,
  kpis: keys.map((key) => def(key)),
});

const view = (key: KpiKey, display: string | null): KpiView => ({
  ...def(key),
  value: display === null ? null : { value: 1, unit: "W", si: 1, dimension: "Power", display },
  verdict: null,
  fill: null,
  tally: null,
});

const result = (id: string, kpis: KpiView[]): AnalysisRunResult => ({
  id,
  case: null,
  case_source: null,
  status: "ok",
  checks: [],
  verdict: null,
  kpis,
  input: null,
  output: {},
  dims: {},
  labels: {},
  missing: [],
  error: null,
  overview: { main: [], checks: [], chart: null, hidden_tables: [] },
});

const state = (id: string, kpis: KpiView[] | null): AnalysisState => ({
  id,
  result: kpis === null ? undefined : result(id, kpis),
  error: null,
  loading: kpis === null,
  updating: false,
});

const entries = [
  entry("power.mode_load", "power", ["mode_load"]),
  entry("power.solar_sunlit", "power", []),
  entry("power.energy_balance", "power", ["orbit_balance", "max_dod"]),
];

describe("kpiSlots", () => {
  it("前提条件の KPI は話題の区分を保つ", () => {
    const topic = entry("orbit.example", "power", ["mode_load"]);
    topic.info.area = { kind: "condition", value: "orbit" };
    const [slot] = kpiSlots([topic], []);
    expect(slot?.area).toEqual({ kind: "condition", value: "orbit" });
  });
  it("解析の一覧の順に KPI を並べ、値は結果の kpis をキーで引く", () => {
    const slots = kpiSlots(entries, [
      state("power.mode_load", [view("mode_load", "17.4 W")]),
      state("power.energy_balance", null),
    ]);
    expect(slots.map((s) => s.def.key)).toEqual(["mode_load", "orbit_balance", "max_dod"]);
    expect(slots[0]).toMatchObject({
      analysis: "power.mode_load",
      area: { kind: "system", value: "power" },
      loading: false,
    });
    expect(slots[0]?.kpi?.value?.display).toBe("17.4 W");
    // 結果を待っている解析の KPI は、値が無く読み込み中
    expect(slots[1]).toMatchObject({ kpi: null, loading: true });
  });

  it("測れたときだけ出す KPI は、結果が届いて値が無ければ並べない", () => {
    const e = entry("aocs.unload_time", "aocs", []);
    e.kpis = [def("mtq_unload_time", true), def("rcs_unload_time", true)];
    const keys = (s: AnalysisState | null) =>
      kpiSlots([e], s ? [s] : []).map((slot) => slot.def.key);
    expect(keys(state("aocs.unload_time", [view("mtq_unload_time", "25.4 min")]))).toEqual([
      "mtq_unload_time",
    ]);
    expect(keys(state("aocs.unload_time", null))).toEqual(["mtq_unload_time", "rcs_unload_time"]);
    expect(keys(null)).toEqual(["mtq_unload_time", "rcs_unload_time"]);
  });

  it("結果に無いキーと、まだ状態の無い解析は値を持たない", () => {
    const slots = kpiSlots(entries, [state("power.energy_balance", [view("max_dod", "100 %")])]);
    expect(slots.find((s) => s.def.key === "orbit_balance")?.kpi).toBeNull();
    expect(slots.find((s) => s.def.key === "max_dod")?.kpi?.value?.display).toBe("100 %");
    expect(slots.find((s) => s.def.key === "mode_load")).toMatchObject({
      kpi: null,
      loading: true,
    });
  });
});

describe("analysesFor", () => {
  it("キーを出す解析だけを、一覧の順に重ねずに選ぶ", () => {
    expect(analysesFor(entries, ["max_dod", "orbit_balance", "mode_load"])).toEqual([
      "power.mode_load",
      "power.energy_balance",
    ]);
    expect(analysesFor(entries, ["link_margin"])).toEqual([]);
  });
});

describe("overviewKeys", () => {
  it("API が概要の見出しにした KPI と、そのフッターの KPI のキーを、一覧の順に返す", () => {
    const e = entry("power.energy_balance", "power", ["orbit_balance", "max_dod"]);
    e.kpis = [
      { ...def("orbit_balance"), overview: { order: 0, footer: ["max_dod"], budget: null } },
      { ...def("max_dod"), overview: null },
    ];
    expect(overviewKeys([e, ...entries])).toEqual(["orbit_balance", "max_dod"]);
    expect(overviewKeys(entries)).toEqual([]);
  });
});

describe("hasContent", () => {
  it("値・判定・数えた判定のどれかがあれば true、何も無ければ false", () => {
    expect(hasContent(null)).toBe(false);
    expect(hasContent(view("max_dod", null))).toBe(false);
    expect(hasContent(view("max_dod", "1 W"))).toBe(true);
    expect(hasContent({ ...view("max_dod", null), verdict: "ok" })).toBe(true);
    expect(hasContent({ ...view("nodes_in_range", null), tally: { checked: 0, flagged: 0 } })).toBe(
      false,
    );
    expect(hasContent({ ...view("nodes_in_range", null), tally: { checked: 3, flagged: 0 } })).toBe(
      true,
    );
  });
});

describe("kpiText", () => {
  const tally = (t: { checked: number; flagged: number }) => `${t.flagged}/${t.checked}`;
  it("値は API の整形、数える KPI は呼び手の文、無ければ「—」", () => {
    expect(kpiText(view("max_dod", "100 %"), tally)).toBe("100 %");
    expect(
      kpiText({ ...view("nodes_in_range", null), tally: { checked: 9, flagged: 4 } }, tally),
    ).toBe("4/9");
    expect(kpiText(view("max_dod", null), tally)).toBe("—");
    expect(kpiText(null, tally)).toBe("—");
  });
});
