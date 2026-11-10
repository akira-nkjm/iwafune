import type { CatalogItemView, ProjectInfo } from "@/api/types";
import { useT } from "@/i18n";
import { RenameDialog } from "@/ops/RenameDialog";
import { writableCatalogs } from "./catalog";

/** カタログ品の ID の変更（RenameCatalogItem）。参照している部品（catalog・candidates）も Op が一括で書き換える */
export function RenameCatalogDialog({
  item,
  project,
  open,
  onOpenChange,
  onDone,
}: {
  item: CatalogItemView;
  /** 書き込めるカタログ（ID の頭に付けられる出所）を欄の補足に出す */
  project: ProjectInfo;
  open: boolean;
  onOpenChange: (open: boolean) => void;
  onDone: (id: string) => void;
}) {
  const t = useT();
  const tc = t.catalog;
  return (
    <RenameDialog
      from={item.id}
      open={open}
      onOpenChange={onOpenChange}
      title={tc.renameTitle(item.id)}
      description={tc.renameDesc}
      label={tc.newId}
      hint={tc.idHint(writableCatalogs(project).join(", "))}
      confirmLabel={t.common.change}
      opsFor={(to) => [{ op: "rename_catalog_item", from: item.id, to }]}
      summary={(to) => tc.summaryRename(item.id, to)}
      onDone={onDone}
    />
  );
}
