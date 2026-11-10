import { cn } from "@/lib/cn";
import { subsystemTone } from "@/lib/subsystems";
import { toneBg } from "@/lib/tone";

/** サブシステム色の点（表・凡例） */
export function SubsystemDot({
  group,
  className,
}: {
  /** サブシステム（Group）の ID。無ければ灰色 */
  group: string | null;
  className?: string;
}) {
  return (
    <span
      aria-hidden="true"
      className={cn(
        "inline-block size-2 shrink-0 rounded-full",
        toneBg[subsystemTone(group)],
        className,
      )}
    />
  );
}
