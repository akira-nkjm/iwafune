import { useState } from "react";
import type { Op, PartRow } from "@/api/types";
import { useT } from "@/i18n";
import { Field } from "@/kit/Field";
import { TextInput } from "@/kit/TextInput";

import { OpDialog } from "@/ops/OpDialog";
import { useOpCheck } from "@/ops/use-op-check";

/** 名前の変更（RenamePart）。参照している modes・nets・harness も Op が一括で書き換える */
export function RenamePartDialog({
  part,
  open,
  onOpenChange,
  onRenamed,
}: {
  part: PartRow;
  open: boolean;
  onOpenChange: (open: boolean) => void;
  onRenamed: (id: string) => void;
}) {
  const t = useT();
  const tr = t.components.rename;
  const [to, setTo] = useState(part.id);
  const ops: Op[] | null = to && to !== part.id ? [{ op: "rename_part", id: part.id, to }] : null;
  const error = useOpCheck(ops).error;
  return (
    <OpDialog
      open={open}
      onOpenChange={onOpenChange}
      title={tr.title(part.id)}
      description={tr.description}
      ops={ops}
      summary={`${part.id} → ${to}`}
      confirmLabel={tr.confirm}
      onDone={() => onRenamed(to)}
    >
      <Field label={tr.newId} htmlFor="rename-to" error={error}>
        <TextInput
          id="rename-to"
          mono
          value={to}
          invalid={!!error}
          onChange={(e) => setTo(e.target.value)}
        />
      </Field>
    </OpDialog>
  );
}
