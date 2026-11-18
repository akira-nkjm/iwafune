import { useEffect, useRef } from "react";
import { useHistoryStep } from "@/api/history";
import { historyKey } from "@/lib/history-keys";

/**
 * 画面全体で ⌘Z / Ctrl+Z（元に戻す）と ⇧⌘Z / Ctrl+Y（やり直す）を受け、`POST /undo`・`POST /redo` を送る。
 * 文字を打つ欄の中では何もしない（欄自身の取り消しを使う。判定は `lib/history-keys.ts`）。送っている間のキーは無視する。
 */
export function useHistoryKeys() {
  const { mutate, isPending } = useHistoryStep();
  const latest = useRef({ mutate, isPending });
  latest.current = { mutate, isPending };
  useEffect(() => {
    const onKey = (e: KeyboardEvent) => {
      const action = historyKey(e);
      if (!action) return;
      e.preventDefault();
      if (!latest.current.isPending) latest.current.mutate(action);
    };
    window.addEventListener("keydown", onKey);
    return () => window.removeEventListener("keydown", onKey);
  }, []);
}
