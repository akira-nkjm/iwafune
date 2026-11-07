import type { ReactNode } from "react";
import { cn } from "@/lib/cn";

/** パネルの見出し（左にタイトルと説明、右に操作） */
export function PanelHeader({
  title,
  caption,
  right,
  className,
}: {
  title: ReactNode;
  caption?: ReactNode;
  right?: ReactNode;
  className?: string;
}) {
  return (
    <div className={cn("mb-3 flex items-start justify-between gap-3", className)}>
      <div className="min-w-0">
        <h2 className="font-semibold text-ink text-md">{title}</h2>
        {caption && <p className="mt-0.5 text-ink-3 text-xs">{caption}</p>}
      </div>
      {right && <div className="flex max-w-full shrink-0 items-center gap-2">{right}</div>}
    </div>
  );
}
