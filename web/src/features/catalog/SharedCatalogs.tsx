import { useFetchCatalogs } from "@/api/catalog";
import { useEnumLabel } from "@/api/labels";
import type { CatalogSourceView, SharedCatalogState } from "@/api/types";
import { useT } from "@/i18n";
import { Panel } from "@/kit/panel/Panel";
import { PanelHeader } from "@/kit/panel/PanelHeader";
import { Pill, type PillTone } from "@/kit/status/Pill";
import { Button } from "@/ui/button";

/** 状態の色（読み込めていれば pos、取得が要れば warn） */
const TONE: Record<SharedCatalogState, PillTone> = {
  ready: "pos",
  not_locked: "warn",
  lock_mismatch: "warn",
  not_fetched: "warn",
};

/**
 * 共有カタログ（`[[catalog]] git = …`）の lock の状態（commit・取得の要否）と取得のボタン。
 * 状態の判定はサービス（`CatalogSourceView.shared`）、ここは表示するだけ。共有カタログが無ければ何も出さない。
 */
export function SharedCatalogs({ catalogs }: { catalogs: CatalogSourceView[] }) {
  const m = useT().catalog.shared;
  const label = useEnumLabel();
  const fetchCatalogs = useFetchCatalogs();
  const shared = catalogs.flatMap((c) => (c.shared ? [c.shared] : []));
  if (!shared.length) return null;
  const needed = shared.some((s) => s.state !== "ready");
  return (
    <Panel className="mb-3">
      <PanelHeader
        title={m.title}
        caption={needed ? m.needed : m.ready}
        right={
          <Button
            variant="outline"
            size="xs"
            disabled={fetchCatalogs.isPending}
            onClick={() => fetchCatalogs.mutate({ locked: false })}
          >
            {fetchCatalogs.isPending ? m.fetching : m.fetch}
          </Button>
        }
      />
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
          </li>
        ))}
      </ul>
      {fetchCatalogs.error && (
        <p role="alert" className="mt-2 text-neg-ink text-xs">
          {fetchCatalogs.error.message}
        </p>
      )}
    </Panel>
  );
}
