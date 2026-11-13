import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { renderToStaticMarkup } from "react-dom/server";
import { describe, expect, it } from "vitest";
import { AnalysisKpis } from "@/analysis/AnalysisKpis";
import { getT } from "@/i18n";
import { fixture } from "@/test/fixture";
import { SystemHeaderActions } from "./SystemHeaderActions";
import { SystemTabs } from "./SystemTabs";

/** 解析がない系でも、結果・設定・時系列の各タブを開ける。 */
describe("解析がない系", () => {
  it("KPI と再計算の操作を表示しない", () => {
    expect(renderToStaticMarkup(<AnalysisKpis entries={[]} states={[]} loading={false} />)).toBe(
      "",
    );
    expect(
      renderToStaticMarkup(
        <SystemHeaderActions
          cases={[]}
          kase={null}
          defaultCaseId={null}
          onCase={() => {}}
          ids={[]}
        />,
      ),
    ).toBe("");
  });

  it("結果・設定・時系列に空の理由を出す", () => {
    const dict = getT().systems;
    for (const [tab, message] of [
      ["results", dict.noAnalyses],
      ["settings", dict.noSettings],
      ["series", dict.noSeries],
    ] as const) {
      const html = renderToStaticMarkup(
        <QueryClientProvider client={new QueryClient()}>
          <SystemTabs
            tab={tab}
            system="payload"
            design={fixture}
            states={[]}
            entries={new Map()}
            noAnalyses
          />
        </QueryClientProvider>,
      );
      expect(html).toContain(message);
    }
  });
});

it("モードの見方を持つ系だけにモードタブの中身を出す", () => {
  for (const system of [
    "power",
    "cdh",
    "thermal",
    "aocs",
    "structure",
    "payload",
    "comm",
  ] as const) {
    const html = renderToStaticMarkup(
      <QueryClientProvider client={new QueryClient()}>
        <SystemTabs tab="modes" system={system} design={fixture} states={[]} entries={new Map()} />
      </QueryClientProvider>,
    );
    if (system === "structure" || system === "payload" || system === "comm") expect(html).toBe("");
    else {
      expect(html).toContain(getT().modes.matrixTitle);
      expect(html).toContain(getT().modes.sunlit);
      expect(html).toContain(getT().modes.eclipseView);
    }
  }
});
