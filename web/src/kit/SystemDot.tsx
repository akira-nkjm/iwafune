import type { System } from "@/api/types";
import { cn } from "@/lib/cn";
import { systemTone } from "@/lib/systems";
import { toneBg } from "@/lib/tone";

/** 系の色の点（表・凡例） */
export function SystemDot({
  system,
  className,
}: {
  /** API の系。無ければ灰色 */
  system: System | null;
  className?: string;
}) {
  return (
    <span
      aria-hidden="true"
      className={cn(
        "inline-block size-2 shrink-0 rounded-full",
        toneBg[systemTone(system)],
        className,
      )}
    />
  );
}
