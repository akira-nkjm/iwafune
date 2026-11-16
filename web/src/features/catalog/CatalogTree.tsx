import { useMemo } from "react";
import { useEnumLabel } from "@/api/labels";
import type { CatalogItemView, DesignView } from "@/api/types";
import { useT } from "@/i18n";
import { Tree } from "@/kit/tree/Tree";
import { type CatalogGrouping, catalogTree, filterTree } from "@/kit/tree/tree-nodes";
import { kindLabel } from "@/lib/design";
import { itemLabel } from "./catalog";

/** カタログ品を `grouping` の分け方でまとめたツリー。同じツリーで検索の一致まで展開する（`PartsTree` と同じ形の Props）。 */
export function CatalogTree({
  design,
  all,
  items,
  selected,
  onSelect,
  grouping,
}: {
  design: DesignView;
  /** すべてのカタログ品（ツリーの形を作る） */
  all: CatalogItemView[];
  /** 絞り込んだカタログ品（出すもの。祖先のフォルダーは残す） */
  items: CatalogItemView[];
  selected: string | null;
  onSelect: (id: string) => void;
  grouping: CatalogGrouping;
}) {
  const t = useT().catalog;
  const enumLabel = useEnumLabel();
  const tree = useMemo(() => {
    const ids = new Set(items.map((item) => item.id));
    const labels = {
      unset: t.notSet,
      kind: (id: string) => kindLabel(design.defs, id),
      maturity: (value: string) => enumLabel("maturity", value),
    };
    return filterTree(catalogTree(all, grouping, design.defs.kinds, labels), (item) =>
      ids.has(item.id),
    );
  }, [all, items, grouping, design.defs, t.notSet, enumLabel]);
  return (
    <Tree
      nodes={tree.nodes}
      expanded={items.length < all.length ? tree.expanded : undefined}
      selected={selected}
      onSelect={onSelect}
      label={t.title}
      renderItem={(item, name) => (
        <>
          <span className="min-w-0 flex-1 truncate font-medium font-mono" title={item.id}>
            {name}
          </span>
          <span className="min-w-0 flex-1 truncate text-ink-2" title={item.kind}>
            {kindLabel(design.defs, item.kind)}
          </span>
          <span className="min-w-0 flex-1 truncate text-ink-3" title={itemLabel(item)}>
            {itemLabel(item)}
          </span>
        </>
      )}
    />
  );
}
