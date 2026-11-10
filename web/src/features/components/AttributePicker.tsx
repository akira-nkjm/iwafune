import { useMemo, useState } from "react";
import type { AttributeDefView, PartRow } from "@/api/types";
import { useT } from "@/i18n";
import { TextInput } from "@/kit/TextInput";
import { cn } from "@/lib/cn";
import { defName } from "@/lib/design";

/** 標準の属性（組み込み＋プロジェクト独自）を検索して 1 つ選ぶ。値のある属性は外す（状態ごとの属性は残す） */
export function AttributePicker({
  attributes,
  part,
  selected,
  onSelect,
}: {
  attributes: AttributeDefView[];
  /** 値のある属性を候補から外すための部品 */
  part: PartRow;
  /** 選んでいる属性の ID（無ければ空） */
  selected: string;
  onSelect: (id: string) => void;
}) {
  const tp = useT().components.param;
  const [search, setSearch] = useState("");
  const candidates = useMemo(() => {
    const used = new Set(
      Object.entries(part.values)
        .filter(([, v]) => v.origin !== "unset")
        .map(([id]) => id),
    );
    const s = search.trim().toLowerCase();
    const hit = (a: AttributeDefView) =>
      !s || [a.id, a.label, a.label_en ?? ""].some((x) => x.toLowerCase().includes(s));
    return attributes.filter((a) => (!used.has(a.id) || a.by_state) && hit(a));
  }, [attributes, search, part.values]);
  return (
    <div className="grid gap-2">
      <TextInput
        aria-label={tp.searchLabel}
        placeholder={tp.searchPlaceholder}
        value={search}
        onChange={(e) => setSearch(e.target.value)}
      />
      <ul className="max-h-48 overflow-y-auto rounded-control border border-line">
        {candidates.length === 0 && <li className="px-3 py-3 text-ink-3 text-xs">{tp.noMatch}</li>}
        {candidates.map((a) => (
          <li key={a.id}>
            <button
              type="button"
              onClick={() => onSelect(a.id)}
              className={cn(
                "grid w-full grid-cols-[160px_1fr_auto] items-baseline gap-2 border-line-soft border-b px-3 py-1.5 text-left text-xs hover:bg-panel-2",
                selected === a.id && "bg-accent-tint",
              )}
            >
              <span className="truncate font-mono">{a.id}</span>
              <span className="truncate text-ink-2">{defName(a)}</span>
              <span className="text-ink-3">
                {a.dimension ?? a.value_type}
                {a.by_state ? tp.byState : ""}
                {a.units.length > 0 ? ` · ${a.units.join(", ")}` : ""}
              </span>
            </button>
          </li>
        ))}
      </ul>
    </div>
  );
}
