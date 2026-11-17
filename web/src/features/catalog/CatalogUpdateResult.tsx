import type { CatalogItemChangeKind, CatalogSyncEntry, CatalogSyncResult } from "@/api/types";
import { useT } from "@/i18n";

/** 品の変わり方の印 */
const MARK: Record<CatalogItemChangeKind, string> = { added: "+", removed: "-", changed: "~" };

/** 値が無い側（追加前・削除後）の表示 */
const NONE = "—";

/**
 * 共有カタログの更新の結果（`POST /catalog/update` の返り値）。commit の動き、追加・削除・変わった値を出す。
 * 何も変わっていなければ「すでに最新です」だけを出す。値の文字列は API が整形したものをそのまま出す。
 */
export function CatalogUpdateResult({ name, result }: { name: string; result: CatalogSyncResult }) {
  const m = useT().catalog.shared;
  const entry: CatalogSyncEntry | undefined = result.catalogs.find((c) => c.name === name);
  if (!result.lock_changed && result.items.length === 0) {
    return (
      <p role="status" className="text-ink-2 text-xs">
        {m.upToDate}
      </p>
    );
  }
  const kindLabel: Record<CatalogItemChangeKind, string> = {
    added: m.changeAdded,
    removed: m.changeRemoved,
    changed: m.changeChanged,
  };
  return (
    <div role="status" className="space-y-1 text-xs">
      {entry && (
        <p className="font-mono text-ink-2">
          {entry.previous && entry.previous !== entry.commit
            ? m.updated(entry.name, entry.previous, entry.commit)
            : m.updatedNew(entry.name, entry.commit)}
        </p>
      )}
      <ul className="max-h-48 space-y-1 overflow-auto">
        {result.items.map((item) => (
          <li key={item.id}>
            <span className="font-mono">
              <span aria-hidden>{MARK[item.change]}</span> {item.id}
            </span>
            <span className="ml-2 text-ink-3">{kindLabel[item.change]}</span>
            {item.values.length > 0 && (
              <ul className="ml-4 font-mono text-ink-2">
                {item.values.map((v) => (
                  <li key={v.key}>
                    {v.key}: {v.before ?? NONE} → {v.after ?? NONE}
                  </li>
                ))}
              </ul>
            )}
          </li>
        ))}
      </ul>
      {result.lock_changed && <p className="text-ink-3">{m.lockRewritten}</p>}
    </div>
  );
}
