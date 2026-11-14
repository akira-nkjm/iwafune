import { Handle, type Node, type NodeProps, Position } from "@xyflow/react";
import { ChevronDown, ChevronUp } from "lucide-react";
import type { FlowNode, FlowRow, Verdict } from "@/api/types";
import { localized, useT } from "@/i18n";
import { VerdictPill } from "@/kit/status/VerdictPill";
import { verdictTone } from "@/kit/status/verdict-tone";
import { cn } from "@/lib/cn";
import { withNote } from "@/lib/design";
import { systemTone } from "@/lib/systems";
import { toneBorder, toneTint } from "@/lib/tone";
import { rowCenter, rowHandle, SIZE } from "./layout";

/** 強調の状態（何も載せていない・強調・薄く） */
export type Emphasis = "normal" | "focus" | "dim";

export type FlowCardType = Node<
  {
    node: FlowNode;
    emphasis: Emphasis;
    /** 強調する行の ID */
    rows: ReadonlySet<string>;
    selected: boolean;
    onRowHover: (row: string | null) => void;
  },
  "flowCard"
>;

export type FlowFoldType = Node<
  { hidden: number; expanded: boolean; emphasis: Emphasis; onToggle: () => void },
  "flowFold"
>;

export type FlowHeadingType = Node<{ label: string }, "flowHeading">;

const HANDLE = "!size-1.5 !min-h-0 !min-w-0 !border-0 !bg-ink-3";
const HIDDEN = "!opacity-0 !pointer-events-none";

const verdictText = {
  pos: "text-pos-ink",
  warn: "text-warn-ink",
  neg: "text-neg-ink",
} as const;

/** 設定・モード・ケースのカードの色（系に属さない中立色） */
function toneOf(node: FlowNode) {
  return node.kind === "part" || node.kind === "analysis" ? systemTone(node.system) : "gray";
}

/**
 * フロー図のカード: 系統図の部品のカードと同じ角丸・枠・サブシステム色の淡い地の見出しに、値の行（ラベルと値）を並べる。
 * 解析のカードは見出しに判定、行は主な出力。値の行の右端に線の出口、解析のカードの左に入口（右に結果の出入口）。
 */
export function FlowCard({ data, width, height }: NodeProps<FlowCardType>) {
  const { node } = data;
  const tone = toneOf(node);
  const analysis = node.kind === "analysis";
  return (
    <div
      className={cn(
        "relative rounded-[10px] border-[1.5px] bg-panel text-ink transition-opacity",
        toneBorder[tone],
        data.emphasis === "dim" && "opacity-35",
        data.selected && "ring-2 ring-accent ring-offset-1",
      )}
      style={{ width, height }}
    >
      <div
        className="flex items-start justify-between gap-2 rounded-t-[8.5px] px-2.5 pt-1.5"
        style={{ height: SIZE.header - 4, background: toneTint(tone, "card") }}
      >
        <div className="min-w-0">
          <div className="truncate font-bold text-[12.5px] leading-tight">
            {localized(node.label, node.label_en)}
          </div>
          {node.sub && (
            <div className="truncate font-mono text-[10px] text-ink-3 leading-tight">
              {node.sub}
            </div>
          )}
        </div>
        {analysis && node.verdict && <VerdictPill verdict={node.verdict} />}
      </div>
      <ul className="pt-1">
        {node.rows.map((r) => (
          <CardRow key={r.id} row={r} focused={data.rows.has(r.id)} onHover={data.onRowHover} />
        ))}
      </ul>
      {analysis && (
        <>
          <Handle id="in" type="target" position={Position.Left} className={HANDLE} />
          <Handle
            id="res-out"
            type="source"
            position={Position.Right}
            className={HANDLE}
            style={{ top: SIZE.header / 2 - 6 }}
          />
          <Handle
            id="res-in"
            type="target"
            position={Position.Right}
            className={HANDLE}
            style={{ top: SIZE.header / 2 + 6 }}
          />
        </>
      )}
      {!analysis &&
        node.rows.map((r, i) => (
          <Handle
            key={r.id}
            id={rowHandle(r.id)}
            type="source"
            position={Position.Right}
            className={r.derived ? HIDDEN : HANDLE}
            style={{ top: Math.min(rowCenter(i), (height ?? 0) - 4) }}
          />
        ))}
    </div>
  );
}

function CardRow({
  row,
  focused,
  onHover,
}: {
  row: FlowRow;
  focused: boolean;
  onHover: (row: string | null) => void;
}) {
  const t = useT().systems.flow;
  const label = localized(row.label, row.label_en);
  return (
    <li
      className={cn(
        "grid grid-cols-[minmax(0,1fr)_auto] items-center gap-2 px-2.5 text-[11px]",
        focused && "bg-accent-tint",
      )}
      style={{ height: SIZE.row }}
      onMouseEnter={() => onHover(row.id)}
      onMouseLeave={() => onHover(null)}
    >
      <span className="truncate text-ink-2" title={label}>
        {label}
      </span>
      <RowValue row={row} unset={t.unset} derived={t.derived} />
    </li>
  );
}

function RowValue({ row, unset, derived }: { row: FlowRow; unset: string; derived: string }) {
  if (!row.display) return <span className="text-ink-4">{row.set ? "—" : unset}</span>;
  return (
    <span
      className={cn(
        "max-w-[132px] truncate font-mono text-[10.5px]",
        row.derived && "text-ink-3 italic",
        valueTone(row.verdict),
      )}
      title={row.derived ? withNote(row.display, derived) : row.display}
    >
      {row.display}
    </span>
  );
}

function valueTone(verdict: Verdict | null): string | undefined {
  return verdict ? verdictText[verdictTone[verdict]] : undefined;
}

/** 畳んだ部品の「ほか N 件」（開いていれば「畳む」）のカード */
export function FlowFoldCard({ data, width, height }: NodeProps<FlowFoldType>) {
  const t = useT().systems.flow;
  return (
    <button
      type="button"
      onClick={data.onToggle}
      className={cn(
        "flex items-center justify-center gap-1 rounded-[10px] border-[1.5px] border-line-strong border-dashed bg-panel-2 text-[11.5px] text-ink-2 hover:bg-sunken hover:text-ink",
        data.emphasis === "dim" && "opacity-35",
      )}
      style={{ width, height }}
    >
      {data.expanded ? <ChevronUp size={13} aria-hidden /> : <ChevronDown size={13} aria-hidden />}
      {data.expanded ? t.fewer : t.more(data.hidden)}
      <Handle id="out" type="source" position={Position.Right} className={HANDLE} />
    </button>
  );
}

/** 列の見出し（部品・条件と設定・解析） */
export function FlowHeading({ data }: NodeProps<FlowHeadingType>) {
  return (
    <div className="font-semibold text-[11px] text-ink-3 uppercase tracking-wide">{data.label}</div>
  );
}
