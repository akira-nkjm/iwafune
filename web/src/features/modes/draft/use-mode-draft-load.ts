import { useQuery } from "@tanstack/react-query";
import { useCommandOps } from "@/api/ops";
import { q } from "@/api/queries";
import type { ModeTable, Op, System, SystemModesView } from "@/api/types";
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
  view: DraftComputed<SystemModesView | null>;
  error: string | null;
}

/**
 * モード表の下書きの問い合わせ: 下書きを `fune mode set|unset` の引数にして service で Op にし（`useCommandOps`）、
 * それを書き込まずに当てた系別のモード表示を問い合わせる。行に並べるのは `matrixGroups`、
 * 表示は `ModesTable`。Op を待つ間も前の Op で計算を出しておく（`latest`）。
 */
export function useModeDraftLoad(
  table: ModeTable,
  draft: ModeDraft,
  system?: System,
): ModeDraftLoad {
  const made = useCommandOps(draftCommand(table, draft));
  const ops = made.latest ?? NO_OPS;
  const view = useQuery({ ...q.systemModes(system ?? "power", ops), enabled: !!system });
  return {
    save: { ops: made.ops, error: made.error },
    view: {
      data: system ? (view.data ?? null) : null,
      pending: !!system && view.isPending,
      updating: !!system && view.isFetching && !view.isPending,
    },
    error: system ? (view.error?.message ?? null) : null,
  };
}
