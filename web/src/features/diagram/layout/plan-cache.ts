import type { LayoutPlan } from "./layout";
import { LAYOUT_VERSION } from "./plan-key";

/** ブラウザの保存領域に残す 1 件 */
export interface StoredPlan {
  key: string;
  version: number;
  /** 最後に使った時刻（古いものから捨てる） */
  usedAt: number;
  plan: unknown;
}

/** 保存領域（IndexedDB など）への出し入れ。失敗したら例外を投げてよい（`PlanCache` が飲み込む） */
export interface PlanStorage {
  get(key: string): Promise<StoredPlan | undefined>;
  put(entry: StoredPlan): Promise<void>;
  delete(key: string): Promise<void>;
  /** 全部の件の鍵と最後に使った時刻 */
  list(): Promise<{ key: string; usedAt: number }[]>;
}

/** 残す件数の上限（図 1 枚は数十 KB。設計や表示の組み合わせ数件分） */
export const MAX_STORED_PLANS = 12;

const isMap = (v: unknown): boolean => v instanceof Map;

/** 読み出した値が配置の結果の形をしているか（壊れた・古い形のものを使わないため） */
export function isLayoutPlan(v: unknown): v is LayoutPlan {
  if (!v || typeof v !== "object") return false;
  const { tree, geometry } = v as {
    tree?: Record<string, unknown>;
    geometry?: Record<string, unknown>;
  };
  if (!tree || !geometry) return false;
  return (
    Array.isArray(tree.roots) &&
    isMap(tree.children) &&
    isMap(tree.parent) &&
    tree.assemblies instanceof Set &&
    tree.frames instanceof Set &&
    ["boxes", "absolute", "ports", "taps", "routes", "labels"].every((k) => isMap(geometry[k]))
  );
}

/**
 * 配置の結果の置き場（保存領域の上の、版の確認・形の確認・件数の上限）。どこかで失敗しても、結果が無かったものとして
 * 扱う（残せなくても図は計算できる）。壊れた件・古い版の件は、見つけたときに消す。
 */
export class PlanCache {
  private readonly storage: PlanStorage;
  private readonly max: number;
  private readonly now: () => number;

  constructor(storage: PlanStorage, max = MAX_STORED_PLANS, now: () => number = Date.now) {
    this.storage = storage;
    this.max = max;
    this.now = now;
  }

  async get(key: string): Promise<LayoutPlan | null> {
    try {
      const entry = await this.storage.get(key);
      if (!entry) return null;
      if (entry.key !== key || entry.version !== LAYOUT_VERSION || !isLayoutPlan(entry.plan)) {
        await this.storage.delete(key);
        return null;
      }
      await this.storage.put({ ...entry, usedAt: this.now() });
      return entry.plan;
    } catch {
      return null;
    }
  }

  async put(key: string, plan: LayoutPlan): Promise<void> {
    try {
      await this.storage.put({ key, version: LAYOUT_VERSION, usedAt: this.now(), plan });
      const all = (await this.storage.list()).toSorted((a, b) => a.usedAt - b.usedAt);
      for (const old of all.slice(0, Math.max(0, all.length - this.max)))
        await this.storage.delete(old.key);
    } catch {
      /* 保存領域が使えない・いっぱいでも、図は出せる（残さないだけ） */
    }
  }
}

const DB_NAME = "iwafune.diagram.plans";
const STORE = "plans";

const request = <T>(r: IDBRequest<T>): Promise<T> =>
  new Promise((resolve, reject) => {
    r.onsuccess = () => resolve(r.result);
    r.onerror = () => reject(r.error);
  });

/** IndexedDB の保存領域。開けなければ（プライベートモードなど）例外を投げ、`PlanCache` が無かったことにする */
export function indexedDbStorage(factory: IDBFactory = indexedDB): PlanStorage {
  let opened: Promise<IDBDatabase> | null = null;
  const open = () => {
    opened ??= new Promise<IDBDatabase>((resolve, reject) => {
      const r = factory.open(DB_NAME, 1);
      r.onupgradeneeded = () => r.result.createObjectStore(STORE, { keyPath: "key" });
      r.onsuccess = () => resolve(r.result);
      r.onerror = () => reject(r.error);
    });
    return opened;
  };
  const store = async (mode: IDBTransactionMode) =>
    (await open()).transaction(STORE, mode).objectStore(STORE);
  return {
    get: async (key) =>
      (await request((await store("readonly")).get(key))) as StoredPlan | undefined,
    put: async (entry) => void (await request((await store("readwrite")).put(entry))),
    delete: async (key) => void (await request((await store("readwrite")).delete(key))),
    list: async () =>
      ((await request((await store("readonly")).getAll())) as StoredPlan[]).map((e) => ({
        key: e.key,
        usedAt: e.usedAt,
      })),
  };
}
