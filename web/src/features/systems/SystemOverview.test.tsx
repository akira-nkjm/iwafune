import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { renderToStaticMarkup } from "react-dom/server";
import { MemoryRouter } from "react-router";
import { describe, expect, it } from "vitest";
import type { AnalysisState } from "@/api/queries";
import type { AnalysisEntry, AnalysisRunResult, KpiView, System } from "@/api/types";
import { getT } from "@/i18n";
import { fixture, qv } from "@/test/fixture";
import { type SystemTab, SystemTabs } from "./SystemTabs";

const kpi: KpiView = {
  key: "link_margin",
  label: "リンクマージン",
  label_en: "Link margin",
  value: { ...qv(3.05, "dB", "Decibel"), display: "3.05 dB" },
  verdict: "ok",
  fill: null,
  tally: null,
};

const entry = (id: string, area: System, title: string): AnalysisEntry => ({
  info: {
    id,
    title,
    title_en: title,
    description: "",
    description_en: "",
    area: { kind: "system", value: area },
    kind: "steady",
  },
  input_type: "",
  output_type: "",
  projectable: true,
  kpis: [],
});

const result = (id: string): AnalysisRunResult => ({
  id,
  case: null,
  case_source: null,
  status: "ok",
  checks: [],
  verdict: "ok",
  kpis: [kpi],
  input: null,
  output: { margin: 3.05 },
  dims: {},
  labels: {},
  missing: [],
  error: null,
  overview: { main: [kpi], checks: [], chart: null, hidden_tables: [] },
});

const state = (id: string, over: Partial<AnalysisState> = {}): AnalysisState => ({
  id,
  result: result(id),
  error: null,
  loading: false,
  updating: false,
  ...over,
});

function render(
  system: System,
  states: AnalysisState[],
  entries: AnalysisEntry[],
  tab: SystemTab = "results",
) {
  return renderToStaticMarkup(
    <MemoryRouter>
      <QueryClientProvider client={new QueryClient()}>
        <SystemTabs
          tab={tab}
          kase={null}
          system={system}
          design={fixture}
          states={states}
          entries={new Map(entries.map((e) => [e.info.id, e]))}
        />
      </QueryClientProvider>
    </MemoryRouter>,
  );
}

describe("系の概要（結果のタブ）", () => {
  it("熱と電源以外の系は解析の表（主な結果は API の overview）", () => {
    const html = render(
      "comm",
      [state("comm.link_budget")],
      [entry("comm.link_budget", "comm", "リンクバジェット")],
    );
    const t = getT().systems.analysisTable;
    expect(html).toContain(t.title);
    expect(html).toContain(t.main);
    expect(html).toContain("リンクバジェット");
    expect(html).toContain("3.05 dB");
    expect(html).toContain('aria-expanded="false"');
  });

  it("結果がまだ無い解析は読み込み中のスケルトンの行、問い合わせの失敗はエラーの行", () => {
    const html = render(
      "aocs",
      [
        state("aocs.slew_time", { result: undefined, loading: true }),
        state("aocs.sensor_fov", { result: undefined, error: "接続できません" }),
      ],
      [
        entry("aocs.slew_time", "aocs", "スルー時間"),
        entry("aocs.sensor_fov", "aocs", "センサの視野"),
      ],
    );
    expect(html).toContain("スルー時間");
    expect(html).toContain('aria-busy="true"');
    expect(html).toContain("接続できません");
    expect(html).toContain(getT().systems.status.error);
    // 結果の無い行は開けない
    expect(html).not.toContain("aria-expanded");
  });

  it("熱の概要は表も解析のカードも出さず、カードは解析結果のタブに出す", () => {
    const states = [state("thermal.steady")];
    const entries = [entry("thermal.steady", "thermal", "定常の熱")];
    const overview = render("thermal", states, entries);
    expect(overview).not.toContain(getT().systems.analysisTable.main);
    expect(overview).not.toContain("定常の熱");
    const cards = render("thermal", states, entries, "analyses");
    expect(cards).toContain(getT().systems.status.ok);
    expect(cards).toContain("定常の熱");
  });
});
