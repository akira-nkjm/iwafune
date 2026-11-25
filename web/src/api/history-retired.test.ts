import {
  MutationObserver,
  QueryClient,
  QueryObserver,
  type QueryObserverOptions,
} from "@tanstack/react-query";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { historyStepOptions } from "./history";
import { keys, retiredBetween } from "./keys";
import { q } from "./queries";
import type { DesignView } from "./types";

// 模擬のサーバー: 今の設計にある部品だけが読める。無い部品は 404
const calls: string[] = [];
let partIds: string[] = [];

const design = (): Partial<DesignView> => ({
  project: { revision: "r" } as DesignView["project"],
  defs: { budget_kinds: [] } as unknown as DesignView["defs"],
  parts: partIds.map((id) => ({ id })) as DesignView["parts"],
  configurations: [],
  cases: [],
});

vi.mock("./client", async (original) => ({
  ...(await original<typeof import("./client")>()),
  api: vi.fn(async (path: string) => {
    calls.push(path);
    if (path === "/design") return design();
    if (path === "/undo" || path === "/redo")
      return {
        action: path.slice(1),
        revision: "r",
        previous_revision: "p",
        changed: [],
        edit_cli: [],
      };
    const id = path.startsWith("/parts/") ? path.slice(7) : null;
    if (id && !partIds.includes(id)) throw new Error("404");
    return {};
  }),
}));

vi.mock("sonner", () => ({ toast: { success: vi.fn(), error: vi.fn(), message: vi.fn() } }));
vi.mock("@/lib/history", () => ({ recordChange: vi.fn(), copyText: vi.fn() }));

const settle = () => new Promise((resolve) => setTimeout(resolve, 30));
const count = (what: string) => calls.filter((c) => c === what).length;

function watch<D, K extends readonly unknown[]>(
  qc: QueryClient,
  options: QueryObserverOptions<D, Error, D, D, K>,
) {
  return new QueryObserver(qc, options).subscribe(() => {});
}

/** 取り消し・やり直しの mutation を hook を通さずに走らせる */
async function step(qc: QueryClient, action: "undo" | "redo", after: string[]) {
  partIds = after;
  await new MutationObserver(qc, historyStepOptions(qc)).mutate(action);
}

describe("取り消し・やり直しで対象が消える・戻る", () => {
  let qc: QueryClient;
  beforeEach(() => {
    calls.length = 0;
    partIds = ["A", "B"];
    qc = new QueryClient();
  });
  afterEach(() => qc.clear());

  it("前後の設計を比べて、消えた部品・形態・ケースを求める", () => {
    const d = (p: string[], c: string[], k: string[]) =>
      ({
        parts: p.map((id) => ({ id })),
        configurations: c.map((id) => ({ id })),
        cases: k.map((id) => ({ id })),
      }) as unknown as DesignView;
    const r = retiredBetween(d(["A", "B"], ["C"], ["K"]), d(["A"], [], ["K", "L"]));
    expect([...r.parts]).toEqual(["B"]);
    expect([...r.configurations]).toEqual(["C"]);
    expect([...r.cases]).toEqual([]);
  });

  it("取り消しで部品が消えても、その部品の問い合わせを取りに行かない", async () => {
    const stops = [watch(qc, q.design()), watch(qc, q.part("A")), watch(qc, q.part("B"))];
    await settle();
    calls.length = 0;
    await step(qc, "undo", ["A"]);
    await settle();
    expect(count("/parts/B")).toBe(0);
    expect(count("/parts/A")).toBe(1);
    for (const s of stops) s();
  });

  it("やり直しで部品が戻れば、その部品の問い合わせは読み直す", async () => {
    partIds = ["A"];
    const stops = [watch(qc, q.design()), watch(qc, q.part("A"))];
    await settle();
    calls.length = 0;
    await step(qc, "redo", ["A", "B"]);
    stops.push(watch(qc, q.part("B")));
    await settle();
    expect(count("/parts/B")).toBe(1);
    expect(qc.getQueryState(keys.part("B"))?.status).toBe("success");
    for (const s of stops) s();
  });
});
