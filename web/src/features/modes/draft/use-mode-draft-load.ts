import { useQuery } from "@tanstack/react-query";
import { analysisOutput } from "@/api/analysis-output";
import { useCommandOps } from "@/api/ops";
import { q, useAnalyses } from "@/api/queries";
import type { BudgetView, ModeLoadOutput, ModeTable, Op } from "@/api/types";
import { draftCommand, type ModeDraft } from "./draft";

const NO_OPS: readonly Op[] = [];

/** 下書きを当てて計算した値と、その問い合わせの段階 */
export interface DraftComputed<T> {
  data: T;
  /** まだ一度も値が無い */
  pending: boolean;
  /** 前の値を出したまま、次の値を問い合わせている（前の数字を薄く出す） */
  updating: boolean;
}

/** `useModeDraftLoad` の戻り値 */
export interface ModeDraftLoad {
  /** 保存に送る Op（下書きを service が Op にしたもの。待っている間と読めないときは null）と、Op にできなかった理由 */
  save: { ops: Op[] | null; error: string | null };
  /** 下書きを当てた power.mode_load の出力（セルの電力） */
  load: DraftComputed<ModeLoadOutput | null>;
  /** 下書きを当てた電力のバジェット（足もとの合計・使用可能電力・余裕。`BudgetView.modes`） */
  budget: DraftComputed<BudgetView | undefined>;
}

/**
 * モード表の下書きの問い合わせ: 下書きを `fune mode set|unset` の引数にして service で Op にし（`useCommandOps`）、
 * それを書き込まずに当てた電力（power.mode_load）とバジェットを問い合わせる。行に並べるのは `matrixGroups`、
 * 表示は `ModesTable`。Op を待つ間も前の Op で計算を出しておく（`latest`）。
 */
export function useModeDraftLoad(table: ModeTable, draft: ModeDraft): ModeDraftLoad {
  const made = useCommandOps(draftCommand(table, draft));
  const ops = made.latest ?? NO_OPS;
  const [load] = useAnalyses(["power.mode_load"], null, ops);
  const budget = useQuery(q.budget("power", ops));
  return {
    save: { ops: made.ops, error: made.error },
    load: {
      data: analysisOutput(load?.result, "power.mode_load"),
      pending: load?.loading ?? true,
      updating: load?.updating ?? false,
    },
    budget: {
      data: budget.data,
      pending: budget.isPending,
      updating: budget.isFetching && !budget.isPending,
    },
  };
}
