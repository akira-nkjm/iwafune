import { useMemo } from "react";
import type { CatalogItemView } from "@/api/types";
import { useT } from "@/i18n";
import { Tree } from "@/kit/tree/Tree";
import { catalogTree, filterTree } from "@/kit/tree/tree";
import { itemLabel } from "./catalog";

/** カタログ ID の階層。同じツリーで検索の一致まで展開する（`PartsTree` と同じ形の Props）。 */
export function CatalogTree({
  all,
  items,
  selected,
  onSelect,
}: {
  /** すべてのカタログ品（ツリーの形を作る） */
  all: CatalogItemView[];
  /** 絞り込んだカタログ品（出すもの。祖先のフォルダーは残す） */
  items: CatalogItemView[];
  selected: string | null;
  onSelect: (id: string) => void;
}) {
  const t = useT();
  const tree = useMemo(() => {
    const ids = new Set(items.map((item) => item.id));
    return filterTree(catalogTree(all), (item) => ids.has(item.id));
  }, [all, items]);
  return (
    <Tree
      nodes={tree.nodes}
      expanded={items.length < all.length ? tree.expanded : undefined}
      selected={selected}
      onSelect={onSelect}
      label={t.catalog.title}
      renderItem={(item, name) => (
        <>
          <span className="min-w-0 flex-1 truncate font-medium font-mono" title={item.id}>
            {name}
          </span>
          <span className="min-w-0 flex-1 truncate text-ink-2" title={item.kind}>
            {item.kind}
          </span>
          <span className="min-w-0 flex-1 truncate text-ink-3" title={itemLabel(item)}>
            {itemLabel(item)}
          </span>
        </>
      )}
    />
  );
}
