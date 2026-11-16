import { prefetchSystemAnalyses } from "@/api/queries";
import type { System } from "@/api/types";
import { loadLayout3D } from "@/features/layout3d/LazyLayout3D";

/**
 * 大きなチャンクを使う追加の表示（`extras.tsx` の `systemExtras`）を持つ系と、そのチャンク（熱の温度で塗った 3D は three.js）。
 * **大きなチャンクを使う系はここだけに書く**: `systemExtras` はこの系を型（`ExtraSystem`）で突き合わせる。
 * 表示の本体（extras.tsx）は大きいので、ナビの先読み（`routes/pages.ts`）はこちらの軽いモジュールだけを読む。
 */
export const EXTRA_CHUNKS = {
  thermal: [loadLayout3D],
} as const satisfies Partial<Record<System, readonly (() => Promise<unknown>)[]>>;

/** 追加の表示を持つ系 */
export type ExtraSystem = keyof typeof EXTRA_CHUNKS;

export function hasExtras(system: System): system is ExtraSystem {
  return system in EXTRA_CHUNKS;
}

/**
 * 系のページを開く前に読んでおくもの: その系の解析の結果（系のページと同じキーでキャッシュに入る）と、
 * 追加の表示のチャンク。失敗はページを開いたときに出すので、呼び手は捨ててよい。
 */
export function preloadSystem(system: System): Promise<unknown>[] {
  const chunks = hasExtras(system) ? EXTRA_CHUNKS[system].map((load) => load()) : [];
  return [prefetchSystemAnalyses(system), ...chunks];
}
