import type { ReactNode } from "react";
import { cn } from "@/lib/cn";

/** ラベル + 入力 + 補足 / エラー（エラーは文言でも示す） */
export function Field({
  label,
  htmlFor,
  hint,
  error,
  children,
  className,
}: {
  label: string;
  htmlFor?: string;
  hint?: ReactNode;
  error?: string | null;
  children: ReactNode;
  className?: string;
}) {
  return (
    <div className={cn("grid min-w-0 content-start gap-1", className)}>
      <label htmlFor={htmlFor} className="font-medium text-ink-2 text-xs">
        {label}
      </label>
      {children}
      {error ? (
        <p className="text-[11px] text-neg-ink">{error}</p>
      ) : hint ? (
        <p className="text-[11px] text-ink-3">{hint}</p>
      ) : null}
    </div>
  );
}
