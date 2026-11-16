import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { renderToStaticMarkup } from "react-dom/server";
import { expect, it, vi } from "vitest";
import { keys } from "./keys";
import { q, useConditionAnalyses, useSystemAnalyses } from "./queries";
import type { AnalysisArea, AnalysisEntry } from "./types";

const entry = (id: string, area: AnalysisArea): AnalysisEntry => ({
  info: { id, area, title: id, title_en: id, kind: "steady", description: "", description_en: "" },
  input_type: "",
  output_type: "",
  projectable: true,
  kpis: [],
});

function Selection() {
  const system = useSystemAnalyses("power");
  const topic = useConditionAnalyses("orbit");
  return <span>{JSON.stringify([system.ids, topic.ids])}</span>;
}

it("系と話題の解析を area で分け、ID の接頭辞では選ばない", () => {
  const client = new QueryClient();
  client.setQueryData(keys.analyses, [
    entry("orbit.system_example", { kind: "system", value: "power" }),
    entry("power.condition_example", { kind: "condition", value: "orbit" }),
    entry("comm.visibility", { kind: "system", value: "comm" }),
  ]);
  const html = renderToStaticMarkup(
    <QueryClientProvider client={client}>
      <Selection />
    </QueryClientProvider>,
  );
  expect(html).toBe(
    renderToStaticMarkup(
      <span>{JSON.stringify([["orbit.system_example"], ["power.condition_example"]])}</span>,
    ),
  );
  client.clear();
});

it("電源の全タブ用に単一のケース別 API とキャッシュを使う", async () => {
  const page = { modes: [], analyses: [], parts: [] };
  const fetcher = vi
    .fn()
    .mockImplementation(async () => new Response(JSON.stringify(page), { status: 200 }));
  vi.stubGlobal("fetch", fetcher);
  const client = new QueryClient({ defaultOptions: { queries: { retry: false } } });
  try {
    expect(await client.fetchQuery(q.powerPage("case A"))).toEqual(page);
    expect(await client.fetchQuery(q.powerPage("case A"))).toEqual(page);
    expect(fetcher).toHaveBeenCalledTimes(1);
    expect(fetcher.mock.calls[0]?.[0]).toBe("/api/v1/power_page?case=case%20A");
    expect(q.powerPage(null).queryKey).not.toEqual(q.powerPage("case A").queryKey);
    expect(await client.fetchQuery(q.powerPage(null))).toEqual(page);
    expect(fetcher.mock.calls[1]?.[0]).toBe("/api/v1/power_page");
  } finally {
    client.clear();
    vi.unstubAllGlobals();
  }
});
