import { Pencil, Trash2 } from "lucide-react";
import type { CaseView } from "@/api/types";
import { useT } from "@/i18n";
import { DetailAction } from "@/kit/detail/DetailPane";
import { useDialogs } from "@/kit/detail/use-dialogs";
import { OpDialog } from "@/ops/OpDialog";
import { RenameDialog } from "@/ops/RenameDialog";

/**
 * 解析ケース 1 つの操作（名前の変更 RenameCase・削除 RemoveCase）。参照しているもの（既定のケース・要求の `quantify.cases`）の扱いは
 * `fune case rename|rm` と同じ Op が決める（名前の変更は書き換え、削除は参照が残るなら検査の誤り `ref.case` で拒む。理由はプレビューに出る）。
 * 取り消し・やり直しや変更後に、消えた・名前が変わった旧い ID を問い合わせ続けないのは `api/keys.ts` の `retiredBy`・`retiredBetween`
 */
export function CaseActions({ kase }: { kase: CaseView }) {
  const t = useT().conditions.caseActions;
  const dialogs = useDialogs<"rename" | "remove">();
  return (
    <>
      <div className="flex items-center gap-1.5">
        <DetailAction
          icon={Pencil}
          label={t.rename(kase.id)}
          onClick={() => dialogs.show("rename")}
        />
        <DetailAction
          icon={Trash2}
          label={t.remove(kase.id)}
          onClick={() => dialogs.show("remove")}
        />
      </div>
      {dialogs.current === "rename" && (
        <RenameDialog
          {...dialogs.props}
          from={kase.id}
          title={t.renameTitle(kase.id)}
          description={t.renameDescription}
          label={t.newId}
          confirmLabel={t.renameConfirm}
          opsFor={(to) => [{ op: "rename_case", id: kase.id, to }]}
          summary={(to) => t.renameSummary(kase.id, to)}
        />
      )}
      {dialogs.current === "remove" && (
        <OpDialog
          {...dialogs.props}
          title={t.removeTitle(kase.id)}
          description={t.removeDescription(kase.file)}
          ops={[{ op: "remove_case", id: kase.id }]}
          summary={t.removeSummary(kase.id)}
          confirmLabel={t.removeConfirm}
          destructive
        />
      )}
    </>
  );
}
