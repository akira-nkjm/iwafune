import type { DesignView } from "@/api/types";
import { PartDetail } from "@/features/components/PartDetail";
import { getT } from "@/i18n";
import { Empty } from "@/kit/Empty";
import { cn } from "@/lib/cn";
import type { DiagramModel } from "./model";
import { NetInspector } from "./NetInspector";

export type DiagramSelection = {
  kind: "card" | "edge" | "net";
  id: string;
  member?: string;
} | null;

/** 右のインスペクタ: カードなら部品の詳細（コンポーネントのページと同じもの）、辺ならネット（まとめた辺は選んで切り替え） */
export function DiagramInspector({
  design,
  model,
  selected,
  onSelect,
}: {
  design: DesignView;
  model: DiagramModel;
  selected: DiagramSelection;
  onSelect: (s: DiagramSelection) => void;
}) {
  const t = getT().diagram.inspector;
  if (!selected) return <Empty>{t.empty}</Empty>;
  if (selected.kind === "card") {
    const card = model.cards.find((c) => c.id === selected.id);
    if (!card) return <Empty>{t.noCard}</Empty>;
    const member = selected.member ?? card.members[0];
    const row = design.parts.find((p) => p.id === member);
    return (
      <div className="flex h-full flex-col">
        {card.members.length > 1 && (
          <div className="flex flex-wrap gap-1 border-line border-b px-4 py-2">
            {card.members.map((m) => (
              <button
                key={m}
                type="button"
                onClick={() => onSelect({ ...selected, member: m })}
                className={cn(
                  "rounded-chip border px-1.5 py-0.5 font-mono text-[11px]",
                  m === member
                    ? "border-accent bg-accent-tint text-accent-ink"
                    : "border-line hover:bg-panel-2",
                )}
              >
                {m}
              </button>
            ))}
          </div>
        )}
        {row ? (
          <PartDetail
            key={row.id}
            design={design}
            row={row}
            onSelect={(id) => onSelect(id ? { kind: "card", id: card.id, member: id } : null)}
          />
        ) : (
          <Empty>{t.noPart}</Empty>
        )}
      </div>
    );
  }
  const nets =
    selected.kind === "net"
      ? [selected.id]
      : (model.edges.find((e) => e.id === selected.id)?.nets ?? []);
  const current = selected.member && nets.includes(selected.member) ? selected.member : nets[0];
  const net = design.nets.find((n) => n.id === current);
  return (
    <div>
      {nets.length > 1 && (
        <div className="flex flex-wrap gap-1 border-line border-b px-4 py-2">
          {nets.map((n) => (
            <button
              key={n}
              type="button"
              onClick={() => onSelect({ ...selected, member: n })}
              className={cn(
                "rounded-chip border px-1.5 py-0.5 font-mono text-[11px]",
                n === current
                  ? "border-accent bg-accent-tint text-accent-ink"
                  : "border-line hover:bg-panel-2",
              )}
            >
              {n}
            </button>
          ))}
        </div>
      )}
      {net ? <NetInspector net={net} /> : <Empty>{t.noNet}</Empty>}
    </div>
  );
}
