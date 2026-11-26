import { Redo2, Undo2 } from "lucide-react";
import { useHistoryStep, useHistoryView } from "@/api/history";
import { useT } from "@/i18n";
import { Button } from "@/ui/button";

/**
 * 元に戻す・やり直すのボタン対（⌘Z / Ctrl+Z・⇧⌘Z / Ctrl+Y と同じ `POST /undo`・`POST /redo`）。
 * 押せるかは API の履歴（`can_undo`・`can_redo`）で決める。変更履歴のポップオーバーと系統図の見出しが使う。
 */
export function UndoRedoButtons({ labelled = false }: { labelled?: boolean }) {
  const t = useT().common;
  const history = useHistoryView();
  const step = useHistoryStep();
  return (
    <>
      <Button
        size={labelled ? "xs" : "icon-sm"}
        variant="outline"
        title={t.undoTitle}
        aria-label={t.undo}
        disabled={!history?.can_undo || step.isPending}
        onClick={() => step.mutate("undo")}
      >
        <Undo2 size={13} />
        {labelled && t.undo}
      </Button>
      <Button
        size={labelled ? "xs" : "icon-sm"}
        variant="outline"
        title={t.redoTitle}
        aria-label={t.redo}
        disabled={!history?.can_redo || step.isPending}
        onClick={() => step.mutate("redo")}
      >
        <Redo2 size={13} />
        {labelled && t.redo}
      </Button>
    </>
  );
}
