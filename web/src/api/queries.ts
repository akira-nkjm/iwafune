import { keepPreviousData, QueryClient, queryOptions, useQuery } from "@tanstack/react-query";
import { useDebounced } from "@/lib/use-debounced";
import { api, seg } from "./client";
import { keys } from "./keys";
import type {
  AnalysisEntry,
  AnalysisRunResult,
  BudgetKind,
  BudgetRequest,
  BudgetView,
  CheckReport,
  DefsView,
  DesignView,
  Op,
  PartRow,
  PartView,
  PartViewRequest,
  ProjectInfo,
  RunAnalysisRequest,
  UnitsCatalog,
} from "./types";

export const queryClient = new QueryClient({
  defaultOptions: {
    queries: { staleTime: 10_000, retry: 1, refetchOnWindowFocus: true },
  },
});

/** 読み取りの queryOptions（画面は useQuery(q.xxx()) で使う） */
export const q = {
  project: () =>
    queryOptions({ queryKey: keys.project, queryFn: () => api<ProjectInfo>("/project") }),
  design: () => queryOptions({ queryKey: keys.design, queryFn: () => api<DesignView>("/design") }),
  /** 1 部品。`ops` を渡すと、保存していない編集を書き込まずに当てた設計の部品（`POST /parts/{id}`） */
  part: (id: string, ops: readonly Op[] = []) => {
    const json = ops.length > 0 ? JSON.stringify(ops) : "";
    return queryOptions({
      queryKey: keys.part(id, json),
      queryFn: () => {
        if (!json) return api<PartView>(`/parts/${seg(id)}`);
        const body: PartViewRequest = { ops: [...ops] };
        return api<PartView>(`/parts/${seg(id)}`, { method: "POST", body });
      },
    });
  },
  defs: () => queryOptions({ queryKey: keys.defs, queryFn: () => api<DefsView>("/defs") }),
  diagnostics: () =>
    queryOptions({
      queryKey: keys.diagnostics,
      queryFn: () => api<CheckReport>("/diagnostics"),
    }),
  units: (dimension?: string | null) =>
    queryOptions({
      queryKey: keys.units(dimension),
      queryFn: () => api<UnitsCatalog>(dimension ? `/units?dimension=${seg(dimension)}` : "/units"),
      staleTime: Number.POSITIVE_INFINITY,
    }),
  analyses: () =>
    queryOptions({
      queryKey: keys.analyses,
      queryFn: () => api<AnalysisEntry[]>("/analyses"),
      staleTime: Number.POSITIVE_INFINITY,
    }),
  /** 解析の実行。`ops` を渡すと、保存していない編集を書き込まずに当てた設計で計算する */
  analysis: (id: string, kase: string | null, ops: readonly Op[] = []) => {
    const json = ops.length > 0 ? JSON.stringify(ops) : "";
    return queryOptions({
      queryKey: keys.analysis(id, kase, json),
      queryFn: () => {
        const body: RunAnalysisRequest = {
          case: kase,
          input_patch: null,
          input: null,
          ops: [...ops],
        };
        return api<AnalysisRunResult>(`/analyses/${seg(id)}`, { method: "POST", body });
      },
      staleTime: 30_000,
      // 下書きを変えるたびにキーが変わる。次の結果が来るまで前の結果を出しておく（数字が消えて跳ねないように）
      placeholderData: keepPreviousData,
    });
  },
  /** バジェット。`ops` の意味は analysis と同じ（下書きがあれば POST で送る） */
  budget: (kind: BudgetKind, ops: readonly Op[] = []) => {
    const json = ops.length > 0 ? JSON.stringify(ops) : "";
    return queryOptions({
      queryKey: keys.budget(kind, json),
      queryFn: () => {
        if (!json) return api<BudgetView>(`/budgets/${seg(kind)}`);
        const body: BudgetRequest = { case: null, ops: [...ops] };
        return api<BudgetView>(`/budgets/${seg(kind)}`, { method: "POST", body });
      },
      placeholderData: keepPreviousData,
    });
  },
};

/** 画面全体で使う設計データ（ほとんどのページはこれ 1 つから表を作る） */
export function useDesign() {
  return useQuery(q.design());
}

/**
 * 保存していない編集（ops）を書き込まずに当てた部品（`POST /parts/{id}`）。ops が null か空なら問い合わせない。
 * 打つ間は 250 ms 待ってから問い合わせる。同じ編集で定義を作る属性のパスを、その部品の `paths` から引くのに使う
 * （Web はパスをつながない）。
 */
export function useDraftPart(
  id: string,
  ops: readonly Op[] | null,
): { part: PartRow | null; error: string | null } {
  // 文字列で待つ（毎回作り直す配列を待つと、描き直すたびに待ち直す）
  const now = ops && ops.length > 0 ? JSON.stringify(ops) : "";
  const json = useDebounced(now, 250);
  const query = useQuery({
    ...q.part(id, json ? (JSON.parse(json) as Op[]) : []),
    enabled: json.length > 0,
    retry: false,
  });
  const settled = json === now && now.length > 0;
  return {
    part: settled ? (query.data?.part ?? null) : null,
    error: settled ? (query.error?.message ?? null) : null,
  };
}

/** 解析を計算し直す（設計の編集の後は api/ops.ts が自動で無効化する。これは「再計算」ボタン用） */
export function recomputeAnalyses(ids: readonly string[], kase: string | null): void {
  for (const id of ids) void queryClient.invalidateQueries({ queryKey: keys.analysis(id, kase) });
}

/** ある系の解析の ID。系への所属は sat-analysis の info.system が正本 */
function systemAnalysisIds(entries: readonly AnalysisEntry[], system: string | null): string[] {
  return entries.filter((e) => e.info.system === system).map((e) => e.info.id);
}

/** 解析の一覧（GET /analyses）から、ある系の解析を引く */
export function useSystemAnalyses(system: string | null) {
  const catalog = useQuery(q.analyses());
  const entries = catalog.data ?? [];
  return { entries, ids: systemAnalysisIds(entries, system), loading: catalog.isPending };
}

/**
 * ある系の解析を先に計算させておく（ナビにホバーしたとき）。結果は系のページと同じキーでキャッシュに入る。
 * 失敗はページを開いたときの問い合わせが出すので、ここでは捨てる。
 */
export async function prefetchSystemAnalyses(system: string): Promise<void> {
  try {
    const entries = await queryClient.fetchQuery(q.analyses());
    await Promise.all(
      systemAnalysisIds(entries, system).map((id) =>
        queryClient.prefetchQuery(q.analysis(id, null)),
      ),
    );
  } catch {
    // 一覧が取れなかった。ページが同じ問い合わせをしてエラーを出す
  }
}
