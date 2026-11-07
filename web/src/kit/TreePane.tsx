import { type PointerEvent, type ReactNode, useEffect, useRef, useState } from "react";
import { useT } from "@/i18n";
import {
  clampTreeWidth,
  DETAIL_WIDTH,
  type PaneWidth,
  readTreeWidth,
  TREE_WIDTH,
  writeTreeWidth,
} from "./tree-width";

/** 幅を決める欄の幅と、境目のつまみの操作。`sign` は右へ動かしたときに幅が増えるなら 1、減るなら -1 */
function usePaneWidth(spec: PaneWidth, sign: 1 | -1) {
  const [width, setWidth] = useState<number>(spec.initial);
  const drag = useRef<{ x: number; width: number } | null>(null);
  useEffect(() => setWidth(readTreeWidth(undefined, spec)), [spec]);
  const update = (next: number, save = false) => {
    const value = clampTreeWidth(next, spec);
    setWidth(value);
    if (save) writeTreeWidth(value, undefined, spec);
  };
  const dragged = (event: PointerEvent, from: { x: number; width: number }) =>
    from.width + sign * (event.clientX - from.x);
  const move = (event: PointerEvent) => {
    if (drag.current) update(dragged(event, drag.current));
  };
  const finish = (event: PointerEvent) => {
    if (!drag.current) return;
    update(dragged(event, drag.current), true);
    drag.current = null;
    if (event.currentTarget.hasPointerCapture(event.pointerId))
      event.currentTarget.releasePointerCapture(event.pointerId);
  };
  return {
    width,
    separator: {
      onPointerDown: (event: PointerEvent) => {
        if (event.button !== 0) return;
        event.preventDefault();
        event.currentTarget.setPointerCapture(event.pointerId);
        drag.current = { x: event.clientX, width };
      },
      onPointerMove: move,
      onPointerUp: finish,
      onPointerCancel: () => {
        drag.current = null;
        writeTreeWidth(width, undefined, spec);
      },
      onLostPointerCapture: () => {
        drag.current = null;
      },
      onKeyDown: (event: React.KeyboardEvent) => {
        const widths: Record<string, number> = {
          ArrowLeft: width - sign * 10,
          ArrowRight: width + sign * 10,
          Home: spec.min,
          End: spec.max,
        };
        const next = widths[event.key];
        if (next === undefined) return;
        event.preventDefault();
        update(next, true);
      },
    },
  };
}

/**
 * 一覧と詳細を左右に並べ、境目のつまみ（ドラッグとキーボード）で幅を変える。`fixed` は幅を決める欄で、
 * `list`（既定。ツリー）は一覧の列幅、`detail`（カードの一覧）は詳細の欄の幅を決め、もう一方が残りを使う。
 * `detail` で詳細が無いときは一覧だけを全幅に出す（一覧の要素はそのままなので、選んでもスクロールが戻らない）。
 */
export function TreePane({
  list,
  detail,
  fixed = "list",
}: {
  list: ReactNode;
  detail: ReactNode;
  fixed?: "list" | "detail";
}) {
  const t = useT().common;
  const spec = fixed === "list" ? TREE_WIDTH : DETAIL_WIDTH;
  const { width, separator } = usePaneWidth(spec, fixed === "list" ? 1 : -1);
  const alone = fixed === "detail" && !detail;
  const columns = alone
    ? "minmax(0, 1fr)"
    : fixed === "list"
      ? `${width}px 6px minmax(0, 1fr)`
      : `minmax(0, 1fr) 6px ${width}px`;
  return (
    <div
      className="grid min-h-0 flex-1 border-line border-t"
      style={{ gridTemplateColumns: columns }}
    >
      <div className="min-h-0 min-w-0 overflow-hidden">{list}</div>
      {!alone && (
        <>
          {/* biome-ignore lint/a11y/useSemanticElements: 操作可能な縦のウィンドウ分割バー（静的な hr ではない） */}
          <div
            {...separator}
            role="separator"
            tabIndex={0}
            aria-label={t.resizeTree}
            aria-orientation="vertical"
            aria-valuemin={spec.min}
            aria-valuemax={spec.max}
            aria-valuenow={width}
            className="touch-none cursor-col-resize border-line border-x bg-panel-2 hover:bg-accent-tint focus-visible:bg-accent-tint focus-visible:outline-2 focus-visible:outline-accent"
          />
          <aside className="flex min-h-0 min-w-0 flex-col overflow-hidden [&>div]:flex-1">
            {detail}
          </aside>
        </>
      )}
    </div>
  );
}
