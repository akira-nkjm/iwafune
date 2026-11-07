import { MoreHorizontal } from "lucide-react";
import { useState } from "react";
import type { ModeHeader, Op } from "@/api/types";
import { useT } from "@/i18n";
import { Field } from "@/kit/Field";
import { TextInput } from "@/kit/TextInput";

import { OpDialog } from "@/ops/OpDialog";
import { useOpCheck } from "@/ops/use-op-check";
import {
  DropdownMenu,
  DropdownMenuContent,
  DropdownMenuItem,
  DropdownMenuTrigger,
} from "@/ui/dropdown-menu";

/** モードの列見出しの操作（名前の変更・削除） */
export function ModeHeaderMenu({ mode }: { mode: ModeHeader }) {
  const t = useT();
  const tm = t.modes.menu;
  const [dialog, setDialog] = useState<"rename" | "remove" | null>(null);
  const [to, setTo] = useState(mode.id);
  const renameOps: Op[] | null =
    to && to !== mode.id ? [{ op: "rename_mode", id: mode.id, to }] : null;
  const error = useOpCheck(dialog === "rename" ? renameOps : null).error;
  return (
    <>
      <DropdownMenu>
        <DropdownMenuTrigger asChild>
          <button
            type="button"
            aria-label={tm.actions(mode.id)}
            className="rounded p-0.5 text-ink-4 hover:bg-panel hover:text-ink"
          >
            <MoreHorizontal size={13} />
          </button>
        </DropdownMenuTrigger>
        <DropdownMenuContent align="end">
          <DropdownMenuItem onSelect={() => setDialog("rename")}>{tm.rename}</DropdownMenuItem>
          <DropdownMenuItem variant="destructive" onSelect={() => setDialog("remove")}>
            {tm.remove}
          </DropdownMenuItem>
        </DropdownMenuContent>
      </DropdownMenu>
      {dialog === "rename" && (
        <OpDialog
          open
          onOpenChange={(o) => !o && setDialog(null)}
          title={tm.renameTitle(mode.id)}
          description={tm.renameDescription}
          ops={renameOps}
          summary={tm.renameSummary(mode.id, to)}
          confirmLabel={tm.renameConfirm}
        >
          <Field label={tm.newId} htmlFor="mode-rename" error={error}>
            <TextInput
              id="mode-rename"
              mono
              value={to}
              invalid={!!error}
              onChange={(e) => setTo(e.target.value)}
            />
          </Field>
        </OpDialog>
      )}
      {dialog === "remove" && (
        <OpDialog
          open
          onOpenChange={(o) => !o && setDialog(null)}
          title={tm.removeTitle(mode.id)}
          description={tm.removeDescription(mode.file)}
          ops={[{ op: "remove_mode", id: mode.id }]}
          summary={tm.removeSummary(mode.id)}
          confirmLabel={tm.removeConfirm}
          destructive
        />
      )}
    </>
  );
}
