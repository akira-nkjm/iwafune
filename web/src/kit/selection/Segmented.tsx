import { cn } from "@/lib/cn";

/** 2〜5 個の選択肢の切り替え（溝の上に選ばれた面が浮く） */
export function Segmented<T extends string>({
  options,
  value,
  onChange,
  size = "md",
  className,
  label,
}: {
  options: readonly { value: T; label: string }[];
  value: T | null;
  onChange: (v: T) => void;
  size?: "sm" | "md";
  className?: string;
  label: string;
}) {
  return (
    <div
      role="radiogroup"
      aria-label={label}
      className={cn("inline-flex rounded-control border border-line bg-sunken p-0.5", className)}
    >
      {options.map((o) => (
        // biome-ignore lint/a11y/useSemanticElements: 見た目を揃えたボタン型のラジオ
        <button
          type="button"
          role="radio"
          aria-checked={o.value === value}
          key={o.value}
          onClick={() => onChange(o.value)}
          className={cn(
            "rounded-[8px] font-medium text-ink-3 transition-colors hover:text-ink",
            size === "sm" ? "h-6 px-2 text-2xs" : "h-7 px-2.5 text-xs",
            o.value === value && "bg-panel text-ink shadow-panel ring-1 ring-line",
          )}
        >
          {o.label}
        </button>
      ))}
    </div>
  );
}
