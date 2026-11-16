import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import type { ReactNode } from "react";
import { renderToStaticMarkup } from "react-dom/server";
import { MemoryRouter } from "react-router";
import { afterEach, describe, expect, it, vi } from "vitest";
import type { AnalysisOverview, CheckView, KpiView, OverviewChart } from "@/api/types";
import { setLang } from "@/i18n";
import { qv } from "@/test/fixture";
import { isHiddenTable } from "./AnalysisRowDetail";
import {
  AnalysisTable,
  type AnalysisTableItem,
  AnalysisTableRow,
  toggleOpen,
} from "./AnalysisTable";
import { OverviewBars } from "./OverviewBars";

// SSR は日本語で固定されるので、選んだ辞書を使うようにする
vi.mock("@/i18n", async (importOriginal) => {
  const actual = await importOriginal<typeof import("@/i18n")>();
  return { ...actual, useT: actual.getT };
});

const kpi = (key: KpiView["key"], label: string, display: string): KpiView => ({
  key,
  label,
  label_en: `${label} (en)`,
  value: { ...qv(1, "", "Fraction"), display },
  verdict: "ok",
  fill: null,
  tally: null,
});

const check = (at: string, scope: string | null, message: string): CheckView => ({
  key: "dod",
  label: "DoD の上限",
  label_en: null,
  verdict: "ok",
  message,
  message_en: null,
  at,
  scope,
});

const chart: OverviewChart = {
  label: { label: "ケースごとの最大 DoD", label_en: "Maximum DoD per case" },
  bars: [
    {
      key: "NOMINAL",
      label: { label: "定常", label_en: "定常" },
      value: { ...qv(14.1, "%", "Fraction", 0.141), display: "14.1 %" },
      fill: 0.25,
      reference: {
        label: { label: "上限", label_en: "Limit" },
        value: { ...qv(30, "%", "Fraction", 0.3), display: "30.0 %" },
        at: 0.5,
      },
      verdict: "ok",
      note: { label: "上限の 47.0 %", label_en: "47.0 % of limit" },
    },
  ],
};

const kpis = [
  kpi("orbit_balance", "軌道平均の収支（最悪）", "15.8 W"),
  kpi("max_dod", "最大 DoD", "15.1 %"),
  kpi("sunlit_fraction", "日照率", "78.4 %"),
  kpi("mode_load", "四つ目の KPI", "9.99 W"),
];

const overview: AnalysisOverview = {
  main: kpis.slice(0, 3),
  checks: [
    { scope: "撮像", checks: [check("/cases/0/checks/0", "撮像", "最大 DoD 14.1 %")] },
    {
      scope: "モードの並び",
      checks: [check("/sequence/balance/checks/0", "モードの並び", "並び")],
    },
  ],
  chart,
  hidden_tables: [["mode_loads", "modes"]],
};

const item: AnalysisTableItem = {
  entry: {
    info: {
      id: "power.energy_balance",
      title: "電力収支",
      title_en: "Energy balance",
      description: "",
      description_en: "",
      area: { kind: "system", value: "power" },
      kind: "time_series",
    },
    input_type: "",
    output_type: "",
    projectable: true,
    kpis: [],
  },
  result: {
    id: "power.energy_balance",
    case: null,
    case_source: null,
    status: "ok",
    checks: [],
    verdict: "ok",
    kpis,
    input: null,
    output: {
      cases: [{ id: "NOMINAL", max_dod: 0.141 }],
      mode_loads: { modes: [{ id: "NOMINAL", total: 7 }] },
    },
    dims: {},
    labels: {},
    missing: [],
    error: null,
    overview,
  },
};

const missingItem: AnalysisTableItem = {
  entry: { ...item.entry, info: { ...item.entry.info, id: "power.sizing", kind: "steady" } },
  result: {
    ...item.result,
    id: "power.sizing",
    status: "missing_inputs",
    verdict: null,
    kpis: [],
    output: null,
    missing: [
      { part: "BAT", path: "BAT.capacity", target: null, reason: "値がありません" },
      { part: "SAP", path: "SAP.area", target: null, reason: "値がありません" },
    ],
    overview: { main: [], checks: [], chart: null, hidden_tables: [] },
  },
};

function wrap(node: ReactNode) {
  return renderToStaticMarkup(
    <MemoryRouter>
      <QueryClientProvider client={new QueryClient()}>{node}</QueryClientProvider>
    </MemoryRouter>,
  );
}

const row = (open: boolean, it = item) =>
  wrap(
    <table>
      <tbody>
        <AnalysisTableRow item={it} open={open} onToggle={() => {}} />
      </tbody>
    </table>,
  );

afterEach(() => setLang("ja"));

describe("analysis table", () => {
  it("one row per analysis with kind, main results from the API, verdict and missing count", () => {
    const html = wrap(<AnalysisTable items={[item, missingItem]} />);
    expect(html).toContain("電力収支");
    expect(html).toContain("時系列");
    expect(html).toContain("定常");
    // 主な結果は API の `main`（3 つ）だけ。4 つ目は開くまで出さない
    expect(html).toContain("軌道平均の収支（最悪）");
    expect(html).toContain("15.8 W");
    expect(html).toContain("78.4 %");
    expect(html).not.toContain("四つ目の KPI");
    // 足りない入力の数と状態
    expect(html).toContain("2 件");
    expect(html).toContain("入力不足");
    expect(html).toContain("結果なし");
    expect(html.match(/aria-expanded="false"/g)).toHaveLength(2);
    setLang("en");
    const en = wrap(<AnalysisTable items={[item, missingItem]} />);
    expect(en).toContain("Energy balance");
    expect(en).toContain("Main results");
    expect(en).toContain("最大 DoD (en)");
  });

  it("opens and closes in place with a button (aria-expanded)", () => {
    const closed = row(false);
    expect(closed).toContain('aria-expanded="false"');
    expect(closed).not.toContain("ケースごとの最大 DoD");
    expect(closed).not.toContain("データを見る");
    const open = row(true);
    expect(open).toContain('aria-expanded="true"');
    expect(open).toMatch(/aria-controls="([^"]+)".*<tr id="\1"/s);
    // 開くと KPI の全部（4 つ目も）・グラフ・持ち主ごとの判定の文・データを見る
    expect(open).toContain("四つ目の KPI");
    expect(open).toContain("ケースごとの最大 DoD");
    expect(open).toContain("<h4");
    expect(open).toContain("撮像");
    expect(open).toContain("モードの並び");
    expect(open).toContain("データを見る");
    expect(open).not.toContain("<details open");
    expect(toggleOpen([], "a")).toEqual(["a"]);
    expect(toggleOpen(["a", "b"], "a")).toEqual(["b"]);
    expect(wrap(<AnalysisTable items={[item]} initialOpen={[item.result.id]} />)).toContain(
      'aria-expanded="true"',
    );
  });

  it("shows missing inputs when a row with missing inputs is opened", () => {
    const html = row(true, missingItem);
    expect(html).toContain("足りない入力（2）");
    expect(html).toContain("BAT.capacity");
    expect(html).not.toContain("データを見る");
  });

  it("draws bars from the API's fill, reference position and display strings", () => {
    const html = renderToStaticMarkup(<OverviewBars chart={chart} />);
    expect(html).toContain("width:25%");
    expect(html).toContain("left:calc(50% - 1px)");
    expect(html).toContain("14.1 %");
    expect(html).toContain("/ 上限 30.0 %");
    expect(html).toContain("上限の 47.0 %");
    expect(html).toContain("bg-pos");
    const plain = renderToStaticMarkup(
      <OverviewBars
        chart={{
          ...chart,
          bars: chart.bars.map((b) => ({ ...b, verdict: null, reference: null })),
        }}
      />,
    );
    expect(plain).toContain("bg-accent");
    expect(plain).not.toContain("left:calc");
    setLang("en");
    expect(renderToStaticMarkup(<OverviewBars chart={chart} />)).toContain("Maximum DoD per case");
  });

  it("hides tables the API marks as repeated in another row", () => {
    expect(isHiddenTable({ at: ["mode_loads", "modes"] }, [["mode_loads", "modes"]])).toBe(true);
    expect(isHiddenTable({ at: ["cases"] }, [["mode_loads", "modes"]])).toBe(false);
    expect(isHiddenTable({ at: ["mode_loads"] }, [["mode_loads", "modes"]])).toBe(false);
    const open = row(true);
    // 表の見出しは欄の表示名。cases は出し、mode_loads.modes は出さない
    expect(open.match(/<h3 class="mb-2 font-medium text-sm">/g)).toHaveLength(1);
  });
});
