import { useMemo } from "react";
import type { PartRow } from "@/api/types";
import { useT } from "@/i18n";
import { SubsystemDot } from "@/kit/SubsystemDot";
import { Tree } from "@/kit/Tree";
import { filterTree, partsTree } from "@/kit/tree";
import { attrOf } from "@/lib/design";
import { valueText } from "@/quantity/format";

/**
 * 部品のファイル階層と組立品の親子。絞り込みでも祖先を残す（`CatalogTree` と同じ形の Props）。
 * フォルダーの見出しは呼び手が渡す（製品ツリーのノードの表示名。区間は API の `PartRow.dir` のノードの ID）。
 */
export function PartsTree({
  all,
  items,
  selected,
  onSelect,
  folderLabel,
}: {
  /** すべての部品（ツリーの形を作る） */
  all: PartRow[];
  /** 絞り込んだ部品（出すもの。祖先のフォルダーは残す） */
  items: PartRow[];
  selected: string | null;
  onSelect: (id: string) => void;
  /** フォルダー（製品ツリーのノードの ID）の見出し（`lib/design.ts` の `nodeLabel`） */
  folderLabel: (id: string) => string;
}) {
  const t = useT();
  const tree = useMemo(() => {
    const ids = new Set(items.map((part) => part.id));
    return filterTree(partsTree(all), (part) => ids.has(part.id));
  }, [all, items]);
  return (
    <Tree
      nodes={tree.nodes}
      expanded={items.length < all.length ? tree.expanded : undefined}
      label={t.components.title}
      selected={selected}
      onSelect={onSelect}
      folderLabel={folderLabel}
      renderItem={(part) => <PartTreeRow part={part} />}
    />
  );
}

function PartTreeRow({ part }: { part: PartRow }) {
  const t = useT().components.table;
  const mass = attrOf(part, "mass");
  return (
    <>
      <span
        className="flex min-w-0 flex-1 items-center gap-1.5"
        title={[part.id, part.kind, part.file].filter((s) => s !== null).join(" · ")}
      >
        <SubsystemDot group={part.subsystem} />
        <span className="max-w-20 shrink-0 truncate font-medium font-mono">{part.id}</span>
        <span className="min-w-0 flex-1 truncate text-ink-2">{part.kind}</span>
      </span>
      <span className="shrink-0 text-right tabular-nums" title={t.count}>
        {part.count}
      </span>
      <span className="w-14 shrink-0 text-right tabular-nums" title={t.mass}>
        {mass ? valueText(mass) : "—"}
      </span>
    </>
  );
}
