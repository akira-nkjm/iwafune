import { useMemo } from "react";
import type { DesignView, PartRow } from "@/api/types";
import { useT } from "@/i18n";
import { SubsystemDot } from "@/kit/SubsystemDot";
import { Tree } from "@/kit/Tree";
import { filterTree, partsTree } from "@/kit/tree";
import { attrOf, nodeLabel } from "@/lib/design";
import { valueText } from "@/quantity/format";

/**
 * 部品のファイル階層と組立品の親子。絞り込みでも祖先を残す。
 * フォルダーの見出しは製品ツリーのノードの表示名（`nodeLabel`。区間は API の `PartRow.dir` のノードの ID）。
 */
export function PartsTree({
  design,
  parts,
  selected,
  onSelect,
}: {
  design: DesignView;
  parts: PartRow[];
  selected: string | null;
  onSelect: (id: string) => void;
}) {
  const t = useT();
  const all = design.parts;
  const folderLabel = (id: string) => nodeLabel(design, id);
  const tree = useMemo(() => {
    const ids = new Set(parts.map((part) => part.id));
    return filterTree(partsTree(all), (part) => ids.has(part.id));
  }, [all, parts]);
  return (
    <Tree
      nodes={tree.nodes}
      expanded={parts.length < all.length ? tree.expanded : undefined}
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
        <SubsystemDot group={part.subsystem} role={part.role} />
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
