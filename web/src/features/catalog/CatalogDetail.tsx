import { Pencil, Trash2 } from "lucide-react";
import type { CatalogItemView, DesignView } from "@/api/types";
import { useT } from "@/i18n";
import { DetailAction, DetailHeader, DetailPane } from "@/kit/detail/DetailPane";
import { DetailSection } from "@/kit/detail/DetailSection";
import { useDialogs } from "@/kit/detail/use-dialogs";
import { ChipLink } from "@/kit/selection/Chip";
import { paths } from "@/lib/nav";
import { OpDialog } from "@/ops/OpDialog";
import { CatalogFields } from "./CatalogFields";
import { CatalogValues } from "./CatalogValues";
import { itemLabel } from "./catalog";
import { RenameCatalogDialog } from "./RenameCatalogDialog";

/** カタログ品 1 つの詳細: 基本の欄・値・使っている部品。共有カタログ（`git = …` の出所）の品は読み取り専用 */
export function CatalogDetail({
  design,
  item,
  onSelect,
}: {
  design: DesignView;
  item: CatalogItemView;
  onSelect: (id: string | null) => void;
}) {
  const t = useT();
  const readOnly = !item.editable;
  const users = item.users;
  const dialogs = useDialogs<"rename" | "remove">();
  return (
    <DetailPane
      header={
        <DetailHeader
          title={item.id}
          subtitle={`${itemLabel(item)} · ${item.kind}`}
          actions={
            readOnly ? null : (
              <>
                <DetailAction
                  icon={Pencil}
                  label={t.catalog.rename}
                  onClick={() => dialogs.show("rename")}
                />
                <DetailAction
                  icon={Trash2}
                  label={t.catalog.remove}
                  onClick={() => dialogs.show("remove")}
                />
              </>
            )
          }
        />
      }
    >
      {readOnly && (
        <p className="rounded-control bg-warn-bg px-3 py-2 text-warn-ink text-xs">
          {t.catalog.readOnly(item.scope)}
        </p>
      )}
      <DetailSection title={t.catalog.fields}>
        <CatalogFields key={item.id} item={item} />
      </DetailSection>
      <DetailSection title={t.catalog.values}>
        <CatalogValues defs={design.defs} item={item} />
      </DetailSection>
      <DetailSection title={t.catalog.usedBy}>
        {users.length === 0 ? (
          <p className="px-1 text-ink-3 text-xs">{t.catalog.notUsed}</p>
        ) : (
          <ul className="flex flex-wrap gap-1">
            {users.map((u) => (
              <li key={u}>
                <ChipLink to={paths.component(u)}>{u}</ChipLink>
              </li>
            ))}
          </ul>
        )}
      </DetailSection>
      {dialogs.current === "rename" && (
        <RenameCatalogDialog
          {...dialogs.props}
          item={item}
          project={design.project}
          onDone={onSelect}
        />
      )}
      {dialogs.current === "remove" && (
        <OpDialog
          {...dialogs.props}
          title={t.catalog.removeTitle(item.id)}
          description={t.catalog.removeDesc(users.length)}
          ops={[{ op: "remove_catalog_item", id: item.id }]}
          summary={t.catalog.summaryRemove(item.id)}
          confirmLabel={t.common.remove}
          destructive
          onDone={() => onSelect(null)}
        />
      )}
    </DetailPane>
  );
}
