import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import type { ReactNode } from "react";
import { renderToStaticMarkup } from "react-dom/server";
import { expect, it } from "vitest";
import type { AnalysisRunResult } from "@/api/types";
import { getT } from "@/i18n";
import { AnalysisCard } from "./AnalysisCard";

const result: AnalysisRunResult = {
  id: "orbit.eclipse",
  case: null,
  case_source: null,
  status: "ok",
  checks: [],
  verdict: null,
  kpis: [],
  input: null,
  output: { eclipses: [{ start: 0, end: 2100 }], period: 5700 },
  dims: {},
  labels: {},
  missing: [],
  error: null,
  overview: { main: [], checks: [], chart: null, hidden_tables: [] },
};

const render = (node: ReactNode) =>
  renderToStaticMarkup(
    <QueryClientProvider client={new QueryClient()}>{node}</QueryClientProvider>,
  );

it("解析の出力の表は、結果のカードの畳んだ「データ」に出す（表が無ければ出さない）", () => {
  const html = render(
    <AnalysisCard
      state={{ id: result.id, result, error: null, loading: false, updating: false }}
      entry={undefined}
      design={undefined}
    />,
  );
  expect(html).toContain("<details");
  expect(html).toContain(getT().systems.data);
  const none = render(
    <AnalysisCard
      state={{
        id: result.id,
        result: { ...result, output: { period: 5700 } },
        error: null,
        loading: false,
        updating: false,
      }}
      entry={undefined}
      design={undefined}
    />,
  );
  expect(none).not.toContain("<details");
});
