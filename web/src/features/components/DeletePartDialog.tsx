import type { DesignView, PartRow } from "@/api/types";
import { useT } from "@/i18n";
import { OpDialog } from "@/ops/OpDialog";

/** 部品の削除（RemovePart）。モードの行やポートの参照がどうなるかは dry_run の CLI と変更ファイルで見せる */
export function DeletePartDialog({
  design,
  part,
  open,
  onOpenChange,
  onDeleted,
}: {
  design: DesignView;
  part: PartRow;
  open: boolean;
  onOpenChange: (open: boolean) => void;
  onDeleted: () => void;
}) {
  const t = useT().components.remove;
  const inModes = Object.keys(
    design.modes.rows.find((r) => r.part === part.id)?.cells ?? {},
  ).length;
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
      onDone={onDeleted}
    />
  );
}
