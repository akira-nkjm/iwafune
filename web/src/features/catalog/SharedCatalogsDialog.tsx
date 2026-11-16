import { Plus, Trash2 } from "lucide-react";
import { useState } from "react";
import { useFetchCatalogs } from "@/api/catalog";
import { useEnumLabel } from "@/api/labels";
import type { CatalogSourceView, SharedCatalogState, SharedCatalogStatus } from "@/api/types";
import { useT } from "@/i18n";
import { Pill, type PillTone } from "@/kit/status/Pill";
import { OpDialog } from "@/ops/OpDialog";
import { Button } from "@/ui/button";
import { Dialog, DialogContent, DialogDescription, DialogHeader, DialogTitle } from "@/ui/dialog";
import { AddSharedCatalogDialog } from "./AddSharedCatalogDialog";

/** 状態の色（読み込めていれば pos、取得が要れば warn） */
const TONE: Record<SharedCatalogState, PillTone> = {
  ready: "pos",
  not_locked: "warn",
  lock_mismatch: "warn",
  not_fetched: "warn",
};

/** 共有カタログの状態の一覧（共有でないカタログは除く） */
export function sharedOf(catalogs: CatalogSourceView[]): SharedCatalogStatus[] {
  return catalogs.flatMap((c) => (c.shared ? [c.shared] : []));
}

/** 取得が要る（lock から読み込めていない）共有カタログの数。ページの見出しのボタンの印に使う */
export function neededCount(catalogs: CatalogSourceView[]): number {
  return sharedOf(catalogs).filter((s) => s.state !== "ready").length;
}

/**
 * 共有カタログ（`[[catalog]] git = …`）の lock の状態（commit・取得の要否）と取得のボタン、足す・外すダイアログ。
 * ページの見出しのボタンから開く。状態の判定はサービス（`CatalogSourceView.shared`）、名前の規則や外せるかの判定もサービス（dry_run）で、ここは表示するだけ。
 * 足す・外すは、このダイアログの上に重ねて開く（Radix のダイアログは重ねて使える。閉じるとここに戻る）。
 */
export function SharedCatalogsDialog({
  catalogs,
  open,
  onOpenChange,
}: {
  catalogs: CatalogSourceView[];
  open: boolean;
  onOpenChange: (open: boolean) => void;
}) {
  const t = useT();
  const m = t.catalog.shared;
  const label = useEnumLabel();
  const fetchCatalogs = useFetchCatalogs();
  const [adding, setAdding] = useState(false);
  /** 外す確認を開いている共有カタログの名前 */
  const [removing, setRemoving] = useState<string | null>(null);
  const shared = sharedOf(catalogs);
  const needed = neededCount(catalogs) > 0;
  return (
    <>
      <Dialog open={open} onOpenChange={onOpenChange}>
        <DialogContent className="sm:max-w-2xl">
          <DialogHeader>
            <DialogTitle>{m.title}</DialogTitle>
            <DialogDescription>
              {shared.length === 0 ? m.none : needed ? m.needed : m.ready}
            </DialogDescription>
          </DialogHeader>
          {shared.length > 0 && (
            <ul className="space-y-1.5">
              {shared.map((s) => (
                <li key={s.name} className="flex flex-wrap items-center gap-x-3 gap-y-1 text-xs">
                  <span className="font-mono font-semibold">{s.name}</span>
                  <Pill tone={TONE[s.state]}>{label("shared_catalog_state", s.state)}</Pill>
                  <span className="max-w-[320px] truncate font-mono text-ink-3" title={s.git}>
                    {s.git}
                  </span>
                  {s.rev && <span className="font-mono text-ink-2">{m.rev(s.rev)}</span>}
                  <span
                    className="max-w-[120px] truncate font-mono text-ink-3"
                    title={s.commit ?? undefined}
                  >
                    {s.commit ?? m.noCommit}
                  </span>
                  {s.state !== "ready" && <span className="text-ink-3">{m.hint[s.state]}</span>}
                  <Button
                    variant="ghost"
                    size="icon-xs"
                    className="ml-auto"
                    aria-label={m.remove(s.name)}
                    title={m.remove(s.name)}
                    onClick={() => setRemoving(s.name)}
                  >
                    <Trash2 />
                  </Button>
                </li>
              ))}
            </ul>
          )}
          {fetchCatalogs.error && (
            <p role="alert" className="mt-2 text-neg-ink text-xs">
              {fetchCatalogs.error.message}
            </p>
          )}
          <div className="flex items-center gap-1.5">
            <Button variant="outline" size="sm" onClick={() => setAdding(true)}>
              <Plus />
              {m.add}
            </Button>
            {shared.length > 0 && (
              <Button
                variant="outline"
                size="sm"
                disabled={fetchCatalogs.isPending}
                onClick={() => fetchCatalogs.mutate({ locked: false })}
              >
                {fetchCatalogs.isPending ? m.fetching : m.fetch}
              </Button>
            )}
          </div>
        </DialogContent>
      </Dialog>
      {adding && <AddSharedCatalogDialog open onOpenChange={setAdding} />}
      {removing !== null && (
        <OpDialog
          open
          onOpenChange={(open) => {
            if (!open) setRemoving(null);
          }}
          title={m.removeTitle(removing)}
          description={m.removeDesc}
          ops={[{ op: "remove_catalog_source", name: removing }]}
          summary={m.summaryRemove(removing)}
          confirmLabel={t.common.remove}
          destructive
        />
      )}
    </>
  );
}
