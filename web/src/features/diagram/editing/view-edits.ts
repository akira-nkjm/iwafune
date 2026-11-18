import { useCallback } from "react";
import { useApplyOps } from "@/api/ops";
import type { Op, ViewEdge } from "@/api/types";
import { getT } from "@/i18n";

/** 系統図の配置の図（view/electrical.toml） */
export const VIEW = "electrical";

export type ViewOp = Extract<
  Op,
  { op: "set_view_layout" | "set_view_edge" | "unset_view_edge" | "reset_view" }
>;

/**
 * 系統図の配置の編集を送る。元に戻す・やり直すは画面全体の `POST /undo`・`POST /redo`（`api/history.ts`。
 * 配置の編集も普通の Op なので、サーバーの履歴にほかの編集と同じ順で積まれる）。送れなかったら `onError` を呼ぶ。
 */
export function useViewEdits() {
  const { mutate } = useApplyOps();
  const send = useCallback(
    (ops: ViewOp[], summary: string, onError?: () => void) =>
      mutate({ ops, summary }, { onError: () => onError?.() }),
    [mutate],
  );
  /** 線の経路を書く・消す（`edge` を省くと消す）。`key` が null なら全体をリセット */
  const editRoute = useCallback(
    (key: string | null, edge?: ViewEdge, onError?: () => void) => {
      const op: ViewOp =
        key === null
          ? { op: "reset_view", view: VIEW }
          : edge
            ? { op: "set_view_edge", view: VIEW, key, edge }
            : { op: "unset_view_edge", view: VIEW, key };
      const t = getT().diagram;
      const summary = key === null ? t.resetLayout : edge ? t.editRoute : t.resetRoute;
      send([op], summary, onError);
    },
    [send],
  );
  return { send, editRoute };
}
