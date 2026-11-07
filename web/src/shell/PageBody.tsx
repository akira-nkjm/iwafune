import type { ReactNode } from "react";
import { cn } from "@/lib/cn";

/** ページの本文（スクロールする器）。Panel のグリッドを入れる */
export function PageBody({ children, className }: { children: ReactNode; className?: string }) {
  return (
    <div className={cn("min-h-0 flex-1 space-y-4 overflow-y-auto px-6 py-4", className)}>
      {children}
    </div>
  );
}
