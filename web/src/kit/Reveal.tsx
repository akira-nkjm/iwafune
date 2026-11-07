import { type ReactNode, useRef } from "react";
import { cn } from "@/lib/cn";
import { PANEL_MOTION } from "@/lib/motion";

/**
 * 開くと高さを滑らかに広げ、閉じると滑らかに縮める領域（保存の帯など、下書きができたときだけ出るもの）。
 * 閉じていく間は、最後に開いていたときの中身を出したままにする（中身が先に消えると高さが一気に 0 になる）。
 * 閉じている間は操作もフォーカスもできない（inert）。
 */
export function Reveal({ open, children }: { open: boolean; children: ReactNode }) {
  const last = useRef<ReactNode>(null);
  if (open) last.current = children;
  return (
    <div
      data-reveal={open ? "open" : "closed"}
      aria-hidden={!open}
      inert={!open}
      className={cn(
        "grid shrink-0 transition-[grid-template-rows,opacity]",
        PANEL_MOTION,
        open ? "grid-rows-[1fr] opacity-100" : "grid-rows-[0fr] opacity-0",
      )}
    >
      <div className="min-h-0 overflow-hidden">{open ? children : last.current}</div>
    </div>
  );
}
