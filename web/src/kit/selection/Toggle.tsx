import type { ReactNode } from "react";
import { cn } from "@/lib/cn";
import { PRESSED } from "./pressed";

/**
 * オン・オフを切り替えるボタン（チェックボックスの代わり）。押されているときはアクセント色の淡い地と枠で、
 * 系統図の層の切り替え・Segmented と同じ高さと角丸。
 */
export function Toggle({
  value,
  onChange,
  children,
  title,
  className,
}: {
  /** 押されているか（オン） */
  value: boolean;
  onChange: (value: boolean) => void;
  children: ReactNode;
  title?: string;
  className?: string;
}) {
  return (
    <button
      type="button"
      aria-pressed={value}
      title={title}
      onClick={() => onChange(!value)}
      className={cn(
        "h-7 shrink-0 rounded-[8px] border px-2.5 font-medium text-xs transition-colors",
        value ? PRESSED : "border-line bg-panel text-ink-3 hover:text-ink",
        className,
      )}
    >
      {children}
    </button>
  );
}
