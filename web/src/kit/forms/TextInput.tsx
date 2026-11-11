import type { ComponentProps } from "react";
import { cn } from "@/lib/cn";

/** TextInput の Props（`size` は欄の高さ。HTML の `size` 属性は使わない。`ref` は input に渡る） */
export type TextInputProps = Omit<ComponentProps<"input">, "size"> & {
  invalid?: boolean;
  mono?: boolean;
  /** `md`（既定。ダイアログの欄）か `sm`（表や詳細の行の中の欄） */
  size?: "sm" | "md";
};

/**
 * 1 行の文字入力（ID・ラベルなど）。エラーは枠の色と、Field の文言の両方で示す。入力欄の見た目（高さ・角丸・枠・
 * フォーカスの輪）はここだけで決める（検索欄 `SearchInput`・確定する欄 `CommitInput`・量の欄 `QuantityInput` もこれを使う）
 */
export function TextInput({ invalid, mono, size = "md", className, ...rest }: TextInputProps) {
  return (
    <input
      {...rest}
      aria-invalid={invalid ? true : undefined}
      spellCheck={false}
      autoComplete="off"
      className={cn(
        "w-full min-w-0 rounded-[8px] border bg-panel outline-none focus-visible:ring-2 focus-visible:ring-accent/40",
        size === "sm" ? "h-7 px-2 text-[12.5px]" : "h-8 px-2.5 text-[13px]",
        invalid ? "border-neg" : "border-line-strong",
        mono && (size === "sm" ? "font-mono" : "font-mono text-[12.5px]"),
        className,
      )}
    />
  );
}
