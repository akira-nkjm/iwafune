import type { ReactNode } from "react";
import { cn } from "@/lib/cn";

/** 密な表（ヘッダ帯は panel-2、行はヘアライン）。行は <tr>、セルは Th / Td */
export function DataTable({
  head,
  children,
  className,
  stickyHead,
}: {
  head: ReactNode;
  children: ReactNode;
  className?: string;
  /** スクロールする器の中でヘッダを固定する */
  stickyHead?: boolean;
}) {
  return (
    <div className={cn("min-w-0 overflow-auto", className)}>
      <table className="w-full caption-bottom border-collapse text-md">
        <thead className={cn(stickyHead && "sticky top-0 z-10")}>
          <tr className="border-line border-y bg-panel-2">{head}</tr>
        </thead>
        <tbody className="[&_tr]:border-line-soft [&_tr]:border-b [&_tr:last-child]:border-b-0">
          {children}
        </tbody>
      </table>
    </div>
  );
}
