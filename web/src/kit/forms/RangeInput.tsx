import type { ComponentProps } from "react";
import { cn } from "@/lib/cn";

/** 連続値のスライダー。ブラウザーの数値をそのまま返す。 */
export function RangeInput({
  onChange,
  className,
  ...rest
}: Omit<ComponentProps<"input">, "type" | "onChange"> & {
  onChange: (value: number) => void;
}) {
  return (
    <input
      {...rest}
      type="range"
      onChange={(e) => onChange(e.currentTarget.valueAsNumber)}
      className={cn(
        "h-7 min-w-0 cursor-pointer rounded-control accent-accent outline-none focus-visible:ring-2 focus-visible:ring-accent/40",
        className,
      )}
    />
  );
}
