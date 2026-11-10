import { MoreHorizontal } from "lucide-react";
import type { ModeHeader } from "@/api/types";
import { useT } from "@/i18n";
import { useDialogs } from "@/kit/use-dialogs";
import { OpDialog } from "@/ops/OpDialog";
import { RenameDialog } from "@/ops/RenameDialog";
import {
  DropdownMenu,
  DropdownMenuContent,
  DropdownMenuItem,
  DropdownMenuTrigger,
} from "@/ui/dropdown-menu";

/** モードの列見出しの操作（名前の変更・削除） */
export function ModeHeaderMenu({ mode }: { mode: ModeHeader }) {
  const tm = useT().modes.menu;
  const dialogs = useDialogs<"rename" | "remove">();
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
          <DropdownMenuItem onSelect={() => dialogs.show("rename")}>{tm.rename}</DropdownMenuItem>
          <DropdownMenuItem variant="destructive" onSelect={() => dialogs.show("remove")}>
            {tm.remove}
          </DropdownMenuItem>
        </DropdownMenuContent>
      </DropdownMenu>
      {dialogs.current === "rename" && (
        <RenameDialog
          {...dialogs.props}
          from={mode.id}
          title={tm.renameTitle(mode.id)}
          description={tm.renameDescription}
          label={tm.newId}
          confirmLabel={tm.renameConfirm}
          opsFor={(to) => [{ op: "rename_mode", id: mode.id, to }]}
          summary={(to) => tm.renameSummary(mode.id, to)}
        />
      )}
      {dialogs.current === "remove" && (
        <OpDialog
          {...dialogs.props}
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
