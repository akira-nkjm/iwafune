import type { ReactNode } from "react";
import { cn } from "@/lib/cn";

/** 表の見出しセル */
export function Th({
  children,
  align = "left",
  className,
  title,
}: {
  children?: ReactNode;
  align?: "left" | "right" | "center";
  className?: string;
  title?: string;
}) {
  return (
    <th
      title={title}
      className={cn(
        "h-8 whitespace-nowrap px-3 font-medium text-[11px] text-ink-3 tracking-wide",
        align === "right" ? "text-right" : align === "center" ? "text-center" : "text-left",
        className,
      )}
    >
      {children}
    </th>
  );
}
