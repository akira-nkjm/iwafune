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

it("設計データから入力を作れない解析（projectable でない）は系のページにも話題にも並べない", () => {
  const client = new QueryClient();
  client.setQueryData(keys.analyses, [
    { ...entry("power.a", { kind: "system", value: "power" }), projectable: false },
    entry("power.b", { kind: "system", value: "power" }),
    { ...entry("orbit.c", { kind: "condition", value: "orbit" }), projectable: false },
  ]);
  const html = renderToStaticMarkup(
    <QueryClientProvider client={client}>
      <Selection />
    </QueryClientProvider>,
  );
  expect(html).toBe(renderToStaticMarkup(<span>{JSON.stringify([["power.b"], []])}</span>));
  client.clear();
});

it("系別のモード表示は、解析を回す依頼と回さない依頼でキャッシュを分け、ケースと足もとの範囲を本文で送る", async () => {
  const view = { system: "power", modes: [], missing: [], summary: [] };
  const fetcher = vi
    .fn()
    .mockImplementation(async () => new Response(JSON.stringify(view), { status: 200 }));
  vi.stubGlobal("fetch", fetcher);
  const client = new QueryClient({ defaultOptions: { queries: { retry: false } } });
  const body = (i: number) => JSON.parse(String(fetcher.mock.calls[i]?.[1]?.body));
  try {
    const analyzed = q.systemModes("power", [], { kase: "case A", analyzed: true });
    expect(await client.fetchQuery(analyzed)).toEqual(view);
    expect(await client.fetchQuery(analyzed)).toEqual(view);
    expect(fetcher).toHaveBeenCalledTimes(1);
    expect(fetcher.mock.calls[0]?.[0]).toBe("/api/v1/system_modes/power");
    expect(body(0)).toEqual({ ops: [], case: "case A", analyzed: true, summary: "all" });
    const totals = q.systemModes("power", [], { summary: "totals" });
    expect(totals.queryKey).not.toEqual(q.systemModes("power").queryKey);
    expect(totals.queryKey).not.toEqual(analyzed.queryKey);
    await client.fetchQuery(totals);
    expect(body(1)).toEqual({ ops: [], case: null, analyzed: false, summary: "totals" });
  } finally {
    client.clear();
    vi.unstubAllGlobals();
  }
});

it("部品の一覧と詳細は形態ごとにキャッシュを分け、基準では形態を送らない", async () => {
  const fetcher = vi.fn(async (_path: string) => new Response("[]"));
  vi.stubGlobal("fetch", fetcher);
  const client = new QueryClient();
  try {
    const list = q.parts();
    const configuredList = q.parts("STOWED+1");
    const part = q.part("P");
    const configuredPart = q.part("P", [], "STOWED+1");
    expect(list.queryKey).not.toEqual(configuredList.queryKey);
    expect(part.queryKey).not.toEqual(configuredPart.queryKey);
    await client.fetchQuery(list);
    await client.fetchQuery(configuredList);
    await client.fetchQuery(part);
    await client.fetchQuery(configuredPart);
    expect(fetcher.mock.calls.map(([path]) => path)).toEqual([
      "/api/v1/parts",
      "/api/v1/parts?configuration=STOWED%2B1",
      "/api/v1/parts/P",
      "/api/v1/parts/P?configuration=STOWED%2B1",
    ]);
  } finally {
    client.clear();
    vi.unstubAllGlobals();
  }
});
