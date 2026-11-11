import { useState } from "react";
import type { ModeTable, Op } from "@/api/types";
import { useT } from "@/i18n";
import { Field } from "@/kit/forms/Field";
import { NativeSelect } from "@/kit/forms/NativeSelect";
import { TextInput } from "@/kit/forms/TextInput";
import { idWithName } from "@/lib/design";

import { OpDialog } from "@/ops/OpDialog";
import { useOpCheck } from "@/ops/use-op-check";

/** モードの追加（CreateMode）。既存のモードを写して始められる */
export function AddModeDialog({
  table,
  open,
  onOpenChange,
}: {
  table: ModeTable;
  open: boolean;
  onOpenChange: (open: boolean) => void;
}) {
  const t = useT();
  const ta = t.modes.addDialog;
  const [id, setId] = useState("");
  const [label, setLabel] = useState("");
  const [copy, setCopy] = useState(table.modes[0]?.id ?? "");
  const ops: Op[] | null = id
    ? [{ op: "create_mode", id, label: label || id, description: null, copy_from: copy || null }]
    : null;
  const error = useOpCheck(ops).error;
  return (
    <OpDialog
      open={open}
      onOpenChange={onOpenChange}
      title={ta.title}
      description={ta.description}
      ops={ops}
      summary={ta.summary(id)}
      confirmLabel={ta.confirm}
    >
      <div className="grid grid-cols-2 gap-3">
        <Field label="ID" htmlFor="mode-id" error={error}>
          <TextInput
            id="mode-id"
            mono
            value={id}
            invalid={!!error}
            placeholder="DOWNLINK"
            onChange={(e) => setId(e.target.value)}
          />
        </Field>
        <Field label={ta.label} htmlFor="mode-label">
          <TextInput
            id="mode-label"
            value={label}
            placeholder={ta.labelPlaceholder}
            onChange={(e) => setLabel(e.target.value)}
          />
        </Field>
      </div>
      <Field label={ta.copyFrom} htmlFor="mode-copy">
        <NativeSelect
          id="mode-copy"
          value={copy}
          placeholder={ta.empty}
          onChange={(e) => setCopy(e.target.value)}
          options={table.modes.map((m) => ({ value: m.id, label: idWithName(m) }))}
        />
      </Field>
    </OpDialog>
  );
}
