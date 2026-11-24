import {
  MutationObserver,
  QueryClient,
  QueryObserver,
  type QueryObserverOptions,
} from "@tanstack/react-query";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { keys } from "./keys";
import { applyOpsOptions } from "./ops";
import { q } from "./queries";
import type { Op } from "./types";

// 模擬のサーバー: 問い合わせの回数を数える。消した・名前を変えた対象は、確定のあとは 404 になる
const calls: string[] = [];

vi.mock("./client", async (original) => ({
  ...(await original<typeof import("./client")>()),
  api: vi.fn(async (path: string, init?: { body?: unknown }) => {
    const cfg = (init?.body as { configuration?: string } | undefined)?.configuration;
    calls.push(
      `${init?.body && path !== "/ops" ? "POST" : "GET"} ${path}${cfg ? ` [${cfg}]` : ""}`,
    );
    if (path === "/ops") return { cli: ["fune x"], changed: [], diagnostics: [], revision: "r2" };
    if (path === "/design")
      return { project: { revision: "r1" }, defs: { budget_kinds: [] }, parts: [] };
    return {};
  }),
}));

const settle = () => new Promise((resolve) => setTimeout(resolve, 30));
const count = (what: string) => calls.filter((c) => c === what).length;

/** 画面が見ている問い合わせ（購読者がいる）を足す */
function watch<D, K extends readonly unknown[]>(
  qc: QueryClient,
  options: QueryObserverOptions<D, Error, D, D, K>,
) {
  return new QueryObserver(qc, options).subscribe(() => {});
}

async function apply(qc: QueryClient, ops: Op[]) {
  await new MutationObserver(qc, applyOpsOptions(qc)).mutate({ ops, summary: "x" });
}

describe("消した・名前を変えた対象を指す問い合わせ", () => {
  let qc: QueryClient;
  beforeEach(() => {
    calls.length = 0;
    qc = new QueryClient();
  });
  afterEach(() => qc.clear());

  it("形態を消したあと、その形態の部品一覧・解析は読み直さない（基準は読み直す）", async () => {
    const stops = [
      watch(qc, q.design()),
      watch(qc, q.parts("TMPCFG")),
      watch(qc, q.parts(null)),
      watch(qc, q.analysis("structure.mass_properties", null, [], "TMPCFG")),
      watch(qc, q.analysis("structure.mass_properties", null, [], null)),
    ];
    await settle();
    calls.length = 0;
    await apply(qc, [{ op: "remove_configuration", id: "TMPCFG" }]);
    await settle();
    // 消した形態の部品一覧（GET /parts?configuration=…）も解析（POST の configuration）も取り直さない
    expect(calls.filter((c) => c.includes("TMPCFG"))).toEqual([]);
    expect(count("GET /parts")).toBe(1); // 基準の一覧は読み直す
    expect(count("POST /analyses/structure.mass_properties")).toBe(1); // 基準の結果は読み直す
    expect(count("GET /design")).toBe(1);
    for (const stop of stops) stop();
  });

  it("部品の名前を変えたあと、詳細を開いたままでも旧 ID の GET /parts/<ID> を取り直さない", async () => {
    const stop = watch(qc, q.part("BUS_FRAME"));
    await settle();
    expect(count("GET /parts/BUS_FRAME")).toBe(1);
    await apply(qc, [{ op: "rename_part", id: "BUS_FRAME", to: "BUS_FRAME2" }]);
    await settle();
    expect(count("GET /parts/BUS_FRAME")).toBe(1);
    stop();
  });

  it("別の部品の詳細・消さない部品の改名は読み直す", async () => {
    const stop = watch(qc, q.part("OTHER"));
    await settle();
    await apply(qc, [{ op: "rename_part", id: "BUS_FRAME", to: "BUS_FRAME2" }]);
    await settle();
    expect(count("GET /parts/OTHER")).toBe(2);
    stop();
  });

  it("解析ケースを消したあと、そのケースの解析は読み直さない", async () => {
    const stop = watch(qc, q.analysis("power.mode_load", "hot", []));
    const stopOther = watch(qc, q.analysis("power.mode_load", "cold", []));
    await settle();
    calls.length = 0;
    await apply(qc, [{ op: "remove_case", id: "hot" }]);
    await settle();
    expect(count("POST /analyses/power.mode_load")).toBe(1); // cold だけ
    stop();
    stopOther();
  });

  it("見ていない（購読者のいない）古いキーの問い合わせはキャッシュから外す", async () => {
    await qc.fetchQuery(q.parts("TMPCFG"));
    await qc.fetchQuery(q.parts(null));
    await apply(qc, [{ op: "remove_configuration", id: "TMPCFG" }]);
    expect(qc.getQueryData(keys.parts("TMPCFG"))).toBeUndefined();
    expect(qc.getQueryData(keys.parts(null))).toBeDefined();
  });
});
