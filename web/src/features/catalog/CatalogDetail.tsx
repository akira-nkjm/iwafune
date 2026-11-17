import { Pencil, Plus, Trash2 } from "lucide-react";
import { useNavigate } from "react-router";
import type { CatalogItemView, DesignView } from "@/api/types";
import { AddPartDialog } from "@/features/components/AddPartDialog";
import { useT } from "@/i18n";
import { DetailAction, DetailHeader, DetailPane } from "@/kit/detail/DetailPane";
import { DetailSection } from "@/kit/detail/DetailSection";
import { useDialogs } from "@/kit/detail/use-dialogs";
import { RoleIcon } from "@/kit/RoleIcon";
import { ChipLink } from "@/kit/selection/Chip";
import { Pill } from "@/kit/status/Pill";
import { kindDef, kindLabel } from "@/lib/design";
import { paths } from "@/lib/nav";
import { OpDialog } from "@/ops/OpDialog";
import { CatalogFields } from "./CatalogFields";
import { CatalogInherited } from "./CatalogInherited";
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
  const dialogs = useDialogs<"rename" | "remove" | "addPart">();
  const navigate = useNavigate();
  return (
    <DetailPane
      header={
        <DetailHeader
          title={item.id}
          leading={<RoleIcon role={kindDef(design.defs, item.kind)?.role} className="text-ink-3" />}
          trailing={readOnly && <Pill dot={false}>{item.scope}</Pill>}
          subtitle={`${itemLabel(item)} · ${kindLabel(design.defs, item.kind)}`}
          actions={
            <>
              {/* 使うだけなので、共有カタログの品（読み取り専用）にも出す */}
              <DetailAction
                icon={Plus}
                label={t.catalog.addToDesign}
                onClick={() => dialogs.show("addPart")}
              />
              {!readOnly && (
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
              )}
            </>
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
      <CatalogInherited item={item} />
      <DetailSection title={t.catalog.usedByCount(users.length)}>
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
      {dialogs.current === "addPart" && (
        <AddPartDialog
          {...dialogs.props}
          design={design}
          initialCatalog={item.id}
          onDone={(id) => navigate(paths.component(id))}
        />
      )}
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
