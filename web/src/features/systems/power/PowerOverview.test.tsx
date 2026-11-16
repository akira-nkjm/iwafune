import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { renderToStaticMarkup } from "react-dom/server";
import { MemoryRouter } from "react-router";
import { afterEach, describe, expect, it, vi } from "vitest";
import type { AnalysisRunResult, PowerBudgetView, PowerModeView, PowerPageView } from "@/api/types";
import { setLang } from "@/i18n";
import { qv } from "@/test/fixture";
import { PowerModes } from "./PowerModes";
import { PowerOverview } from "./PowerOverview";

// SSR has a fixed Japanese hydration snapshot; emulate the client's selected dictionary here.
vi.mock("@/i18n", async (importOriginal) => {
  const actual = await importOriginal<typeof import("@/i18n")>();
  return { ...actual, useT: actual.getT };
});

const budget: PowerBudgetView = {
  supply: qv(60, "W", "Power"),
  order: 30,
  segments: [
    {
      kind: "load",
      system: "comm",
      label: { label: "通信", label_en: "Communication" },
      value: qv(30, "W", "Power"),
      fraction: 0.5,
    },
    {
      kind: "shunt",
      system: null,
      label: { label: "余り（シャント）", label_en: "Surplus (shunt)" },
      value: qv(30, "W", "Power"),
      fraction: 0.5,
    },
  ],
};
const modes: PowerModeView[] = [
  {
    id: "nominal",
    label: "Nominal",
    verdict: "ok",
    sunlit: budget,
    eclipse: null,
    charts: [],
  },
];
const result: AnalysisRunResult = {
  id: "power.energy_balance",
  case: null,
  case_source: null,
  status: "ok",
  checks: [],
  verdict: "ok",
  kpis: [],
  input: null,
  output: { rows: [{ id: "nominal", load: 30 }] },
  dims: {},
  labels: {},
  missing: [],
  error: null,
  overview: { main: [], checks: [], chart: null, hidden_tables: [] },
};
const page: PowerPageView = {
  modes,
  flow: { nodes: [], edges: [], folds: [] },
  analyses: [
    {
      result,
      entry: {
        info: {
          id: result.id,
          title: "収支",
          title_en: "Balance",
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
    },
  ],
};

function overview() {
  return renderToStaticMarkup(
    <MemoryRouter>
      <QueryClientProvider client={new QueryClient()}>
        <PowerOverview page={page} />
      </QueryClientProvider>
    </MemoryRouter>,
  );
}

afterEach(() => setLang("ja"));

describe("power overview", () => {
  it("draws one bar per mode with API widths and labels in both languages", () => {
    const render = () => renderToStaticMarkup(<PowerModes page={page} />);
    const ja = render();
    expect(ja).toContain("width:50%");
    expect(ja).toContain("60 W");
    expect(ja).toContain("余り（シャント）");
    expect(ja).toContain("余裕の少ない順");
    setLang("en");
    const en = render();
    expect(en).toContain("Surplus (shunt)");
    expect(en).not.toContain("SAP");
  });
  it("shows only the table of analyses with collapsed rows", () => {
    const html = overview();
    expect(html).not.toContain("このページの目次");
    expect(html).not.toContain("最大 DoD");
    expect(html).toContain("主な結果");
    expect(html).toContain("収支");
    expect(html).toContain('aria-expanded="false"');
    expect(html).not.toContain("データを見る");
    expect(html).not.toContain("<circle");
    setLang("en");
    expect(overview()).toContain("Analyses");
  });
});
