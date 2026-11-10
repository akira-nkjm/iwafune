import { type ReactNode, useId, useState } from "react";
import type { Op } from "@/api/types";
import { Field } from "@/kit/Field";
import { TextInput } from "@/kit/TextInput";
import { OpDialog } from "./OpDialog";
import { useOpCheck } from "./use-op-check";

/**
 * ID を変える編集の共通形（部品・カタログ品・モード）。新しい ID の欄と、その ID で作る Op（`opsFor`）を `OpDialog` で
 * 確かめてから送る。ID の規則と重複はサーバー（dry_run）が検査し、欄の下に出す。元の ID のままか空なら確定できない。
 * 確定すると `onDone(新しい ID)`（呼び手が選び直す）
 */
export function RenameDialog({
  from,
  open,
  onOpenChange,
  title,
  description,
  label,
  hint,
  confirmLabel,
  opsFor,
  summary,
  onDone,
}: {
  /** いまの ID（欄の初期値） */
  from: string;
  open: boolean;
  onOpenChange: (open: boolean) => void;
  title: string;
  description?: ReactNode;
  /** 欄の名前（「新しい ID」） */
  label: string;
  hint?: ReactNode;
  confirmLabel?: string;
  /** 新しい ID で作る Op */
  opsFor: (to: string) => Op[];
  /** 送るときの要約（トースト・履歴） */
  summary: (to: string) => string;
  onDone?: (to: string) => void;
}) {
  const id = useId();
  const [to, setTo] = useState(from);
  const ops = to && to !== from ? opsFor(to) : null;
  const error = useOpCheck(open ? ops : null).error;
  return (
    <OpDialog
      open={open}
      onOpenChange={onOpenChange}
      title={title}
      description={description}
      ops={ops}
      summary={summary(to)}
      confirmLabel={confirmLabel}
      onDone={() => onDone?.(to)}
    >
      <Field label={label} htmlFor={id} error={error} hint={hint}>
        <TextInput
          id={id}
          mono
          value={to}
          invalid={!!error}
          onChange={(e) => setTo(e.target.value)}
        />
      </Field>
    </OpDialog>
  );
}
