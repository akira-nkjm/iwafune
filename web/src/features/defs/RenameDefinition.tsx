import { Pencil } from "lucide-react";
import { useT } from "@/i18n";
import { DetailAction } from "@/kit/detail/DetailPane";
import { useDialogs } from "@/kit/detail/use-dialogs";
import { RenameDialog } from "@/ops/RenameDialog";

/**
 * プロジェクト独自の属性・種別の名前の変更ボタンとダイアログ（RenameAttribute・RenameKind）。
 * 参照の書き換え（種別の属性の一覧・部品とカタログ品の値と `kind`・継承元）は Op が一括でする。
 * 共有カタログの品が使っている定義は書き換えられず、サーバーが拒む理由を欄の下に出す
 */
export function RenameDefinition({ what, id }: { what: "attribute" | "kind"; id: string }) {
  const t = useT();
  const td = t.defs;
  const text = what === "attribute" ? td.attributes : td.kinds;
  const dialogs = useDialogs<"rename">();
  return (
    <>
      <DetailAction icon={Pencil} label={text.rename(id)} onClick={() => dialogs.show("rename")} />
      {dialogs.current === "rename" && (
        <RenameDialog
          {...dialogs.props}
          from={id}
          title={text.rename(id)}
          description={text.renameDescription}
          label={td.newId}
          confirmLabel={td.renameConfirm}
          opsFor={(to) => [
            what === "attribute"
              ? { op: "rename_attribute", id, to }
              : { op: "rename_kind", id, to },
          ]}
          summary={(to) => text.renameSummary(id, to)}
        />
      )}
    </>
  );
}
