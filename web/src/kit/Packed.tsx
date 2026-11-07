import {
  Children,
  isValidElement,
  type ReactNode,
  useCallback,
  useLayoutEffect,
  useRef,
  useState,
} from "react";
import { cn } from "@/lib/cn";
import { PANEL_MOTION } from "@/lib/motion";
import { type Packed as PackedLayout, packRects } from "@/lib/skyline";

/**
 * 子（大きさのばらばらな箱）を、隙間が少なくなるよう詰めて並べる（`lib/skyline.ts`）。子の大きさと並べる幅を測り、
 * 変わるたびに置き直す。子は自分の中身の幅（max-content）で描くので、幅を決めてから中身を折り返す子には向かない。
 * 幅が変わって置き直すときは位置を滑らかに動かす。
 */
export function Packed({
  gap,
  className,
  children,
}: {
  gap: number;
  className?: string;
  children: ReactNode;
}) {
  const items = Children.toArray(children);
  const outer = useRef<HTMLDivElement>(null);
  const boxes = useRef<(HTMLDivElement | null)[]>([]);
  const [layout, setLayout] = useState<PackedLayout | null>(null);
  const placed = useRef(false);
  const measure = useCallback(() => {
    const width = outer.current?.clientWidth ?? 0;
    if (!width) return;
    const sizes = boxes.current
      .slice(0, items.length)
      .map((el) => ({ width: el?.offsetWidth ?? 0, height: el?.offsetHeight ?? 0 }));
    const next = packRects(sizes, width, gap);
    setLayout((prev) => (prev && JSON.stringify(prev) === JSON.stringify(next) ? prev : next));
  }, [gap, items.length]);
  // 描く前に測って置く（中身が変われば毎回）
  useLayoutEffect(measure);
  // 並べる幅（詳細の欄の開け閉め・窓の大きさ）と子の大きさ（文字の読み込みなど）の変化でも置き直す
  useLayoutEffect(() => {
    const observer = new ResizeObserver(() => measure());
    if (outer.current) observer.observe(outer.current);
    for (const el of boxes.current.slice(0, items.length)) if (el) observer.observe(el);
    return () => observer.disconnect();
  }, [measure, items.length]);
  // 最初に置くときは動かさず、置き直すときだけ動かす
  const animate = placed.current;
  if (layout) placed.current = true;
  return (
    <div
      ref={outer}
      className={cn("relative", className)}
      style={{ height: layout?.height ?? undefined }}
    >
      {items.map((child, i) => {
        const spot = layout?.spots[i];
        return (
          <div
            key={isValidElement(child) ? child.key : i}
            ref={(el) => {
              boxes.current[i] = el;
            }}
            className={cn(
              "absolute top-0 left-0 w-max",
              !spot && "invisible",
              animate && `transition-transform ${PANEL_MOTION}`,
            )}
            style={{ transform: spot ? `translate(${spot.x}px, ${spot.y}px)` : undefined }}
          >
            {child}
          </div>
        );
      })}
    </div>
  );
}
