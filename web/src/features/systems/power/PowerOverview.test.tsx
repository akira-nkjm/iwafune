import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { renderToStaticMarkup } from "react-dom/server";
import { MemoryRouter } from "react-router";
import { afterEach, describe, expect, it, vi } from "vitest";
import type {
  AnalysisRunResult,
  PowerBulletView,
  PowerFlowView,
  PowerModeView,
  PowerPageView,
} from "@/api/types";
import { setLang } from "@/i18n";
import { qv } from "@/test/fixture";
import { PowerBullet } from "./PowerBullet";
import { PowerControls } from "./PowerControls";
import { PowerFlow } from "./PowerFlow";
import { PowerOverview } from "./PowerOverview";

// SSR has a fixed Japanese hydration snapshot; emulate the client's selected dictionary here.
vi.mock("@/i18n", async (importOriginal) => {
  const actual = await importOriginal<typeof import("@/i18n")>();
  return { ...actual, useT: actual.getT };
});

const bar: PowerBulletView = {
  key: "dod",
  label: { label: "最大 DoD", label_en: "Maximum DoD" },
  value: qv(20, "%", "Fraction", 0.2),
  limit: qv(40, "%", "Fraction", 0.4),
  verdict: "ok",
  band_start: 0,
  band_end: 80,
  marker: 40,
  limit_marker: 80,
};
const flow: PowerFlowView = {
  height: 240,
  nodes: [
    {
      id: "solar",
      label: { label: "太陽電池", label_en: "Solar Cells" },
      system: "power",
      x: 100,
      y: 120,
      value: qv(60, "W", "Power"),
    },
    {
      id: "comm",
      label: { label: "通信", label_en: "Communication" },
      system: "comm",
      x: 650,
      y: 120,
      value: qv(30, "W", "Power"),
    },
  ],
  edges: [{ x1: 160, y1: 120, x2: 570, y2: 120, width: 11, system: "comm" }],
};
const modes: PowerModeView[] = [
  {
    id: "nominal",
    label: "Nominal",
    verdict: "ok",
    sunlit: flow,
    eclipse: null,
    bars: [bar],
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
};
const page: PowerPageView = {
  modes,
  parts: [],
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
  it("renders API values, subsystem-colored widths and Solar Cells in both languages", () => {
    const ja = renderToStaticMarkup(<PowerFlow flow={flow} />);
    expect(ja).toContain("太陽電池");
    expect(ja).toContain("60 W");
    expect(ja).toContain('stroke-width="11"');
    setLang("en");
    const en = renderToStaticMarkup(<PowerFlow flow={flow} />);
    expect(en).toContain("Solar Cells");
    expect(en).not.toContain("SAP");
  });
  it("uses returned bar positions and keeps unavailable margins explicit", () => {
    const html = renderToStaticMarkup(<PowerBullet item={bar} />);
    expect(html).toContain('x1="40"');
    expect(html).toContain('x2="80"');
    expect(html).toContain("20 %");
    expect(html).toContain("40 %");
    const missing = renderToStaticMarkup(
      <PowerBullet item={{ ...bar, value: null, marker: null, verdict: "warning" }} />,
    );
    expect(missing).toContain("データなし");
    expect(missing).not.toContain('x1="40"');
  });
  it("shows mode verdicts and the same accessible lighting switch as the matrix", () => {
    const html = renderToStaticMarkup(
      <PowerControls
        modes={modes}
        selected="nominal"
        onSelect={() => {}}
        eclipse
        onLighting={() => {}}
      />,
    );
    expect(html).toContain("Nominal");
    expect(html).toContain('aria-pressed="true"');
    expect(html).toContain("bg-pos");
    expect(html).toContain("蝕");
  });
  it("keeps analysis tables in a collapsed data section with an outline", () => {
    const html = overview();
    expect(html).toContain("データを見る");
    expect(html).toContain("このページの目次");
    expect(html).toContain('<details class="mt-3">');
    expect(html).not.toContain("<details open");
    expect(html).toContain("最大 DoD");
    expect(html).toContain('id="power-flow"');
    expect(html).not.toContain("<circle");
    setLang("en");
    expect(overview()).toContain("View data");
  });
});
