import type { ReactNode } from "react";
import { cn } from "@/lib/cn";

/** 空のとき・読み込み中のときの文言 */
export function Empty({ children, className }: { children: ReactNode; className?: string }) {
  return <div className={cn("py-10 text-center text-ink-3 text-sm", className)}>{children}</div>;
}
