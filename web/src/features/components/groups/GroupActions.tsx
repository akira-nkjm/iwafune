import { ChevronsDownUp, FolderMinus, Pencil } from "lucide-react";
import type { TreeNode } from "@/api/types";
import { useT } from "@/i18n";
import { DetailAction } from "@/kit/detail/DetailPane";
import { useDialogs } from "@/kit/detail/use-dialogs";
import { OpDialog } from "@/ops/OpDialog";
import { RenameDialog } from "@/ops/RenameDialog";
import { ungroupOps, ungroupParts } from "./group-ops";

/**
 * グループの枠の見出しの操作: たたむ（1 枚のカードに戻す）、名前の変更（RenameGroup）と、グループを解いて部品を親へ戻す（部品ごとの MovePart）。
 * 解けるのは入れ子のグループで、中が部品だけのとき（`ungroupParts`）。押しても枠の外の選択解除は起きない。
 */
export function GroupActions({
  group,
  nodes,
  onFold,
}: {
  group: TreeNode;
  nodes: ReadonlyMap<string, TreeNode>;
  /** 渡すと「たたむ」のボタンを出す（開いた枠を 1 枚のカードに戻す） */
  onFold?: () => void;
}) {
  const tg = useT().components.group;
  const dialogs = useDialogs<"rename" | "ungroup">();
  const parts = ungroupParts(group, nodes);
  return (
    // biome-ignore lint/a11y/noStaticElementInteractions: 枠の外の「選択解除」へ押下を伝えないための囲み
    // biome-ignore lint/a11y/useKeyWithClickEvents: 同上（中のボタンがキーで操作できる）
    <span className="ml-auto flex gap-1" onClick={(e) => e.stopPropagation()}>
      {onFold && <DetailAction icon={ChevronsDownUp} label={tg.foldAction} onClick={onFold} />}
      <DetailAction icon={Pencil} label={tg.renameAction} onClick={() => dialogs.show("rename")} />
      {parts && (
        <DetailAction
          icon={FolderMinus}
          label={tg.ungroupAction}
          onClick={() => dialogs.show("ungroup")}
        />
      )}
      {dialogs.current === "rename" && (
        <RenameDialog
          {...dialogs.props}
          from={group.id}
          title={tg.renameTitle(group.id)}
          description={tg.renameDescription}
          label={tg.idLabel}
          confirmLabel={tg.renameConfirm}
          opsFor={(to) => [{ op: "rename_group", id: group.id, to }]}
          summary={(to) => `${group.id} → ${to}`}
        />
      )}
      {dialogs.current === "ungroup" && parts && group.parent && (
        <OpDialog
          {...dialogs.props}
          title={tg.ungroupTitle(group.id)}
          description={tg.ungroupDescription(group.parent)}
          ops={ungroupOps(group.parent, parts)}
          summary={tg.ungroupSummary(group.id)}
          confirmLabel={tg.ungroupConfirm}
        />
      )}
    </span>
  );
}
