import type { SelectHTMLAttributes } from "react";
import { cn } from "@/lib/cn";

export interface SelectOption {
  value: string;
  label: string;
}

/** ネイティブの select（表のセルでも軽い）。空の選択肢を出すときは placeholder を渡す */
export function NativeSelect({
  options,
  placeholder,
  className,
  size = "md",
  ...rest
}: Omit<SelectHTMLAttributes<HTMLSelectElement>, "size"> & {
  options: readonly SelectOption[];
  placeholder?: string;
  size?: "sm" | "md";
}) {
  return (
    <select
      {...rest}
      className={cn(
        "min-w-0 rounded-[8px] border border-line-strong bg-panel pr-6 pl-2 text-[13px] outline-none focus-visible:ring-2 focus-visible:ring-accent/40",
        size === "sm" ? "h-7 text-[12.5px]" : "h-8",
        className,
      )}
    >
      {placeholder !== undefined && <option value="">{placeholder}</option>}
      {options.map((o) => (
        <option key={o.value} value={o.value}>
          {o.label}
        </option>
      ))}
    </select>
  );
}
