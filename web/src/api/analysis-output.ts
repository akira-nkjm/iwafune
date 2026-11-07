import type { AnalysisOutputs, AnalysisRunResult } from "./types";

/** ID と成功状態を確認する型付き出力の唯一の入口。レスポンスの形は Rust が保証する。 */
export function analysisOutput<K extends keyof AnalysisOutputs>(
  result: AnalysisRunResult | null | undefined,
  id: K,
): AnalysisOutputs[K] | null {
  return result?.id === id && result.status === "ok" && result.output !== null
    ? (result.output as AnalysisOutputs[K])
    : null;
}
