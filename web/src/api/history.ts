import {
  type QueryClient,
  type UseMutationOptions,
  useMutation,
  useQuery,
  useQueryClient,
} from "@tanstack/react-query";
import { toast } from "sonner";
import { getT } from "@/i18n";
import { recordChange } from "@/lib/history";
import { type ApiError, api } from "./client";
import { keys, retiredBetween } from "./keys";
import { q } from "./queries";
import { refreshDerived } from "./refresh";
import type { DesignView, HistoryAction, HistoryStepResult, HistoryView } from "./types";

/** 取り消し・やり直しの直前に呼ぶ処理（画面が持つ保存待ちの状態を捨てるため） */
const beforeStep = new Set<() => void>();

/** 取り消し・やり直しの前に `callback` を呼ぶ。返す関数で外す */
export function onBeforeHistoryStep(callback: () => void): () => void {
  beforeStep.add(callback);
  return () => {
    beforeStep.delete(callback);
  };
}

/** 編集の履歴（取り消し・やり直しのボタンの有効・無効に使う） */
export function useHistoryView(): HistoryView | undefined {
  return useQuery(q.history()).data;
}

/**
 * 設計から導出する問い合わせを読み直す（編集と同じ範囲）。
 * 取り消し・やり直しは何が消えたかを返さないので、設計を先に読み直して前後を比べ、消えた・名前が戻った対象を
 * 指す問い合わせは編集と同じく読み直さない（読み直すと、無い対象を読みに行って 404 になる）。
 */
async function refresh(qc: QueryClient) {
  const before = qc.getQueryData<DesignView>(keys.design);
  const after = before
    ? await qc.fetchQuery({ ...q.design(), staleTime: 0 }).catch(() => null)
    : null;
  await refreshDerived(qc, before && after ? retiredBetween(before, after) : undefined);
}

/** 取り消し・やり直しの記録の見出し（元の編集に対応する CLI のコマンドの先頭。無ければ変わったファイル） */
function summaryOf(r: HistoryStepResult): string {
  const t = getT().ops;
  const what = r.edit_cli[0] ?? r.changed.map((c) => c.path).join(", ");
  const label = what || t.lastEdit;
  return r.action === "undo" ? t.undoSummary(label) : t.redoSummary(label);
}

/**
 * 直前の編集を取り消す・取り消した編集をやり直す（`POST /undo`・`POST /redo`）。
 * 成功したら変更履歴に記録し、設計から導出する問い合わせを読み直す。ほかで書き換わっていて断られたら、その理由を出す。
 */
export function useHistoryStep() {
  const qc = useQueryClient();
  return useMutation(historyStepOptions(qc));
}

/** 取り消し・やり直しの mutation の設定（`useHistoryStep` と、hook を通さない検査が使う） */
export function historyStepOptions(
  qc: QueryClient,
): UseMutationOptions<HistoryStepResult, ApiError, HistoryAction> {
  return {
    mutationFn: (action) => {
      for (const callback of beforeStep) callback();
      return api<HistoryStepResult>(`/${action}`, { method: "POST" });
    },
    onSuccess: async (r) => {
      const summary = summaryOf(r);
      const files = r.changed.map((c) => c.path);
      recordChange({ summary, cli: [], files, kind: r.action });
      await refresh(qc);
      toast.success(summary, { description: getT().ops.changed(files.join(", ")) });
    },
    onError: async (e, action) => {
      const t = getT().ops;
      await refresh(qc);
      if (e.code === "history_empty") {
        toast.message(action === "undo" ? t.nothingToUndo : t.nothingToRedo);
        return;
      }
      toast.error(t.stepFailed(action === "redo"), { description: e.message });
    },
  };
}
