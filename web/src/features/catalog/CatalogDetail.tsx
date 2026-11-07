import { Pencil, Trash2 } from "lucide-react";
import { useState } from "react";
import { Link } from "react-router";
import type { CatalogItemView, DesignView, Op } from "@/api/types";
import { DetailSection } from "@/features/components/DetailSection";
import { useT } from "@/i18n";
import { Field } from "@/kit/Field";
import { TextInput } from "@/kit/TextInput";
import { OpDialog } from "@/ops/OpDialog";
import { useOpCheck } from "@/ops/use-op-check";
import { Button } from "@/ui/button";
import { CatalogFields } from "./CatalogFields";
import { CatalogValues } from "./CatalogValues";
import { itemLabel } from "./catalog";

/** カタログ品 1 つの詳細: 基本の欄・値・使っている部品。共有カタログ（lab/…）は読み取り専用 */
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
  const [dialog, setDialog] = useState<"rename" | "remove" | null>(null);
  const [to, setTo] = useState(item.id);
  const renameOps: Op[] | null =
    to && to !== item.id ? [{ op: "rename_catalog_item", from: item.id, to }] : null;
  // ID の規則と重複はサーバー（dry_run）が検査する
  const renameError = useOpCheck(dialog === "rename" ? renameOps : null).error;
  return (
    <div className="flex min-h-0 flex-col">
      <div className="flex items-start justify-between gap-2 border-line border-b px-4 py-3">
        <div className="min-w-0">
          <h2 className="truncate font-mono font-semibold text-[15px]">{item.id}</h2>
          <p className="mt-0.5 truncate text-ink-3 text-xs">
            {itemLabel(item)} · {item.kind}
          </p>
        </div>
        {!readOnly && (
          <div className="flex shrink-0 gap-1">
            <Button
              size="icon-sm"
              variant="outline"
              title={t.catalog.rename}
              onClick={() => setDialog("rename")}
            >
              <Pencil size={13} />
            </Button>
            <Button
              size="icon-sm"
              variant="outline"
              title={t.catalog.remove}
              onClick={() => setDialog("remove")}
            >
              <Trash2 size={13} />
            </Button>
          </div>
        )}
      </div>
      <div className="min-h-0 flex-1 space-y-4 overflow-y-auto px-4 py-3">
        {readOnly && (
          <p className="rounded-control bg-warn-bg px-3 py-2 text-warn-ink text-xs">
            {t.catalog.readOnly}
          </p>
        )}
        <DetailSection title={t.catalog.fields}>
          <CatalogFields key={item.id} item={item} />
        </DetailSection>
        <DetailSection title={t.catalog.values}>
          <CatalogValues design={design} item={item} />
        </DetailSection>
        <DetailSection title={t.catalog.usedBy}>
          {users.length === 0 ? (
            <p className="px-1 text-ink-3 text-xs">{t.catalog.notUsed}</p>
          ) : (
            <ul className="flex flex-wrap gap-1">
              {users.map((u) => (
                <li key={u}>
                  <Link
                    to={`/design/components/${u}`}
                    className="rounded-chip border border-line px-1.5 py-0.5 font-mono text-[11px] hover:bg-panel-2"
                  >
                    {u}
                  </Link>
                </li>
              ))}
            </ul>
          )}
        </DetailSection>
      </div>
      {dialog === "rename" && (
        <OpDialog
          open
          onOpenChange={(o) => !o && setDialog(null)}
          title={t.catalog.renameTitle(item.id)}
          description={t.catalog.renameDesc}
          ops={renameOps}
          summary={t.catalog.summaryRename(item.id, to)}
          confirmLabel={t.common.change}
          onDone={() => onSelect(to)}
        >
          <Field
            label={t.catalog.newId}
            htmlFor="cat-rename"
            error={renameError}
            hint={t.catalog.idLocal}
          >
            <TextInput
              id="cat-rename"
              mono
              value={to}
              invalid={!!renameError}
              onChange={(e) => setTo(e.target.value)}
            />
          </Field>
        </OpDialog>
      )}
      {dialog === "remove" && (
        <OpDialog
          open
          onOpenChange={(o) => !o && setDialog(null)}
          title={t.catalog.removeTitle(item.id)}
          description={t.catalog.removeDesc(users.length)}
          ops={[{ op: "remove_catalog_item", id: item.id }]}
          summary={t.catalog.summaryRemove(item.id)}
          confirmLabel={t.common.remove}
          destructive
          onDone={() => onSelect(null)}
        />
      )}
    </div>
  );
}
