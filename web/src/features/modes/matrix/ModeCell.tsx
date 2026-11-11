import { type KeyboardEvent, type MouseEvent, useState } from "react";
import { useT } from "@/i18n";
import { cn } from "@/lib/cn";
import { useFormatted } from "@/quantity/use-formatted";
import { Popover, PopoverAnchor, PopoverContent, PopoverTrigger } from "@/ui/popover";
import type { CellText } from "../draft/draft";
import { ModeCellForm } from "./ModeCellForm";
import { cellSymbol, ON_SHADES, toggleCell } from "./matrix";
import { useCellFocus } from "./matrix-focus";
import type { MatrixCell, OnCell } from "./matrix-rows";

/**
 * モード表の 1 セル。クリックでオン・オフを切り替え、右クリック・D キー・右上の小さなボタンで、
 * 状態（idle・peak など）とほかの状態の時間割合を編集する（ポップオーバー）。数字はそのモードでのその部品の平均消費電力
 * （power.mode_load。下書きがあれば、それを書き込まずに当てて計算した値）。矢印キーの移動は行列の側（`MatrixFocus`）。
 */
export function ModeCell({
  cell,
  onCell,
}: {
  /** 出すセル（`matrixGroups` が作る。状態・文言・電力・塗りの段階・行列の中の位置） */
  cell: MatrixCell;
  /** セルを書き換えたとき（クリックの切り替えとポップオーバーの編集。下書きにする） */
  onCell: OnCell;
}) {
  const t = useT().modes;
  const [editing, setEditing] = useState(false);
  const focus = useCellFocus(cell.at);
  const { part, mode, states, value, changed } = cell;
  const formatted = useFormatted(cell.power ? cell.power : null, "Power");
  const symbol = cellSymbol(value);
  const lit = symbol !== "—";
  const change = (next: CellText | null) => onCell(cell, next);
  const openEditor = (event: KeyboardEvent | MouseEvent) => {
    event.preventDefault();
    setEditing(true);
  };
  const onKeyDown = (event: KeyboardEvent<HTMLButtonElement>) => {
    if (focus.onKeyDown(event)) return;
    if (event.key === "d" || event.key === "D" || event.key === "ContextMenu") openEditor(event);
  };
  const where = `${part} · ${mode}`;
  return (
    <Popover open={editing} onOpenChange={setEditing}>
      <PopoverAnchor asChild>
        <div className="group relative">
          <button
            ref={focus.ref}
            type="button"
            aria-label={t.cellLabel(
              part,
              mode,
              value?.state ?? t.cell.unassigned,
              formatted ?? "—",
            )}
            aria-keyshortcuts="D"
            onClick={() => change(toggleCell(states, value))}
            onContextMenu={openEditor}
            onKeyDown={onKeyDown}
            className={cn(
              "flex h-12 w-full min-w-24 select-none flex-col items-center justify-center rounded-md px-2 leading-tight outline-none hover:ring-1 hover:ring-accent focus-visible:ring-2 focus-visible:ring-accent",
              lit ? `${ON_SHADES[Math.min(3, Math.max(0, cell.shade))]} text-ink` : "text-ink-4",
              changed && "ring-1 ring-warn",
            )}
          >
            <span className="whitespace-nowrap text-xs">
              {symbol}
              {formatted && <span className="num ml-1">{formatted}</span>}
            </span>
            <CellDetail cell={cell} />
          </button>
          <PopoverTrigger asChild>
            <button
              type="button"
              tabIndex={-1}
              aria-hidden="true"
              title={t.cell.editOf(where)}
              className="absolute top-0.5 right-0.5 rounded px-1 text-[10px] text-ink-3 opacity-0 hover:bg-panel group-hover:opacity-100"
            >
              ◐
            </button>
          </PopoverTrigger>
        </div>
      </PopoverAnchor>
      <PopoverContent className="w-72 p-3" aria-label={t.cell.editOf(where)}>
        <ModeCellForm states={states} value={value} onChange={change} />
      </PopoverContent>
    </Popover>
  );
}

/** セルの 2 行目: 状態の名前と時間割合（オン・オフだけのときは出さない） */
function CellDetail({ cell }: { cell: Pick<MatrixCell, "value" | "display" | "changed"> }) {
  const t = useT().modes.cell;
  const { value, display, changed } = cell;
  if (!value) return <span className="text-[10px] text-warn-ink">{t.unassigned}</span>;
  const duty = Object.entries(value.duty ?? {});
  if (duty.length === 0 && (value.state === "on" || value.state === "off")) return null;
  // 下書きのセルはサーバーの整形が無いので、打った値をそのまま見せる
  const text =
    changed || !display ? [value.state, ...duty.map(([s, f]) => `${s} ${f}`)].join(" · ") : display;
  return <span className="max-w-full truncate text-[10px] opacity-80">{text}</span>;
}
