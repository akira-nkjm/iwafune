import { useState } from "react";
import { useCommandOps, useTextOp } from "@/api/ops";
import type { DesignView, Op } from "@/api/types";
import { useT } from "@/i18n";
import { choiceLabel } from "@/kit/forms/choice-label";
import { Field } from "@/kit/forms/Field";
import { NativeSelect } from "@/kit/forms/NativeSelect";
import { OpDialog } from "@/ops/OpDialog";
import { type PickFix, pickEdit } from "./missing-fix";

/**
 * 別のもの（マヌーバのスラスタ・回線の送受信機とアンテナ・打ち上げ機・ディスペンサー・下りの回線）を指す欄に、設計にあるものから
 * 選んで入れるダイアログ。選べるもの（`PickFix.choices`）は API が絞って返す。マヌーバは `fune maneuver set` と同じ引数、
 * 設定の欄は `fune set` と同じ文字列を service で Op にして、確定前に CLI と変わるファイルを見せる。
 * 選べるものが無いときは、それを足す CLI のコマンドを出す。
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
  const [chosen, setChosen] = useState("");
  const edit = pickEdit(fix, chosen, design);
  const text = useTextOp(edit && "text" in edit ? edit.text : null);
  const command = useCommandOps(edit && "command" in edit ? edit.command : null);
  const ops: Op[] | null = text.op ? [text.op] : command.ops;
  const kinds = fix.kinds.join(", ");
  return (
    <OpDialog
      open
      onOpenChange={onOpenChange}
      title={t.title(fix.path)}
      description={t.prompt[fix.what](kinds)}
      ops={ops}
      summary={t.summary(fix.path, chosen)}
      confirmLabel={t.confirm}
    >
      {fix.choices.length === 0 ? (
        <div className="grid gap-1 text-ink-3 text-sm">
          <p>{t.none[fix.what](kinds)}</p>
          {fix.add_command && (
            <code className="font-mono text-ink-2 text-xs">{fix.add_command}</code>
          )}
        </div>
      ) : (
        <Field label={t.label[fix.what]} htmlFor="missing-pick" error={text.error ?? command.error}>
          <NativeSelect
            id="missing-pick"
            value={chosen}
            placeholder={t.choose}
            options={fix.choices.map((c) => ({ value: c.value, label: choiceLabel(c) }))}
            onChange={(e) => setChosen(e.target.value)}
          />
        </Field>
      )}
    </OpDialog>
  );
}
