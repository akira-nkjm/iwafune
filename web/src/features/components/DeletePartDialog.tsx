import type { ModeTable, PartRow } from "@/api/types";
import { useT } from "@/i18n";
import { OpDialog } from "@/ops/OpDialog";

/** 部品の削除（RemovePart）。モードの行やポートの参照がどうなるかは dry_run の CLI と変更ファイルで見せる */
export function DeletePartDialog({
  table,
  part,
  open,
  onOpenChange,
  onDone,
}: {
  /** モード表（部品を書いているモードの数を説明に出す） */
  table: ModeTable;
  part: PartRow;
  open: boolean;
  onOpenChange: (open: boolean) => void;
  /** 消したとき */
  onDone: () => void;
}) {
  const t = useT().components.remove;
  const inModes = Object.keys(table.rows.find((r) => r.part === part.id)?.cells ?? {}).length;
  const connected = part.ports.filter((p) => p.net).length;
  return (
    <OpDialog
      open={open}
      onOpenChange={onOpenChange}
      title={t.title(part.id)}
      description={t.description(part.file, inModes, connected)}
      ops={[{ op: "remove_part", id: part.id }]}
      summary={t.summary(part.id)}
      confirmLabel={t.confirm}
      destructive
      onDone={onDone}
    />
  );
}
