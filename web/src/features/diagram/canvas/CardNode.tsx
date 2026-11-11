import { Handle, type Node, type NodeProps, Position } from "@xyflow/react";
import type { ViewPortSide } from "@/api/types";
import { cn } from "@/lib/cn";
import { toneBorder, toneHex, toneTint } from "@/lib/tone";
import type { DiagramCard } from "../model";

/** 線の端を手で付けた所（ハンドル ID・カードの側・その側の左か上の角からの距離 [px]） */
export type Attachment = { id: string; side: ViewPortSide; offset: number };

export type CardNodeType = Node<
  {
    card: DiagramCard;
    selected: boolean;
    ports: {
      id: string;
      port: string;
      x: number;
      y: number;
      side: "left" | "right" | "top" | "bottom";
    }[];
    connectable: boolean;
    /** 線の端を手で付けた所（use-diagram-flow.ts）。その位置に端子を描く */
    attachments?: Attachment[];
  },
  "card"
>;

const sidePos = {
  left: Position.Left,
  right: Position.Right,
  top: Position.Top,
  bottom: Position.Bottom,
} as const;

/**
 * 部品のカード: サブシステム色の淡い地と枠、ID（太字）とカタログ・種別。員数が 2 以上か、同種をまとめたときは
 * 重ねたカードに「×N」。組立品は子のカードを中に持つ（大きさは自動配置が決める）。ポートは自動配置の位置にハンドルを置く。
 */
export function CardNode({ data, width, height }: NodeProps<CardNodeType>) {
  const { card } = data;
  const stroke = toneHex[card.tone];
  const stacked = card.count > 1 && !card.assembly;
  return (
    <div
      className={cn(
        "relative h-full w-full rounded-[10px] border-[1.5px] text-ink",
        toneBorder[card.tone],
        data.selected && "ring-2 ring-accent ring-offset-1",
      )}
      style={{
        width,
        height,
        background: toneTint(card.tone, card.assembly ? "header" : "card"),
        boxShadow: stacked
          ? `4px 4px 0 -1px var(--color-panel), 4px 4px 0 0 ${stroke}, 8px 8px 0 -1px var(--color-panel), 8px 8px 0 0 ${stroke}`
          : undefined,
      }}
    >
      <div
        className={cn(
          "px-2.5",
          card.assembly ? "pt-1.5" : "flex h-full flex-col items-center justify-center text-center",
        )}
      >
        <div className="truncate font-bold text-[12.5px] leading-tight">
          {card.label}
          {stacked && <span className="ml-0.5">×{card.count}</span>}
        </div>
        <div className="truncate text-[10px] text-ink-3 leading-tight">{card.sub}</div>
      </div>
      {data.ports
        .flatMap((p, index) => [
          p,
          ...(["left", "right", "top", "bottom"] as const).map((side) => ({
            ...p,
            id: `${p.id}@${side}`,
            side,
            x:
              side === "left"
                ? 0
                : side === "right"
                  ? width
                  : ((index + 1) * (width ?? 150)) / (data.ports.length + 1),
            y:
              side === "top"
                ? 0
                : side === "bottom"
                  ? height
                  : ((index + 1) * (height ?? 52)) / (data.ports.length + 1),
          })),
        ])
        .map((p) => (
          <Handle
            key={p.id}
            id={p.id}
            type="source"
            position={sidePos[p.side]}
            isConnectable={data.connectable && !p.id.includes("@")}
            className={
              p.id.includes("@")
                ? "!opacity-0 !pointer-events-none"
                : "!size-1.5 !min-h-0 !min-w-0 !border-0 !bg-ink-3"
            }
            style={{ left: p.x, top: p.y, transform: "translate(-50%, -50%)" }}
            title={`${card.members.join(", ")}.${p.port}`}
          />
        ))}
      {/* 線の端を手で付けた所の端子（位置はカードの大きさの中に収める） */}
      {(data.attachments ?? []).map((a) => {
        const w = width ?? 150;
        const h = height ?? 52;
        const along = Math.min(
          a.side === "left" || a.side === "right" ? h : w,
          Math.max(0, a.offset),
        );
        const at = {
          left: { x: 0, y: along },
          right: { x: w, y: along },
          top: { x: along, y: 0 },
          bottom: { x: along, y: h },
        }[a.side];
        return (
          <Handle
            key={a.id}
            id={a.id}
            type="source"
            position={sidePos[a.side]}
            isConnectable={false}
            className="!size-1.5 !min-h-0 !min-w-0 !border-0 !bg-ink-3"
            style={{ left: at.x, top: at.y, transform: "translate(-50%, -50%)" }}
          />
        );
      })}
    </div>
  );
}
