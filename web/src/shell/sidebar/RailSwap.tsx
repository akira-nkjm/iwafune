import type { ReactNode } from "react";
import { cn } from "@/lib/cn";
import { SIDEBAR_MOTION } from "./sidebar-motion";

/**
 * 開いたときの中身（`full`）と、畳んだレールの中身（`compact`）を、高さと透明度を滑らかに変えて入れ替える。
 * 片方を一度に差し替えると、高さが急に変わって上の行が跳ね、開け閉めがカクッとするため。2 つを縦に重ね、
 * 片方の高さを 0 へ、もう片方を中身の高さへ、サイドバーの幅と同じ時間で動かす。見えていない方は操作できない。
 */
export function RailSwap({
  rail,
  full,
  compact,
}: {
  rail: boolean;
  full: ReactNode;
  compact: ReactNode;
}) {
  const part = (shown: boolean, children: ReactNode) => (
    <div
      aria-hidden={!shown}
      inert={!shown}
      className={cn(
        "grid transition-[grid-template-rows,opacity]",
        SIDEBAR_MOTION,
        shown ? "grid-rows-[1fr] opacity-100" : "grid-rows-[0fr] opacity-0",
      )}
    >
      <div className="min-h-0 overflow-hidden">{children}</div>
    </div>
  );
  return (
    <div>
      {part(!rail, full)}
      {part(rail, compact)}
    </div>
  );
}
