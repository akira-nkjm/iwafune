import { prefetchSystemAnalyses } from "@/api/queries";
import type { System } from "@/api/types";

/**
 * 系のページを開く前に読んでおくもの: その系の解析の結果（系のページと同じキーでキャッシュに入る）と、
 * 登録表（`dedicated.tsx` の `SYSTEM_REGISTRY`）にある大きなチャンク（熱の温度で塗った 3D は three.js）。
 * 登録表の本体は重いので、ナビの先読み（`routes/pages.ts`）はこの軽いモジュールだけを読み、登録表は動的に読む。
 * 失敗はページを開いたときに出すので、呼び手は捨ててよい。
 */
export function preloadSystem(system: System): Promise<unknown>[] {
  const chunks = import("./dedicated").then(({ systemChunks }) =>
    Promise.all(systemChunks(system).map((load) => load())),
  );
  return [prefetchSystemAnalyses(system), chunks];
}
