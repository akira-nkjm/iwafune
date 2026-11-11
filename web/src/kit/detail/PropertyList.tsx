import type { ReactNode } from "react";
import { cn } from "@/lib/cn";

/** ラベルと値の行（PropertyRow）を並べる枠。行の間にヘアライン */
export function PropertyList({ children, className }: { children: ReactNode; className?: string }) {
  return (
    <div className={cn("divide-y divide-line-soft rounded-control border border-line", className)}>
      {children}
    </div>
  );
}
