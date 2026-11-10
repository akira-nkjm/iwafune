import {
  type Edge,
  MiniMap,
  Panel,
  useOnViewportChange,
  useReactFlow,
  useStore,
} from "@xyflow/react";
import { Focus, Map as MapIcon, Maximize, ZoomIn, ZoomOut } from "lucide-react";
import { type ReactNode, useEffect, useRef } from "react";
import { useT } from "@/i18n";
import { cn } from "@/lib/cn";
import { inkVeil, uiHex } from "@/lib/tone";
import type { DiagramSelection } from "./DiagramInspector";

/** 拡大率の範囲（10%〜1000%） */
export const ZOOM_RANGE = { min: 0.1, max: 10 } as const;
/** 拡大・縮小・寄せる動きの時間 [ms] */
const DURATION = 200;
/** マウスのホイール 1 目盛りの拡大率（Ctrl / ⌘ + ホイール） */
const WHEEL_STEP = 1.2;
/** 拡大・移動が止まってから、図をいまの拡大率で描き直すまでの時間 [ms] */
const SHARPEN_AFTER = 150;

/** 選んでいるものに寄るときに見せるノード（カードはそのカード、線は両端、ネットはそのジャンクション） */
function focusNodes(selected: DiagramSelection, edges: readonly Edge[]): { id: string }[] {
  if (!selected) return [];
  if (selected.kind === "card") return [{ id: selected.id }];
  if (selected.kind === "edge") {
    const e = edges.find((x) => x.id === selected.id);
    return e ? [{ id: e.source }, { id: e.target }] : [];
  }
  return [];
}

/**
 * 系統図の拡大・縮小の操作（左下）: 縮小・いまの拡大率（押すと 100%）・拡大・全体を見る・選んだものに寄る・
 * 全体図（ミニマップ）の表示。キーボードでも操作できる（+ / − で拡大・縮小、0 で全体、1 で 100%、F で選んだものに寄る。
 * 文字を入力している欄の中では効かない）。マウスはホイールで移動（Shift + ホイールで横）、Ctrl / ⌘ + ホイールで拡大・縮小。
 */
export function DiagramZoom({
  selected,
  edges,
  minimap,
  onMinimap,
}: {
  selected: DiagramSelection;
  edges: readonly Edge[];
  minimap: boolean;
  onMinimap: (on: boolean) => void;
}) {
  const td = useT().diagram;
  const { zoomIn, zoomOut, zoomTo, fitView } = useReactFlow();
  const zoom = useStore((s) => s.transform[2]);
  const targets = focusNodes(selected, edges);
  const actions = {
    zoomIn: () => void zoomIn({ duration: DURATION }),
    zoomOut: () => void zoomOut({ duration: DURATION }),
    actual: () => void zoomTo(1, { duration: DURATION }),
    fit: () => void fitView({ duration: DURATION, padding: 0.1 }),
    focus: () => {
      if (targets.length > 0)
        void fitView({ nodes: targets, duration: DURATION, padding: 0.4, maxZoom: 1.6 });
    },
  };
  useZoomKeys(actions);
  useWheelZoom();
  useSharpAfterMove();
  return (
    <>
      <Panel position="bottom-left">
        <div className="flex items-center gap-0.5 rounded-control border border-line bg-panel p-0.5 shadow-panel">
          <ZoomButton
            label={td.zoomOut}
            onClick={actions.zoomOut}
            disabled={zoom <= ZOOM_RANGE.min}
          >
            <ZoomOut size={14} aria-hidden />
          </ZoomButton>
          <button
            type="button"
            onClick={actions.actual}
            title={td.zoomActual}
            aria-label={td.zoomActual}
            className="h-7 min-w-12 rounded-[7px] px-1 font-mono text-[11.5px] text-ink-2 hover:bg-sunken hover:text-ink"
          >
            {`${Math.round(zoom * 100)}%`}
          </button>
          <ZoomButton label={td.zoomIn} onClick={actions.zoomIn} disabled={zoom >= ZOOM_RANGE.max}>
            <ZoomIn size={14} aria-hidden />
          </ZoomButton>
          <span className="mx-0.5 h-4 w-px bg-line" />
          <ZoomButton label={td.zoomFit} onClick={actions.fit}>
            <Maximize size={14} aria-hidden />
          </ZoomButton>
          <ZoomButton label={td.zoomFocus} onClick={actions.focus} disabled={targets.length === 0}>
            <Focus size={14} aria-hidden />
          </ZoomButton>
          <ZoomButton label={td.minimap} onClick={() => onMinimap(!minimap)} pressed={minimap}>
            <MapIcon size={14} aria-hidden />
          </ZoomButton>
        </div>
      </Panel>
      {minimap && (
        <MiniMap
          position="top-right"
          pannable
          zoomable
          ariaLabel={td.minimap}
          nodeColor={(n) => (n.type === "card" ? uiHex.ink3 : "transparent")}
          nodeStrokeColor={(n) => (n.type === "container" ? uiHex.ink3 : "transparent")}
          maskColor={inkVeil}
          className="!rounded-control !border !border-line !bg-panel !shadow-panel"
        />
      )}
    </>
  );
}

function ZoomButton({
  label,
  onClick,
  disabled,
  pressed,
  children,
}: {
  label: string;
  onClick: () => void;
  disabled?: boolean;
  pressed?: boolean;
  children: ReactNode;
}) {
  return (
    <button
      type="button"
      title={label}
      aria-label={label}
      aria-pressed={pressed}
      disabled={disabled}
      onClick={onClick}
      className={cn(
        "flex size-7 items-center justify-center rounded-[7px] text-ink-2 hover:bg-sunken hover:text-ink disabled:pointer-events-none disabled:opacity-40",
        pressed && "bg-accent-tint text-accent-ink hover:bg-accent-tint",
      )}
    >
      {children}
    </button>
  );
}

/**
 * Ctrl / ⌘ + マウスのホイールで、カーソルの位置を中心に 1 目盛り WHEEL_STEP 倍ずつ拡大・縮小する。React Flow は
 * Ctrl 付きのホイールをトラックパッドのピンチとして扱い、Mac ではマウスの 1 目盛りで 4 倍も動いてしまうので、
 * 目盛りのあるホイール（1 回の量が大きいか、行単位）だけをここで受ける。トラックパッドのピンチ（小さな量）は
 * React Flow に任せる。ブラウザの拡大は止める。
 */
function useWheelZoom() {
  const { getViewport, setViewport } = useReactFlow();
  const domNode = useStore((s) => s.domNode);
  useEffect(() => {
    if (!domNode) return;
    const onWheel = (e: WheelEvent) => {
      if (!(e.ctrlKey || e.metaKey)) return;
      const notch = e.deltaMode === 1 || Math.abs(e.deltaY) >= 40;
      if (!notch || e.deltaY === 0) return;
      e.preventDefault();
      e.stopPropagation();
      const { x, y, zoom } = getViewport();
      const factor = e.deltaY < 0 ? WHEEL_STEP : 1 / WHEEL_STEP;
      const next = Math.min(ZOOM_RANGE.max, Math.max(ZOOM_RANGE.min, zoom * factor));
      const r = domNode.getBoundingClientRect();
      const px = e.clientX - r.left;
      const py = e.clientY - r.top;
      void setViewport({
        x: px - ((px - x) * next) / zoom,
        y: py - ((py - y) * next) / zoom,
        zoom: next,
      });
    };
    domNode.addEventListener("wheel", onWheel, { capture: true, passive: false });
    return () => domNode.removeEventListener("wheel", onWheel, { capture: true });
  }, [domNode, getViewport, setViewport]);
}

/**
 * 拡大・移動が止まって SHARPEN_AFTER たったら、図を今の拡大率で描き直させる。Chromium 系のブラウザは、動かしている間の
 * 図を拡大前の解像度の絵のまま引き伸ばし、止まっても描き直さないことがある（文字や線がガサガサになる）。動かしている
 * 間は will-change: transform で 1 枚の絵として速く動かし、止まったら外して今の拡大率で描き直させる。
 */
function useSharpAfterMove() {
  const domNode = useStore((s) => s.domNode);
  const timer = useRef<number | undefined>(undefined);
  const viewport = () => domNode?.querySelector<HTMLElement>(".react-flow__viewport") ?? null;
  useEffect(() => () => window.clearTimeout(timer.current), []);
  useOnViewportChange({
    onStart: () => {
      window.clearTimeout(timer.current);
      const v = viewport();
      if (v) v.style.willChange = "transform";
    },
    onEnd: () => {
      window.clearTimeout(timer.current);
      timer.current = window.setTimeout(() => {
        const v = viewport();
        if (v) v.style.willChange = "auto";
      }, SHARPEN_AFTER);
    },
  });
}

/** + / − / 0 / 1 / F のキー（修飾キーなし。文字を入力している欄の中では効かない） */
function useZoomKeys(
  actions: Record<"zoomIn" | "zoomOut" | "actual" | "fit" | "focus", () => void>,
) {
  const latest = useRef(actions);
  latest.current = actions;
  useEffect(() => {
    const onKey = (e: KeyboardEvent) => {
      if (e.metaKey || e.ctrlKey || e.altKey) return;
      const el = e.target instanceof HTMLElement ? e.target : null;
      if (el && (el.isContentEditable || ["INPUT", "TEXTAREA", "SELECT"].includes(el.tagName)))
        return;
      const act = {
        "+": "zoomIn",
        "=": "zoomIn",
        "-": "zoomOut",
        "0": "fit",
        "1": "actual",
        f: "focus",
      }[e.key.toLowerCase()] as keyof typeof actions | undefined;
      if (!act) return;
      e.preventDefault();
      latest.current[act]();
    };
    window.addEventListener("keydown", onKey);
    return () => window.removeEventListener("keydown", onKey);
  }, []);
}
