import type { PartRow } from "@/api/types";
import { useT } from "@/i18n";
import { RenameDialog } from "@/ops/RenameDialog";

/** 部品の名前の変更（RenamePart）。参照している modes・nets・harness も Op が一括で書き換える */
export function RenamePartDialog({
  part,
  open,
  onOpenChange,
  onDone,
}: {
  part: PartRow;
  open: boolean;
  onOpenChange: (open: boolean) => void;
  onDone: (id: string) => void;
}) {
  const tr = useT().components.rename;
  return (
    <RenameDialog
      from={part.id}
      open={open}
      onOpenChange={onOpenChange}
      title={tr.title(part.id)}
      description={tr.description}
      label={tr.newId}
      confirmLabel={tr.confirm}
      opsFor={(to) => [{ op: "rename_part", id: part.id, to }]}
      summary={(to) => `${part.id} → ${to}`}
      onDone={onDone}
    />
  );
}
