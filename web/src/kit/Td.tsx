import type { ReactNode } from "react";
import { cn } from "@/lib/cn";

/** 表のセル。数値は右寄せ・等幅 */
export function Td({
  children,
  align = "left",
  className,
  colSpan,
  mono,
}: {
  children?: ReactNode;
  align?: "left" | "right" | "center";
  className?: string;
  colSpan?: number;
  mono?: boolean;
}) {
  return (
    <td
      colSpan={colSpan}
      className={cn(
        "whitespace-nowrap px-3 py-1.5 align-middle text-md",
        align === "right" && "num text-right",
        align === "center" && "text-center",
        mono && "font-mono text-[12px]",
        className,
      )}
    >
      {children}
    </td>
  );
}
