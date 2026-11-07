import { cva, type VariantProps } from "class-variance-authority";
import type { ReactNode } from "react";

const pillVariants = cva(
  "inline-flex items-center gap-1.5 whitespace-nowrap rounded-chip border px-2 py-0.5 font-medium text-[11px] before:size-1.5 before:rounded-full before:bg-current",
  {
    variants: {
      tone: {
        neutral: "border-line-strong bg-panel-2 text-ink-3",
        pos: "border-pos/30 bg-pos-bg text-pos-ink",
        warn: "border-warn/50 bg-warn-bg text-warn-ink",
        neg: "border-neg/30 bg-neg-bg text-neg-ink",
        accent: "border-accent/30 bg-accent-tint text-accent-ink",
      },
      dot: { true: "", false: "before:hidden" },
    },
    defaultVariants: { tone: "neutral", dot: true },
  },
);

export type PillTone = "neutral" | "pos" | "warn" | "neg" | "accent";

/** 状態ピル（点 + 文言。色だけで状態を伝えない） */
export function Pill({
  tone,
  dot,
  children,
  title,
}: { children: ReactNode; title?: string } & VariantProps<typeof pillVariants>) {
  return (
    <span className={pillVariants({ tone, dot })} title={title}>
      {children}
    </span>
  );
}
