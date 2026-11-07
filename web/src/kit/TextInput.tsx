import type { InputHTMLAttributes } from "react";
import { cn } from "@/lib/cn";

/** 1 行の文字入力（ID・ラベルなど）。エラーは枠の色と、Field の文言の両方で示す */
export function TextInput({
  invalid,
  mono,
  className,
  ...rest
}: InputHTMLAttributes<HTMLInputElement> & { invalid?: boolean; mono?: boolean }) {
  return (
    <input
      {...rest}
      aria-invalid={invalid ? true : undefined}
      spellCheck={false}
      autoComplete="off"
      className={cn(
        "h-8 w-full min-w-0 rounded-[8px] border bg-panel px-2.5 text-[13px] outline-none focus-visible:ring-2 focus-visible:ring-accent/40",
        invalid ? "border-neg" : "border-line-strong",
        mono && "font-mono text-[12.5px]",
        className,
      )}
    />
  );
}
