import { useQuery } from "@tanstack/react-query";
import { useCommandOps } from "@/api/ops";
import { q } from "@/api/queries";
import type { DefsView, ModeTable, Op, System, SystemModesView } from "@/api/types";
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
 * `system` が無い（設計のモードのページ）ときは、足もとの合計の行だけ（電力の合計。`summary: "totals"`。行は service が選ぶ）を
 * 同じ下書きで、API が合計を返す系（`DefsView.mode_totals_system`）に問い合わせる。部品の行・セルは系の値を使わない。
 */
export function useModeDraftLoad(
  table: ModeTable,
  draft: ModeDraft,
  defs: Pick<DefsView, "mode_totals_system">,
  system?: System,
): ModeDraftLoad {
  const made = useCommandOps(draftCommand(table, draft));
  const ops = made.latest ?? NO_OPS;
  const view = useQuery(
    q.systemModes(system ?? defs.mode_totals_system, ops, system ? {} : { summary: "totals" }),
  );
  return {
    save: { ops: made.ops, error: made.error },
    view: {
      data: view.data ?? null,
      pending: view.isPending,
      updating: view.isFetching && !view.isPending,
    },
    error: view.error?.message ?? null,
  };
}
