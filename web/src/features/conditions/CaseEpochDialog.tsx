import { useState } from "react";
import { useCommandOps } from "@/api/ops";
import type { CaseView } from "@/api/types";
import { useT } from "@/i18n";
import { Field } from "@/kit/forms/Field";
import { OpDialog } from "@/ops/OpDialog";
import { Input } from "@/ui/input";
import { caseEpochCommand } from "./case-epoch";

/**
 * 解析ケースの開始時刻（`epoch`）の入力（`fune case set-epoch|unset-epoch` と同じ引数を service に送り、Op にしてから確定する）。
 * 時刻の読み方の検査は service がする（読めない時刻は欄の下に出す）。空にすれば外す。
 */
export function CaseEpochDialog({
  kase,
  onOpenChange,
}: {
  kase: CaseView;
  onOpenChange: (open: boolean) => void;
}) {
  const t = useT().conditions.caseEpoch;
  const [epoch, setEpoch] = useState(kase.epoch ?? "");
  const made = useCommandOps(caseEpochCommand(kase.id, epoch));
  return (
    <OpDialog
      open
      onOpenChange={onOpenChange}
      title={t.title(kase.id)}
      description={t.description}
      ops={made.ops}
      summary={t.summary(kase.id, epoch.trim())}
      confirmLabel={t.confirm}
    >
      <Field label={t.field} htmlFor="case-epoch" hint={t.hint} error={made.error}>
        <Input
          id="case-epoch"
          className="font-mono"
          value={epoch}
          placeholder="2031-05-02T00:00:00Z"
          onChange={(e) => setEpoch(e.target.value)}
        />
      </Field>
    </OpDialog>
  );
}
