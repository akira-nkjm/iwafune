import { setValueOp } from "@/api/op-values";
import { useApplyOps } from "@/api/ops";
import type { PartRow, ValueView } from "@/api/types";
import { paramPath } from "@/lib/design";
import { EditableQuantity } from "@/quantity/EditableQuantity";
import { quantityInputText, stateValue, valueText } from "@/quantity/format";
import { Popover, PopoverContent, PopoverTrigger } from "@/ui/popover";

/** 状態ごとの値のセル: 表では display を 1 行で出し、開くと状態ごとに直接編集できる */
export function ByStateCell({
  part,
  attr,
  value,
  dimension,
}: {
  part: PartRow;
  attr: string;
  value: ValueView | undefined;
  dimension: string | null;
}) {
  const apply = useApplyOps();
  return (
    <Popover>
      <PopoverTrigger asChild>
        <button
          type="button"
          className="max-w-56 truncate rounded-[6px] px-1 py-0.5 text-left hover:bg-accent-tint"
          title={valueText(value)}
        >
          {valueText(value)}
        </button>
      </PopoverTrigger>
      <PopoverContent className="w-72 p-2">
        <p className="mb-1 px-1 font-mono text-[11px] text-ink-3">{paramPath(part, attr)}</p>
        {part.states.map((s) => {
          const qv = stateValue(value, s);
          const path = paramPath(part, attr, s);
          return (
            <div
              key={s}
              className="grid grid-cols-[70px_1fr] items-center gap-2 px-1 py-0.5 text-xs"
            >
              <span className="font-mono text-ink-3">{s}</span>
              <EditableQuantity
                display={qv?.display ?? "—"}
                initial={quantityInputText(qv)}
                dimension={dimension}
                label={path}
                muted={!qv}
                onCommit={(q, t) =>
                  apply.mutate({
                    ops: [setValueOp(part.id, attr, s, q)],
                    summary: `${path} = ${t}`,
                  })
                }
              />
            </div>
          );
        })}
      </PopoverContent>
    </Popover>
  );
}
