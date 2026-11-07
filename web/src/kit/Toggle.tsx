import type { ReactNode } from "react";
import { cn } from "@/lib/cn";

/**
 * オン・オフを切り替えるボタン（チェックボックスの代わり）。押されているときはアクセント色の淡い地と枠で、
 * 系統図の層の切り替え・Segmented と同じ高さと角丸。
 */
export function Toggle({
  on,
  onChange,
  children,
  title,
  className,
}: {
  on: boolean;
  onChange: (on: boolean) => void;
  children: ReactNode;
  title?: string;
  className?: string;
}) {
  return (
    <button
      type="button"
      aria-pressed={on}
      title={title}
      onClick={() => onChange(!on)}
      className={cn(
        "h-7 shrink-0 rounded-[8px] border px-2.5 font-medium text-xs transition-colors",
        on
          ? "border-accent bg-accent-tint text-accent-ink"
          : "border-line bg-panel text-ink-3 hover:text-ink",
        className,
      )}
    >
      {children}
    </button>
  );
}
