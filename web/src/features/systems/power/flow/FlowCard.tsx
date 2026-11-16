import { Handle, type Node, type NodeProps, Position } from "@xyflow/react";
import { ChevronDown, ChevronUp } from "lucide-react";
import type { AnalysisKind, FlowNode, FlowRow, Verdict } from "@/api/types";
import { localized, useT } from "@/i18n";
import { VerdictPill } from "@/kit/status/VerdictPill";
import { verdictTone } from "@/kit/status/verdict-tone";
import { cn } from "@/lib/cn";
import { withNote } from "@/lib/design";
import { systemTone } from "@/lib/systems";
import { toneBorder, toneTint } from "@/lib/tone";
import { headerHeight, rowCenter, rowHandle, type ShownRow, SIZE } from "./layout";

/** 強調の状態（何も載せていない・強調・薄く） */
export type Emphasis = "normal" | "focus" | "dim";

export type FlowCardType = Node<
  {
    node: FlowNode;
    /** カードに出す行（`layout.ts` の `shownRows`） */
    shown: ShownRow[];
    emphasis: Emphasis;
    /** 強調する行の鍵 */
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
        style={{ height: headerHeight(node) - 4, background: toneTint(tone, "card") }}
      >
        <div className="min-w-0">
          <div className="truncate font-bold text-[12.5px] leading-tight">
            {localized(node.label, node.label_en)}
          </div>
          <Sub node={node} />
          <Description node={node} />
        </div>
        {analysis && node.verdict && <VerdictPill verdict={node.verdict} />}
      </div>
      <ul className="pt-1">
        {data.shown.map((r) => (
          <CardRow key={r.key} row={r} focused={data.rows.has(r.key)} onHover={data.onRowHover} />
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
            style={{ top: SIZE.analysisHeader / 2 - 6 }}
          />
          <Handle
            id="res-in"
            type="target"
            position={Position.Right}
            className={HANDLE}
            style={{ top: SIZE.analysisHeader / 2 + 6 }}
          />
        </>
      )}
      {!analysis &&
        data.shown.map((r, i) => (
          <Handle
            key={r.key}
            id={rowHandle(r.key)}
            type="source"
            position={Position.Right}
            className={r.kind === "row" && r.row.derived ? HIDDEN : HANDLE}
            style={{ top: Math.min(rowCenter(i), (height ?? 0) - 4) }}
          />
        ))}
    </div>
  );
}

/** 見出しの補足（1 部品は ID、まとめたカードは部品の数、解析は ID と定常か時系列かの印） */
function Sub({ node }: { node: FlowNode }) {
  const t = useT().systems.flow;
  const text = node.members.length > 0 ? t.members(node.members.length) : node.sub;
  if (!text && !node.analysis_kind) return null;
  return (
    <div className="flex min-w-0 items-center gap-1.5 leading-tight">
      {text && <span className="truncate font-mono text-[10px] text-ink-3">{text}</span>}
      {node.analysis_kind && <KindMark kind={node.analysis_kind} />}
    </div>
  );
}

/** 定常か時系列かの印（解析のカードと詳細の欄） */
export function KindMark({ kind }: { kind: AnalysisKind }) {
  const t = useT().systems;
  return (
    <span className="shrink-0 rounded-chip border border-line px-1 text-[9.5px] text-ink-3 leading-[14px]">
      {kind === "steady" ? t.steady : t.timeSeries}
    </span>
  );
}

/** 解析の説明の 1 行（長ければ切る。全文は載せたときの表示と詳細の欄） */
function Description({ node }: { node: FlowNode }) {
  if (!node.description) return null;
  const text = localized(node.description, node.description_en);
  return (
    <div className="mt-0.5 truncate text-[10px] text-ink-3 leading-tight" title={text}>
      {text}
    </div>
  );
}

function CardRow({
  row,
  focused,
  onHover,
}: {
  row: ShownRow;
  focused: boolean;
  onHover: (row: string | null) => void;
}) {
  const t = useT().systems.flow;
  const label =
    row.kind === "row"
      ? localized(row.row.label, row.row.label_en)
      : row.kind === "more"
        ? t.more(row.count)
        : t.unsetSummary(row.count);
  return (
    <li
      className={cn(
        "grid grid-cols-[minmax(0,1fr)_auto] items-center gap-2 px-2.5 text-[11px]",
        focused && "bg-accent-tint",
      )}
      style={{ height: SIZE.row }}
      onMouseEnter={() => onHover(row.key)}
      onMouseLeave={() => onHover(null)}
    >
      <span
        className={cn("truncate", row.kind === "row" ? "text-ink-2" : "text-ink-4 italic")}
        title={label}
      >
        {label}
      </span>
      {row.kind === "row" && <RowValue row={row.row} />}
    </li>
  );
}

/** 値（まとめたカードで部品ごとに違えば部品の数、書かれていなければ「未設定」） */
export function RowValue({ row }: { row: FlowRow }) {
  const t = useT().systems.flow;
  const display = row.display ? localized(row.display, row.display_en) : null;
  if (!display) {
    const text = row.varies ? t.varies(row.varies) : row.set ? "—" : t.unset;
    return <span className="text-ink-4">{text}</span>;
  }
  return (
    <span
      className={cn(
        "max-w-[140px] truncate font-mono text-[10.5px]",
        row.derived && "text-ink-3 italic",
        valueTone(row.verdict),
      )}
      title={row.derived ? withNote(display, t.derived) : display}
    >
      {display}
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
