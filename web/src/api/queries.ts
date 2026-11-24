import {
  keepPreviousData,
  QueryClient,
  queryOptions,
  useQueries,
  useQuery,
} from "@tanstack/react-query";
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
  ComplianceReport,
  ConditionTopic,
  DataStatus,
  DefsView,
  DesignView,
  HistoryView,
  ModeSummaryScope,
  Op,
  PartQuery,
  PartRow,
  PartsQuery,
  PartView,
  PartViewRequest,
  ProjectInfo,
  RunAnalysisRequest,
  System,
  SystemModesRequest,
  SystemModesView,
  UnitsCatalog,
} from "./types";

export const queryClient = new QueryClient({
  defaultOptions: {
    queries: { staleTime: 10_000, retry: 1, refetchOnWindowFocus: true },
  },
});

/** `q.systemModes` の指定 */
export interface SystemModesOptions {
  /** 解析ケースの ID（`analyzed` の解析に使う。部品別の値はケースを取らない） */
  kase?: string | null;
  /** 系の解析も回す */
  analyzed?: boolean;
  /** 足もとの行の範囲 */
  summary?: ModeSummaryScope;
}

function configurationSearch(configuration: PartQuery["configuration"]): string {
  return configuration === null ? "" : `?configuration=${seg(configuration)}`;
}

/** 読み取りの queryOptions（画面は useQuery(q.xxx()) で使う） */
export const q = {
  project: () =>
    queryOptions({ queryKey: keys.project, queryFn: () => api<ProjectInfo>("/project") }),
  design: () => queryOptions({ queryKey: keys.design, queryFn: () => api<DesignView>("/design") }),
  /**
   * 系別のモード表示（`POST /system_modes/{system}`）。`ops` は保存していない編集（書き込まずに当てる）。
   * 既定は部品別の値の表だけ（解析は回さない）。`analyzed` を渡すと系の解析も回して解析に基づく表示を足す
   * （電源: モードごとの電力の行き先・判定・グラフ。`kase` のケースで）。`summary` は足もとの行の範囲（service が行を選ぶ）。
   */
  systemModes: (system: System, ops: readonly Op[] = [], options: SystemModesOptions = {}) => {
    const { kase = null, analyzed = false, summary = "all" } = options;
    const opsJson = JSON.stringify(ops);
    return queryOptions({
      queryKey: analyzed
        ? keys.analyzedModes(system, kase, opsJson)
        : keys.systemModes(system, opsJson, summary),
      queryFn: () =>
        api<SystemModesView>(`/system_modes/${seg(system)}`, {
          method: "POST",
          body: { ops: [...ops], case: kase, analyzed, summary } satisfies SystemModesRequest,
        }),
      placeholderData: keepPreviousData,
      ...(analyzed ? { staleTime: 30_000 } : {}),
    });
  },
  /** 部品の一覧。形態の指定があれば、その形態での取付・姿勢を読む。 */
  parts: (configuration: PartsQuery["configuration"] = null) =>
    queryOptions({
      queryKey: keys.parts(configuration),
      queryFn: () => api<PartRow[]>(`/parts${configurationSearch(configuration)}`),
    }),
  /** 1 部品。`ops` を渡すと、保存していない編集を書き込まずに当てた設計の部品（`POST /parts/{id}`） */
  part: (id: string, ops: readonly Op[] = [], configuration: PartQuery["configuration"] = null) => {
    const json = ops.length > 0 ? JSON.stringify(ops) : "";
    const path = `/parts/${seg(id)}`;
    return queryOptions({
      queryKey: keys.part(id, json, configuration),
      queryFn: () => {
        if (!json) return api<PartView>(`${path}${configurationSearch(configuration)}`);
        const body: PartViewRequest = { ops: [...ops], configuration };
        return api<PartView>(path, { method: "POST", body });
      },
    });
  },
  /** 編集の履歴（取り消し・やり直しのボタンの有効・無効） */
  history: () =>
    queryOptions({ queryKey: keys.history, queryFn: () => api<HistoryView>("/history") }),
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
  /** 解析の実行。`ops` を渡すと、保存していない編集を書き込まずに当てた設計で計算する。`configuration` はその形態の配置で計算する */
  analysis: (
    id: string,
    kase: string | null,
    ops: readonly Op[] = [],
    configuration: string | null = null,
  ) => {
    const json = ops.length > 0 ? JSON.stringify(ops) : "";
    return queryOptions({
      queryKey: keys.analysis(id, kase, json, configuration),
      queryFn: () => {
        const body: RunAnalysisRequest = {
          case: kase,
          ...(configuration ? { configuration } : {}),
          source: { kind: "design" },
          ops: [...ops],
        };
        return api<AnalysisRunResult>(`/analyses/${seg(id)}`, { method: "POST", body });
      },
      staleTime: 30_000,
      // 下書きを変えるたびにキーが変わる。次の結果が来るまで前の結果を出しておく（数字が消えて跳ねないように）
      placeholderData: keepPreviousData,
    });
  },
  /** 外部データ（DE440）の状態。取得は `api/data.ts` の `useFetchData` */
  dataStatus: () =>
    queryOptions({ queryKey: keys.dataStatus, queryFn: () => api<DataStatus>("/data/status") }),
  /** 要求適合表（要求ごとに、確かめる先の解析・バジェットを量化したケースで実行して判定を集める。重いので 30 秒は使い回す） */
  compliance: () =>
    queryOptions({
      queryKey: keys.compliance,
      queryFn: () => api<ComplianceReport>("/requirements/compliance"),
      staleTime: 30_000,
      placeholderData: keepPreviousData,
    }),
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
  void queryClient.invalidateQueries({ queryKey: keys.analyzedModesAll });
  for (const id of ids) void queryClient.invalidateQueries({ queryKey: keys.analysis(id, kase) });
}

/** 解析 1 つの実行の状態（`useAnalyses` の 1 要素） */
export interface AnalysisState {
  id: string;
  result: AnalysisRunResult | undefined;
  error: string | null;
  loading: boolean;
  /** 前の結果を出したまま、次の結果を問い合わせている */
  updating: boolean;
}

/**
 * 解析をまとめて実行する（POST /analyses/{id}。結果は設計の編集で無効化される）。
 * `ops` を渡すと、保存していない編集（下書き）を当てた設計で計算する。系のページ・概要・軌道・モードが使う。
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

/**
 * ページに並べる解析: 設計データから入力を作れるもの（API の `projectable`）だけ。作れない解析（入力を JSON で渡す
 * 解析）は並べない（走らせると「実装していない」という失敗になり、概要・判定の集計にも入ってしまう）。
 * 作れない解析は CLI（`fune analyze <id> --input`・`fune analyze ls`）で使う。
 */
function listable(entries: readonly AnalysisEntry[]): AnalysisEntry[] {
  return entries.filter((e) => e.projectable);
}

/** ある系の解析。系への所属は sat-analysis の info.area が正本 */
function systemEntries(entries: readonly AnalysisEntry[], system: string | null): AnalysisEntry[] {
  return listable(entries).filter(
    (e) => e.info.area.kind === "system" && e.info.area.value === system,
  );
}

/** 解析の一覧（GET /analyses）から、ある系の解析（一覧の順）とその ID を引く */
export function useSystemAnalyses(system: string | null) {
  const catalog = useQuery(q.analyses());
  const entries = systemEntries(catalog.data ?? [], system);
  return { entries, ids: entries.map((e) => e.info.id), loading: catalog.isPending };
}

/**
 * ある系の解析を先に計算させておく（ナビにホバーしたとき）。結果は系のページと同じキーでキャッシュに入る。
 * 失敗はページを開いたときの問い合わせが出すので、ここでは捨てる。
 */
export async function prefetchSystemAnalyses(system: string): Promise<void> {
  try {
    const entries = await queryClient.fetchQuery(q.analyses());
    await Promise.all(
      systemEntries(entries, system).map((e) =>
        queryClient.prefetchQuery(q.analysis(e.info.id, null)),
      ),
    );
  } catch {
    // 一覧が取れなかった。ページが同じ問い合わせをしてエラーを出す
  }
}

/** 前提条件の話題に属する解析（系の解析とは別の区分）。 */
export function useConditionAnalyses(topic: ConditionTopic) {
  const catalog = useQuery(q.analyses());
  const entries = listable(catalog.data ?? []).filter(
    (e) => e.info.area.kind === "condition" && e.info.area.value === topic,
  );
  return { entries, ids: entries.map((e) => e.info.id), loading: catalog.isPending };
}
