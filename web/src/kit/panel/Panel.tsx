import { cva, type VariantProps } from "class-variance-authority";
import type { ReactNode } from "react";

const panelVariants = cva("min-w-0 rounded-card border border-line bg-panel shadow-panel", {
  variants: { pad: { true: "p-4", false: "" } },
  defaultVariants: { pad: true },
});

/** 内側のパネル（角丸 13px、ヘアライン枠） */
export function Panel({
  className,
  children,
  pad,
}: { className?: string; children: ReactNode } & VariantProps<typeof panelVariants>) {
  return <section className={panelVariants({ pad, className })}>{children}</section>;
}
