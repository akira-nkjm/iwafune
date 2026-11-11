import { useT } from "@/i18n";
import { cn } from "@/lib/cn";
import { filledTicks } from "@/lib/ticks";

export type TickTone = "accent" | "pos" | "warn" | "neg" | "ink";

const fillClass: Record<TickTone, string> = {
  accent: "bg-accent",
  pos: "bg-pos",
  warn: "bg-warn",
  neg: "bg-neg",
  ink: "bg-ink",
};

/**
 * 目盛りバー。細い縦棒を並べ、比率ぶんを塗る。marker の位置の 1 本は濃く高くする（上限・目標）。
 * ratio が null のときは「データなし」として破線を引く（0% と区別する）。
 */
export function TickBar({
  ratio,
  tone = "accent",
  ticks = 48,
  marker,
  label,
  className,
}: {
  ratio: number | null;
  tone?: TickTone;
  ticks?: number;
  marker?: number;
  label?: string;
  className?: string;
}) {
  const t = useT();
  if (ratio === null)
    return (
      <div
        role="img"
        aria-label={label ? `${label}: ${t.common.noData}` : t.common.noData}
        title={label}
        className={cn("flex h-5 w-full items-center", className)}
      >
        <span className="w-full border-line-strong border-t-2 border-dashed" />
      </div>
    );
  const filled = filledTicks(ratio, ticks);
  const markerIdx =
    marker !== undefined ? Math.round(Math.max(0, Math.min(1, marker)) * ticks) - 1 : -1;
  return (
    // biome-ignore lint/a11y/useSemanticElements: <meter> では細い縦棒の見た目を作れない
    <div
      className={cn("flex h-5 w-full items-end gap-0.5", className)}
      role="meter"
      aria-label={label}
      title={label}
      aria-valuemin={0}
      aria-valuemax={100}
      aria-valuenow={Math.round(ratio * 100)}
    >
      {Array.from({ length: ticks }, (_, i) => (
        <span
          // biome-ignore lint/suspicious/noArrayIndexKey: 位置そのものが識別子
          key={i}
          className={cn(
            "tick",
            i < filled ? fillClass[tone] : "bg-tick",
            i === markerIdx && "h-full bg-ink",
          )}
        />
      ))}
    </div>
  );
}
