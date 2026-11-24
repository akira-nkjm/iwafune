import { describe, expect, it, vi } from "vitest";
import { diagramDemo } from "@/test/diagram-model";
import type { DiagramModel } from "../model";
import { planLayout } from "./layout";
import { PlanCache, type PlanStorage, type StoredPlan } from "./plan-cache";
import { createPlanService, type PlanWorker, SupersededError } from "./plan-service";
import { handlePlanRequest, type PlanReply, type PlanRequest } from "./plan-worker-core";

/** 作り物のワーカー: 依頼は溜めておき、テストが好きな時に返事をする */
class FakeWorker implements PlanWorker {
  onmessage: ((e: { data: PlanReply }) => void) | null = null;
  onerror: ((e: { message?: string }) => void) | null = null;
  requests: PlanRequest[] = [];
  terminated = false;
  postMessage(req: PlanRequest) {
    this.requests.push(req);
  }
  terminate() {
    this.terminated = true;
  }
  reply(r: PlanReply) {
    this.onmessage?.({ data: r });
  }
  /** 本物の計算で返事をする */
  answer() {
    for (const r of this.requests) this.reply(handlePlanRequest(r));
  }
}

const fakePlan = (tag: string) => ({
  tree: {
    roots: [tag],
    children: new Map(),
    parent: new Map(),
    assemblies: new Set<string>(),
    frames: new Set<string>(),
  },
  geometry: {
    boxes: new Map(),
    absolute: new Map(),
    ports: new Map(),
    taps: new Map(),
    routes: new Map(),
    labels: new Map(),
  },
});

const withLabel = (label: string): DiagramModel => ({
  ...diagramDemo,
  cards: diagramDemo.cards.map((c, i) => (i === 0 ? { ...c, label } : c)),
});

function setup(cache?: PlanCache) {
  const workers: FakeWorker[] = [];
  const service = createPlanService({
    spawn: () => {
      const w = new FakeWorker();
      workers.push(w);
      return w;
    },
    cache,
  });
  return { workers, service };
}
/** n 個目のワーカーが始まるまで待つ（鍵の計算は非同期） */
const started = (workers: FakeWorker[], n: number) =>
  vi.waitFor(() => expect(workers.length).toBeGreaterThanOrEqual(n));

describe("配置の計算の窓口", () => {
  it("ワーカーの計算の結果は、主スレッドで planLayout を回した結果と同じ", async () => {
    const { workers, service } = setup();
    const done = service.plan(diagramDemo);
    await started(workers, 1);
    workers[0]?.answer();
    expect(await done).toEqual(planLayout(diagramDemo));
  }, 60_000);

  it("同じ入力は 2 度目から計算し直さない（メモリ）", async () => {
    const { workers, service } = setup();
    const first = service.plan(diagramDemo);
    await started(workers, 1);
    workers[0]?.reply({ id: 1, plan: fakePlan("a") });
    const plan = await first;
    expect(await service.plan(structuredClone(diagramDemo))).toBe(plan);
    expect(workers).toHaveLength(1);
    expect(workers[0]?.terminated).toBe(true);
  });

  it("別の入力は計算し直す", async () => {
    const { workers, service } = setup();
    const a = service.plan(diagramDemo);
    await started(workers, 1);
    workers[0]?.reply({ id: 1, plan: fakePlan("a") });
    await a;
    const b = service.plan(withLabel("changed"));
    await started(workers, 2);
    expect(workers).toHaveLength(2);
    workers[1]?.reply({ id: 2, plan: fakePlan("b") });
    expect((await b).tree.roots).toEqual(["b"]);
  });

  it("ブラウザの保存領域にある結果は、計算せずに返す（再読み込みのあと）", async () => {
    const rows = new Map<string, StoredPlan>();
    const storage: PlanStorage = {
      get: async (k) => rows.get(k),
      put: async (e) => void rows.set(e.key, e),
      delete: async (k) => void rows.delete(k),
      list: async () => [...rows.values()].map((e) => ({ key: e.key, usedAt: e.usedAt })),
    };
    const first = setup(new PlanCache(storage));
    const p = first.service.plan(diagramDemo);
    await started(first.workers, 1);
    first.workers[0]?.reply({ id: 1, plan: fakePlan("saved") });
    await p;
    await vi.waitFor(() => expect(rows.size).toBe(1));
    // ページを開き直した想定: メモリは空、保存領域だけ残っている
    const second = setup(new PlanCache(storage));
    expect((await second.service.plan(diagramDemo)).tree.roots).toEqual(["saved"]);
    expect(second.workers).toHaveLength(0);
  });

  it("後から別の依頼が来たら、前の依頼の計算は止めて結果を捨てる", async () => {
    const { workers, service } = setup();
    const old = service.plan(diagramDemo);
    const oldResult = old.catch((e: unknown) => e);
    await started(workers, 1);
    const next = service.plan(withLabel("newer"));
    await started(workers, 2);
    expect(await oldResult).toBeInstanceOf(SupersededError);
    expect(workers[0]?.terminated).toBe(true);
    // 止めたあとに遅れて届いた古い返事は、新しい結果に混ざらない
    workers[0]?.reply({ id: 1, plan: fakePlan("old") });
    workers[1]?.reply({ id: 2, plan: fakePlan("new") });
    expect((await next).tree.roots).toEqual(["new"]);
  });

  it("鍵を計算している間に次の依頼が来た前の依頼は、ワーカーを始めない", async () => {
    const { workers, service } = setup();
    const old = service.plan(diagramDemo).catch((e: unknown) => e);
    const next = service.plan(withLabel("newer"));
    await started(workers, 1);
    expect(await old).toBeInstanceOf(SupersededError);
    workers[0]?.reply({ id: 2, plan: fakePlan("new") });
    await next;
    expect(workers).toHaveLength(1);
  });

  it("同じ入力の依頼が重なったら、計算は 1 回", async () => {
    const { workers, service } = setup();
    const a = service.plan(diagramDemo).catch((e: unknown) => e);
    await started(workers, 1);
    const b = service.plan(diagramDemo);
    workers[0]?.reply({ id: 1, plan: fakePlan("x") });
    expect((await b).tree.roots).toEqual(["x"]);
    expect(await a).toBeInstanceOf(SupersededError);
    expect(workers).toHaveLength(1);
  });

  it("計算の失敗は例外として返り（画面の失敗の表示に渡る）、結果は残さない", async () => {
    const { workers, service } = setup();
    const failing = service.plan(diagramDemo);
    const caught = failing.catch((e: unknown) => e);
    await started(workers, 1);
    workers[0]?.reply({ id: 1, error: "boom" });
    const error = await caught;
    expect(error).toBeInstanceOf(Error);
    expect(error).not.toBeInstanceOf(SupersededError);
    expect((error as Error).message).toBe("boom");
    expect(workers[0]?.terminated).toBe(true);
    // もう一度頼むと、計算し直す
    const again = service.plan(diagramDemo);
    await started(workers, 2);
    workers[1]?.reply({ id: 2, plan: fakePlan("ok") });
    expect((await again).tree.roots).toEqual(["ok"]);
  });

  it("ワーカー自体の失敗（読み込めないなど）も例外になる", async () => {
    const { workers, service } = setup();
    const caught = service.plan(diagramDemo).catch((e: unknown) => e);
    await started(workers, 1);
    workers[0]?.onerror?.({ message: "worker crashed" });
    expect(((await caught) as Error).message).toBe("worker crashed");
  });

  it("handlePlanRequest は計算の例外を返事の失敗にする", () => {
    const broken = { ...diagramDemo, cards: null } as unknown as DiagramModel;
    const r = handlePlanRequest({ id: 7, full: broken });
    expect(r.id).toBe(7);
    expect("error" in r).toBe(true);
  });
});
