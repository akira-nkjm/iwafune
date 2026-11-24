import { describe, expect, it } from "vitest";
import type { LayoutPlan } from "./layout";
import { isLayoutPlan, PlanCache, type PlanStorage, type StoredPlan } from "./plan-cache";
import { LAYOUT_VERSION } from "./plan-key";

const planOf = (tag: string): LayoutPlan => ({
  tree: {
    roots: [tag],
    children: new Map(),
    parent: new Map(),
    assemblies: new Set(),
    frames: new Set(),
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

/** メモリ上の保存領域（`broken` で全部の出し入れを失敗させられる） */
function memoryStorage() {
  const rows = new Map<string, StoredPlan>();
  const state = { broken: false };
  const storage: PlanStorage = {
    get: async (k) => {
      if (state.broken) throw new Error("storage unavailable");
      return rows.get(k);
    },
    put: async (e) => {
      if (state.broken) throw new Error("storage unavailable");
      rows.set(e.key, e);
    },
    delete: async (k) => void rows.delete(k),
    list: async () => [...rows.values()].map((e) => ({ key: e.key, usedAt: e.usedAt })),
  };
  return { rows, state, storage };
}

describe("配置の結果のブラウザの保存領域", () => {
  it("同じ鍵で残した結果が戻り、無い鍵は null", async () => {
    const { storage } = memoryStorage();
    const cache = new PlanCache(storage);
    await cache.put("a", planOf("a"));
    expect((await cache.get("a"))?.tree.roots).toEqual(["a"]);
    expect(await cache.get("b")).toBeNull();
  });

  it("件数の上限を超えたら、最後に使ったのが古いものから捨てる（読んだものは新しい扱い）", async () => {
    const { rows, storage } = memoryStorage();
    let t = 0;
    const cache = new PlanCache(storage, 3, () => ++t);
    for (const k of ["a", "b", "c"]) await cache.put(k, planOf(k));
    await cache.get("a");
    await cache.put("d", planOf("d"));
    await cache.put("e", planOf("e"));
    expect(rows.size).toBe(3);
    expect([...rows.keys()].sort()).toEqual(["a", "d", "e"]);
  });

  it("壊れた件・古い版の件は使わず、消す", async () => {
    const { rows, storage } = memoryStorage();
    const cache = new PlanCache(storage);
    rows.set("bad", { key: "bad", version: LAYOUT_VERSION, usedAt: 1, plan: { tree: 1 } });
    rows.set("old", { key: "old", version: LAYOUT_VERSION - 1, usedAt: 1, plan: planOf("old") });
    rows.set("other", { key: "x", version: LAYOUT_VERSION, usedAt: 1, plan: planOf("x") });
    for (const k of ["bad", "old", "other"]) expect(await cache.get(k)).toBeNull();
    expect(rows.size).toBe(0);
  });

  it("保存領域が使えなくても例外にせず、無かったものとして扱う", async () => {
    const { state, storage } = memoryStorage();
    const cache = new PlanCache(storage);
    state.broken = true;
    await expect(cache.put("a", planOf("a"))).resolves.toBeUndefined();
    expect(await cache.get("a")).toBeNull();
  });

  it("結果の形の確認: Map・Set が揃っていること", () => {
    expect(isLayoutPlan(planOf("a"))).toBe(true);
    expect(isLayoutPlan(JSON.parse(JSON.stringify(planOf("a"))))).toBe(false);
    expect(isLayoutPlan(null)).toBe(false);
  });
});
