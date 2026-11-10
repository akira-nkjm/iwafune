import { type KeyboardEvent, type MouseEvent, useState } from "react";
import { useT } from "@/i18n";
import { cn } from "@/lib/cn";
import { useFormatted } from "@/quantity/use-formatted";
import { Popover, PopoverAnchor, PopoverContent, PopoverTrigger } from "@/ui/popover";
import type { CellText } from "./draft";
import { ModeCellForm } from "./ModeCellForm";
import { cellSymbol, ON_SHADES, toggleCell } from "./matrix";

const ARROWS = {
  ArrowLeft: [0, -1],
  ArrowRight: [0, 1],
  ArrowUp: [-1, 0],
  ArrowDown: [1, 0],
} as const satisfies Record<string, readonly [number, number]>;

/** 矢印キーで行列の中のセル（`data-mode-cell="行:列"`）へ動く。動いたら true */
function moveFocus(event: KeyboardEvent<HTMLButtonElement>, row: number, column: number): boolean {
  if (!(event.key in ARROWS)) return false;
  const [dr, dc] = ARROWS[event.key as keyof typeof ARROWS];
  event.preventDefault();
  event.currentTarget
    .closest("table")
    ?.querySelector<HTMLButtonElement>(`[data-mode-cell="${row + dr}:${column + dc}"]`)
    ?.focus();
  return true;
}

/**
 * モード表の 1 セル。クリックでオン・オフを切り替え、右クリック・D キー・右上の小さなボタンで、
 * 状態（idle・peak など）とほかの状態の時間割合を編集する（ポップオーバー）。数字はそのモードでのその部品の平均消費電力
 * （power.mode_load。下書きがあれば、それを書き込まずに当てて計算した値）。
 */
export function ModeCell({
  part,
  mode,
  row,
  column,
  states,
  cell,
  display,
  changed,
  power,
  shade,
  onChange,
}: {
  part: string;
  mode: string;
  /** 行列の中の位置（矢印キーの移動に使う） */
  row: number;
  column: number;
  states: string[];
  cell: CellText | null;
  /** 保存済みのセルの文言（例: `"off（on 20.0 %）"`。割合の `%` は API の整形、並べ方は i18n。`savedCellText`） */
  display: string | undefined;
  changed: boolean;
  /** 平均消費電力 [W]（解析の出力。無ければ null） */
  power: number | null;
  /** 塗りの段階（ON_SHADES の添字） */
  shade: number;
  onChange: (cell: CellText | null) => void;
}) {
  const t = useT().modes;
  const [editing, setEditing] = useState(false);
  const formatted = useFormatted(power ? power : null, "Power");
  const symbol = cellSymbol(cell);
  const lit = symbol !== "—";
  const openEditor = (event: KeyboardEvent | MouseEvent) => {
    event.preventDefault();
    setEditing(true);
  };
  const onKeyDown = (event: KeyboardEvent<HTMLButtonElement>) => {
    if (moveFocus(event, row, column)) return;
    if (event.key === "d" || event.key === "D" || event.key === "ContextMenu") openEditor(event);
  };
  const where = `${part} · ${mode}`;
  return (
    <Popover open={editing} onOpenChange={setEditing}>
      <PopoverAnchor asChild>
        <div className="group relative">
          <button
            type="button"
            data-mode-cell={`${row}:${column}`}
            aria-label={t.cellLabel(part, mode, cell?.state ?? t.cell.unassigned, formatted ?? "—")}
            aria-keyshortcuts="D"
            onClick={() => onChange(toggleCell(states, cell))}
            onContextMenu={openEditor}
            onKeyDown={onKeyDown}
            className={cn(
              "flex h-12 w-full min-w-24 select-none flex-col items-center justify-center rounded-md px-2 leading-tight outline-none hover:ring-1 hover:ring-accent focus-visible:ring-2 focus-visible:ring-accent",
              lit ? `${ON_SHADES[Math.min(3, Math.max(0, shade))]} text-ink` : "text-ink-4",
              changed && "ring-1 ring-warn",
            )}
          >
            <span className="whitespace-nowrap text-xs">
              {symbol}
              {formatted && <span className="num ml-1">{formatted}</span>}
            </span>
            <CellDetail cell={cell} display={display} changed={changed} />
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
        <ModeCellForm states={states} cell={cell} onChange={onChange} />
      </PopoverContent>
    </Popover>
  );
}

/** セルの 2 行目: 状態の名前と時間割合（オン・オフだけのときは出さない） */
function CellDetail({
  cell,
  display,
  changed,
}: {
  cell: CellText | null;
  display: string | undefined;
  changed: boolean;
}) {
  const t = useT().modes.cell;
  if (!cell) return <span className="text-[10px] text-warn-ink">{t.unassigned}</span>;
  const duty = Object.entries(cell.duty ?? {});
  if (duty.length === 0 && (cell.state === "on" || cell.state === "off")) return null;
  // 下書きのセルはサーバーの整形が無いので、打った値をそのまま見せる
  const text =
    changed || !display ? [cell.state, ...duty.map(([s, f]) => `${s} ${f}`)].join(" · ") : display;
  return <span className="max-w-full truncate text-[10px] opacity-80">{text}</span>;
}
