import type { DiagramModel } from "../model";
import type { LayoutPlan } from "./layout";
import type { PlanCache } from "./plan-cache";
import { planKey } from "./plan-key";
import type { PlanReply, PlanRequest } from "./plan-worker-core";

/** 計算を引き受けるもの（Web Worker と同じ口。テストでは作り物に替える） */
export interface PlanWorker {
  onmessage: ((e: { data: PlanReply }) => void) | null;
  onerror: ((e: { message?: string }) => void) | null;
  postMessage(req: PlanRequest): void;
  /** 計算を打ち切る */
  terminate(): void;
}

/** 後から来た依頼に取って代わられた（結果は使わない） */
export class SupersededError extends Error {
  constructor() {
    super("superseded by a newer layout request");
    this.name = "SupersededError";
  }
}

const MEMORY_PLANS = 8;

interface Inflight {
  key: string;
  worker: PlanWorker;
  promise: Promise<LayoutPlan>;
  cancel: () => void;
}

/**
 * 配置の計算の窓口。同じ入力（鍵）の結果は、メモリ → ブラウザの保存領域 → ワーカーの順に探し、計算したら両方に残す。
 * 計算中に別の入力の依頼が来たら、前のものは打ち切って（ワーカーを止めて）捨てる。同じ入力の依頼は、計算中のものを待つ。
 */
export function createPlanService(deps: { spawn: () => PlanWorker; cache?: PlanCache }) {
  const memory = new Map<string, LayoutPlan>();
  let latest = 0;
  let inflight: Inflight | null = null;

  const remember = (key: string, plan: LayoutPlan) => {
    memory.delete(key);
    memory.set(key, plan);
    for (const old of [...memory.keys()].slice(0, Math.max(0, memory.size - MEMORY_PLANS)))
      memory.delete(old);
  };

  const compute = (key: string, full: DiagramModel): Inflight => {
    const worker = deps.spawn();
    let fail: (e: Error) => void = () => {};
    const promise = new Promise<LayoutPlan>((resolve, reject) => {
      fail = reject;
      worker.onmessage = (e) => {
        if ("error" in e.data) reject(new Error(e.data.error));
        else resolve(e.data.plan);
      };
      worker.onerror = (e) => reject(new Error(e.message ?? "layout worker failed"));
      worker.postMessage({ id: latest, full });
    });
    const job: Inflight = { key, worker, promise, cancel: () => fail(new SupersededError()) };
    const done = () => {
      worker.terminate();
      if (inflight === job) inflight = null;
    };
    // 結果は、計算が終わった時点でメモリに残す（待っている依頼の続きより先に。同じ入力の次の依頼が、
    // 終わったばかりの計算を見失って計算し直さないように）
    promise.then((plan) => {
      remember(key, plan);
      done();
    }, done);
    return job;
  };

  return {
    /** `full` の配置を返す。後から別の依頼が来たら `SupersededError` で終わる */
    async plan(full: DiagramModel): Promise<LayoutPlan> {
      const id = ++latest;
      const key = await planKey(full);
      // 保存領域を待っている間に、同じ入力の計算が終わっていることがあるので、メモリをもう一度見る
      const hit = memory.get(key) ?? (await deps.cache?.get(key)) ?? memory.get(key) ?? null;
      if (id !== latest) throw new SupersededError();
      if (hit) {
        remember(key, hit);
        // この依頼で要らなくなった計算（前の依頼のもの）は止める
        if (inflight && inflight.key !== key) inflight.cancel();
        return hit;
      }
      if (inflight?.key !== key) {
        inflight?.cancel();
        inflight = compute(key, full);
      }
      const job = inflight;
      const plan = await job.promise;
      remember(key, plan);
      void deps.cache?.put(key, plan);
      if (id !== latest) throw new SupersededError();
      return plan;
    },
  };
}

export type PlanService = ReturnType<typeof createPlanService>;
