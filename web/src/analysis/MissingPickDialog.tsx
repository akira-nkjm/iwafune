import { useState } from "react";
import { useCommandOps, useTextOp } from "@/api/ops";
import type { DesignView, Op } from "@/api/types";
import { useT } from "@/i18n";
import { Field } from "@/kit/forms/Field";
import { NativeSelect } from "@/kit/forms/NativeSelect";
import { OpDialog } from "@/ops/OpDialog";
import { type PickFix, pickEdit } from "./missing-fix";

/**
 * 別の部品を指す欄（マヌーバのスラスタ・回線の送受信機とアンテナ）に、設計の部品から選んで入れるダイアログ。
 * 選べる部品は API が絞って返す（`PickFix.parts`）。マヌーバは `fune maneuver set` と同じ引数、設定の欄は `fune set` と同じ文字列を
 * service で Op にして、確定前に CLI と変わるファイルを見せる。
 */
export function MissingPickDialog({
  fix,
  design,
  onOpenChange,
}: {
  fix: PickFix;
  design: DesignView;
  onOpenChange: (open: boolean) => void;
}) {
  const t = useT().fixes.pick;
  const [part, setPart] = useState("");
  const edit = pickEdit(fix, part, design);
  const text = useTextOp(edit && "text" in edit ? edit.text : null);
  const command = useCommandOps(edit && "command" in edit ? edit.command : null);
  const ops: Op[] | null = text.op ? [text.op] : command.ops;
  return (
    <OpDialog
      open
      onOpenChange={onOpenChange}
      title={t.title(fix.path)}
      description={t.description}
      ops={ops}
      summary={t.summary(fix.path, part)}
      confirmLabel={t.confirm}
    >
      {fix.parts.length === 0 ? (
        <p className="text-ink-3 text-sm">{t.none}</p>
      ) : (
        <Field label={t.part} htmlFor="missing-part" error={text.error ?? command.error}>
          <NativeSelect
            id="missing-part"
            value={part}
            placeholder={t.choose}
            options={fix.parts.map((id) => ({ value: id, label: id }))}
            onChange={(e) => setPart(e.target.value)}
          />
        </Field>
      )}
    </OpDialog>
  );
}
