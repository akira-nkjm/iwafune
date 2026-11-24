import type { Role } from "@/api/types";
import { RoleIcon } from "@/kit/RoleIcon";
import { cn } from "@/lib/cn";
import { type Tone, toneInkVar, toneTint } from "@/lib/tone";

/**
 * 1 つのものを表すカード（役割の丸いアイコン・小さな補足の行・名前）。`count` が 2 以上のときは右上に数の丸と
 * 重ねた影を出す。押すと `onSelect`（空いた所の選択解除を止める）、`selected` で輪を付ける。
 */
export function ItemCard({
  role,
  tone,
  caption,
  name,
  count,
  title,
  selected,
  onSelect,
  width,
}: {
  /** 役割（アイコンになる。無ければ箱） */
  role: Role | null;
  tone: Tone;
  /** 名前の上の小さな補足の行 */
  caption: string;
  name: string;
  /** 員数。2 以上のときだけ数の丸と重ねた影を出す */
  count?: number;
  /** ホバーで出す説明 */
  title?: string;
  selected: boolean;
  onSelect: () => void;
  /** カードの幅 [px] */
  width: number;
}) {
  const stacked = (count ?? 0) > 1;
  return (
    <div className="relative" style={{ width }}>
      <button
        type="button"
        onClick={(e) => {
          e.stopPropagation();
          onSelect();
        }}
        title={title}
        className={cn(
          "flex w-full items-center gap-3 rounded-[12px] border border-line bg-panel px-3 py-2.5 text-left transition-colors hover:border-line-strong",
          selected && "ring-2 ring-accent ring-offset-1",
        )}
        style={{
          boxShadow: stacked
            ? "4px 4px 0 -1px var(--color-panel), 4px 4px 0 0 var(--color-line-strong), 8px 8px 0 -1px var(--color-panel), 8px 8px 0 0 var(--color-line)"
            : undefined,
        }}
      >
        <span
          className="flex size-10 shrink-0 items-center justify-center rounded-full"
          style={{ background: toneTint(tone, "icon"), color: toneInkVar[tone] }}
        >
          <RoleIcon role={role} size={20} />
        </span>
        <span className="min-w-0">
          <span className="block truncate text-[11.5px] text-ink-3">{caption}</span>
          <span className="block truncate font-medium text-[14px] text-ink">{name}</span>
        </span>
      </button>
      {stacked && (
        <span className="num absolute -top-2.5 -right-2.5 flex size-7 items-center justify-center rounded-full border border-line bg-panel text-[12px] text-ink-2 shadow-panel">
          {count}
        </span>
      )}
    </div>
  );
}
