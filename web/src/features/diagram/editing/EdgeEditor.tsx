import { useReactFlow } from "@xyflow/react";
import { useRef } from "react";
import { useT } from "@/i18n";
import type { Pt } from "../layout/geometry";
import { addBend, moveSegment, orthogonal, removeBend } from "../layout/routes";

/** 取っ手の重なり順（線とラベルより上） */
export const EDIT_Z = 1001;

type EditProps = {
  points: Pt[];
  onChange: (points: Pt[]) => void;
  onPreview: (points: Pt[] | null) => void;
};

function RouteHandle({
  at,
  index,
  bend,
  points,
  onChange,
  onPreview,
}: EditProps & { at: Pt; index: number; bend: boolean }) {
  const { screenToFlowPosition } = useReactFlow();
  const drag = useRef<{ base: Pt[]; next: Pt[]; moved: boolean } | null>(null);
  const td = useT().diagram;
  const change = (base: Pt[], pos: Pt) =>
    bend ? orthogonal(base.map((p, i) => (i === index ? pos : p))) : moveSegment(base, index, pos);
  return (
    <button
      type="button"
      aria-label={bend ? td.bend : td.segment}
      title={bend ? td.bend : td.segment}
      className="nodrag nopan pointer-events-auto absolute size-2.5 border border-accent bg-panel focus:ring-2 focus:ring-accent"
      style={{
        transform: `translate(-50%, -50%) translate(${at.x}px, ${at.y}px)`,
        borderRadius: bend ? "50%" : 0,
        // 線（React Flow の辺の層）より上に出す。下にあると、線の太い当たり判定に押す操作を取られる
        zIndex: EDIT_Z,
      }}
      onPointerDown={(e) => {
        e.stopPropagation();
        e.currentTarget.setPointerCapture(e.pointerId);
        drag.current = { base: points, next: points, moved: false };
      }}
      onPointerMove={(e) => {
        if (!drag.current) return;
        drag.current.next = change(
          drag.current.base,
          screenToFlowPosition({ x: e.clientX, y: e.clientY }),
        );
        drag.current.moved = true;
        onPreview(drag.current.next);
      }}
      onPointerUp={(e) => {
        e.stopPropagation();
        if (drag.current?.moved) onChange(drag.current.next);
        drag.current = null;
        onPreview(null);
      }}
      onPointerCancel={() => {
        drag.current = null;
        onPreview(null);
      }}
      onDoubleClick={(e) => {
        e.stopPropagation();
        onChange(bend ? removeBend(points, index) : addBend(points, index));
      }}
      onKeyDown={(e) => {
        const delta = {
          ArrowLeft: [-10, 0],
          ArrowRight: [10, 0],
          ArrowUp: [0, -10],
          ArrowDown: [0, 10],
        }[e.key];
        if (!delta) return;
        e.preventDefault();
        e.stopPropagation();
        onChange(change(points, { x: at.x + (delta[0] ?? 0), y: at.y + (delta[1] ?? 0) }));
      }}
    />
  );
}

/**
 * 選んだ線の取っ手（区間の中点に四角、折れ点に丸）。動かしている間は親がプレビューを描き、離したら保存する。
 * 取っ手は部品として固定し、プレビューで描き直してもポインタのつかみが外れないようにする。
 */
export function EdgeEditor(props: EditProps) {
  const { points } = props;
  const segments = points.slice(0, -1).map((p, i) => ({
    at: { x: (p.x + (points[i + 1]?.x ?? p.x)) / 2, y: (p.y + (points[i + 1]?.y ?? p.y)) / 2 },
    index: i,
    bend: false,
  }));
  const bends = points.slice(1, -1).map((p, i) => ({ at: p, index: i + 1, bend: true }));
  return (
    <>
      {segments.concat(bends).map((handle) => (
        <RouteHandle key={`${handle.bend}:${handle.index}`} {...props} {...handle} />
      ))}
    </>
  );
}
