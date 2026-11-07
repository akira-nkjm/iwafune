import { useReactFlow } from "@xyflow/react";
import { GripVertical, RotateCcw } from "lucide-react";
import { useEffect, useRef, useState } from "react";
import type { ViewEdge, ViewPortSide } from "@/api/types";
import { useT } from "@/i18n";
import { NativeSelect } from "@/kit/NativeSelect";
import { EDIT_Z } from "./EdgeEditor";
import { CARD_H, CARD_W, type Pt } from "./layout-geometry";
import { attachTo, reattach } from "./routes";

/** 付け替えたときに渡すもの: 側・側の上の位置（左か上の角からの距離 [px]）・付け替えた経路 */
export type Attach = { side: ViewPortSide; offset: number; points: Pt[] };

/**
 * 線の端の丸い取っ手。ドラッグすると、同じカードの縁に沿って端を好きな位置へ付け替える（いちばん近い側の、
 * 4 px 刻みの位置）。動かしている間は取っ手が縁の上をカーソルについて動き、付け替えた経路をプレビューし、
 * 離したら保存する。保存した位置に端子が描かれるまでは、取っ手を離した所に置いておく（元の位置へ戻らないように）。
 * 表示だけで、どのポートにつながるかは変えない。
 */
function EndpointHandle({
  node,
  end,
  at,
  points,
  onPreview,
  onAttach,
}: {
  node: string;
  end: "source" | "target";
  at: Pt;
  points: Pt[];
  onPreview: (points: Pt[] | null) => void;
  onAttach: (end: "source" | "target", attach: Attach) => void;
}) {
  const { getInternalNode, screenToFlowPosition } = useReactFlow();
  const drag = useRef<(Attach & { edge: Pt }) | null>(null);
  // ドラッグ中と保存待ちの間の取っ手の位置（縁の上の点）。端子の位置（`at`）が変わったら外す
  const [held, setHeld] = useState<Pt | null>(null);
  // biome-ignore lint/correctness/useExhaustiveDependencies: 端子の位置が変わった（保存した位置に描かれた）ら外す
  useEffect(() => setHeld(null), [at.x, at.y]);
  const td = useT().diagram;
  const attachAt = (clientX: number, clientY: number): (Attach & { edge: Pt }) | null => {
    const n = getInternalNode(node);
    if (n?.type !== "card") return null;
    const card = {
      ...n.internals.positionAbsolute,
      width: n.measured.width ?? CARD_W,
      height: n.measured.height ?? CARD_H,
    };
    const target = attachTo(card, screenToFlowPosition({ x: clientX, y: clientY }));
    return {
      side: target.side,
      offset: target.offset,
      points: reattach(points, end, target.at, target.side, card),
      edge: target.at,
    };
  };
  const shown = held ?? at;
  return (
    <button
      type="button"
      aria-label={td.reattach}
      title={td.reattach}
      className="nodrag nopan pointer-events-auto absolute size-3 cursor-grab rounded-full border-2 border-accent bg-panel"
      style={{
        transform: `translate(-50%, -50%) translate(${shown.x}px, ${shown.y}px)`,
        zIndex: EDIT_Z,
      }}
      onPointerDown={(e) => {
        e.stopPropagation();
        e.currentTarget.setPointerCapture(e.pointerId);
        drag.current = attachAt(e.clientX, e.clientY);
      }}
      onPointerMove={(e) => {
        if (!drag.current) return;
        drag.current = attachAt(e.clientX, e.clientY) ?? drag.current;
        setHeld(drag.current.edge);
        onPreview(drag.current.points);
      }}
      onPointerUp={(e) => {
        e.stopPropagation();
        const attach = drag.current;
        drag.current = null;
        onPreview(null);
        if (attach) onAttach(end, attach);
        else setHeld(null);
      }}
      onPointerCancel={() => {
        drag.current = null;
        setHeld(null);
        onPreview(null);
      }}
    />
  );
}

/**
 * 線を選んだときの操作の欄（自動に戻す・始点と終点の側）。左端のつまみをドラッグすると欄を動かせる（線やカードに
 * かぶさったとき。動かした位置はこの線を選んでいる間だけ覚える）。
 */
function RoutePanel({
  at,
  manual,
  ends,
  onSide,
  onReset,
}: {
  at: Pt;
  manual?: ViewEdge;
  ends: ("source" | "target")[];
  onSide: (end: "source" | "target", side: ViewPortSide | undefined) => void;
  onReset: () => void;
}) {
  const td = useT().diagram;
  const { getZoom } = useReactFlow();
  const [offset, setOffset] = useState<Pt>({ x: 0, y: 0 });
  const grab = useRef<{ x: number; y: number; from: Pt } | null>(null);
  return (
    <div
      className="nodrag nopan pointer-events-auto absolute flex items-center gap-2 rounded-[10px] border border-line bg-panel py-1 pr-1.5 pl-0.5 text-[11px] shadow-panel"
      style={{
        transform: `translate(${at.x + offset.x}px, ${at.y + 18 + offset.y}px)`,
        zIndex: EDIT_Z,
      }}
    >
      <button
        type="button"
        aria-label={td.movePanel}
        title={td.movePanel}
        className="flex h-6 w-4 cursor-grab items-center justify-center rounded text-ink-3 hover:bg-sunken hover:text-ink active:cursor-grabbing"
        onPointerDown={(e) => {
          e.stopPropagation();
          e.currentTarget.setPointerCapture(e.pointerId);
          grab.current = { x: e.clientX, y: e.clientY, from: offset };
        }}
        onPointerMove={(e) => {
          const g = grab.current;
          if (!g) return;
          // 欄は図の座標の上にあるので、画面の動きを拡大率で割る
          const zoom = getZoom() || 1;
          setOffset({
            x: g.from.x + (e.clientX - g.x) / zoom,
            y: g.from.y + (e.clientY - g.y) / zoom,
          });
        }}
        onPointerUp={() => {
          grab.current = null;
        }}
        onPointerCancel={() => {
          grab.current = null;
        }}
      >
        <GripVertical size={12} aria-hidden />
      </button>
      <button
        type="button"
        onClick={onReset}
        className="flex h-6 items-center gap-1 rounded-[7px] border border-line px-1.5 font-medium text-ink-2 hover:bg-sunken hover:text-ink"
      >
        <RotateCcw size={11} aria-hidden />
        {td.resetRoute}
      </button>
      {ends.map((end) => (
        <div key={end} className="flex items-center gap-1 text-ink-3">
          <span aria-hidden>{end === "source" ? td.sourceSide : td.targetSide}</span>
          <NativeSelect
            aria-label={end === "source" ? td.sourceSide : td.targetSide}
            size="sm"
            className="h-6 text-[11px]"
            value={manual?.[`${end}_side`] ?? ""}
            onChange={(e) => onSide(end, (e.target.value || undefined) as ViewPortSide | undefined)}
            placeholder={td.automatic}
            options={(["left", "right", "top", "bottom"] satisfies ViewPortSide[]).map((side) => ({
              value: side,
              label: td[side],
            }))}
          />
        </div>
      ))}
    </div>
  );
}

export function RouteControls({
  at,
  manual,
  source,
  target,
  start,
  finish,
  points,
  onPreview,
  onAttach,
  onSide,
  onReset,
}: {
  at: Pt;
  manual?: ViewEdge;
  source: string;
  target: string;
  start: Pt;
  finish: Pt;
  /** いま描いている経路（端を付け替えるときの元） */
  points: Pt[];
  onPreview: (points: Pt[] | null) => void;
  onAttach: (end: "source" | "target", attach: Attach) => void;
  onSide: (end: "source" | "target", side: ViewPortSide | undefined) => void;
  onReset: () => void;
}) {
  const { getInternalNode } = useReactFlow();
  // 端の側を選べるのはカードの端だけ（ジャンクションは丸なので側が無い）
  const ends = (["source", "target"] as const).filter(
    (end) => getInternalNode(end === "source" ? source : target)?.type === "card",
  );
  return (
    <>
      {ends.map((end) => (
        <EndpointHandle
          key={end}
          node={end === "source" ? source : target}
          end={end}
          at={end === "source" ? start : finish}
          points={points}
          onPreview={onPreview}
          onAttach={onAttach}
        />
      ))}
      <RoutePanel at={at} manual={manual} ends={ends} onSide={onSide} onReset={onReset} />
    </>
  );
}
