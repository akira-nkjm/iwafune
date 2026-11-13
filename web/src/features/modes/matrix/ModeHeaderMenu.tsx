import { MoreHorizontal } from "lucide-react";
import { useCommandText } from "@/api/ops";
import type { ModeHeader } from "@/api/types";
import { useT } from "@/i18n";
import { useDialogs } from "@/kit/detail/use-dialogs";
import { OpDialog } from "@/ops/OpDialog";
import { RenameDialog } from "@/ops/RenameDialog";
import {
  DropdownMenu,
  DropdownMenuContent,
  DropdownMenuItem,
  DropdownMenuTrigger,
} from "@/ui/dropdown-menu";

/** モードの列見出しの操作（表示順・名前の変更・削除） */
export function ModeHeaderMenu({
  mode,
  previous,
  next,
}: {
  mode: ModeHeader;
  previous?: string;
  next?: string;
}) {
  const tm = useT().modes.menu;
  const edit = useCommandText();
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
          <DropdownMenuItem
            disabled={!previous || edit.isPending}
            onSelect={() =>
              previous &&
              edit.mutate({
                command: {
                  command: "mode_move",
                  id: mode.id,
                  before: previous,
                  after: null,
                  first: false,
                  last: false,
                },
                summary: tm.moveLeftSummary(mode.id),
              })
            }
          >
            {tm.moveLeft}
          </DropdownMenuItem>
          <DropdownMenuItem
            disabled={!next || edit.isPending}
            onSelect={() =>
              next &&
              edit.mutate({
                command: {
                  command: "mode_move",
                  id: mode.id,
                  before: null,
                  after: next,
                  first: false,
                  last: false,
                },
                summary: tm.moveRightSummary(mode.id),
              })
            }
          >
            {tm.moveRight}
          </DropdownMenuItem>
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
