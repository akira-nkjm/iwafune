import { useQueries } from "@tanstack/react-query";
import { q } from "@/api/queries";
import type { AnalysisRunResult, Op } from "@/api/types";

export interface AnalysisState {
  id: string;
  result: AnalysisRunResult | undefined;
  error: string | null;
  loading: boolean;
  /** 前の結果を出したまま、次の結果を問い合わせている */
  updating: boolean;
}

/**
 * 系の解析をまとめて実行する（POST /analyses/{id}。結果は設計の編集で無効化される）。
 * `ops` を渡すと、保存していない編集（下書き）を当てた設計で計算する。
 */
export function useAnalyses(
  ids: readonly string[],
  kase: string | null,
  ops: readonly Op[] = [],
): AnalysisState[] {
  return useQueries({
    queries: ids.map((id) => q.analysis(id, kase, ops)),
    combine: (rs) =>
      rs.map((r, i) => ({
        id: ids[i] ?? "",
        result: r.data,
        error: r.error?.message ?? null,
        loading: r.isPending,
        updating: r.isFetching && !r.isPending,
      })),
  });
}
