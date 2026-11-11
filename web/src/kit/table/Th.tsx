import type { ReactNode } from "react";
import { useT } from "@/i18n";
import { cn } from "@/lib/cn";
import type { SortHeaderProps } from "./useTableSort";

/** 表の見出しセル */
export function Th({
  children,
  align = "left",
  className,
  title,
  sort,
}: {
  children?: ReactNode;
  align?: "left" | "right" | "center";
  className?: string;
  title?: string;
  sort?: SortHeaderProps;
}) {
  const t = useT().common;
  return (
    <th
      title={title}
      aria-sort={sort?.direction}
      className={cn(
        "h-8 whitespace-nowrap px-3 font-medium text-[11px] text-ink-3 tracking-wide",
        align === "right" ? "text-right" : align === "center" ? "text-center" : "text-left",
        className,
      )}
    >
      {sort ? (
        <button
          type="button"
          onClick={sort.onSort}
          title={title ?? t.sortColumn}
          className="inline-flex w-full items-center gap-1 rounded-sm focus-visible:outline-2 focus-visible:outline-accent"
          style={{
            justifyContent:
              align === "right" ? "flex-end" : align === "center" ? "center" : "flex-start",
          }}
        >
          {children}
          <span aria-hidden="true">
            {sort.direction === "ascending" ? "↑" : sort.direction === "descending" ? "↓" : "↕"}
          </span>
        </button>
      ) : (
        children
      )}
    </th>
  );
}
