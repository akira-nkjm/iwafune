import { afterPaint } from "@/lib/after-paint";
import { indexedDbStorage, PlanCache } from "./plan-cache";
import { createPlanService, type PlanWorker } from "./plan-service";
import { handlePlanRequest } from "./plan-worker-core";

/**
 * ワーカーを使えない環境（古いブラウザなど）の代わり。主スレッドで、画面を描いたあとに計算する。
 * 計算の間は画面が止まるが、結果は同じ。
 */
function inlineWorker(): PlanWorker {
  let cancelled = false;
  const w: PlanWorker = {
    onmessage: null,
    onerror: null,
    postMessage: (req) => {
      afterPaint(() => {
        if (!cancelled) w.onmessage?.({ data: handlePlanRequest(req) });
      });
    },
    terminate: () => {
      cancelled = true;
    },
  };
  return w;
}

function spawnWorker(): PlanWorker {
  if (typeof Worker === "undefined") return inlineWorker();
  return new Worker(new URL("./plan.worker.ts", import.meta.url), {
    type: "module",
  }) as unknown as PlanWorker;
}

function browserCache(): PlanCache | undefined {
  try {
    return typeof indexedDB === "undefined" ? undefined : new PlanCache(indexedDbStorage());
  } catch {
    return undefined;
  }
}

/** 系統図の配置の計算の窓口（ページ全体で 1 つ。メモリとブラウザの保存領域に結果を残す） */
export const diagramPlans = createPlanService({ spawn: spawnWorker, cache: browserCache() });
