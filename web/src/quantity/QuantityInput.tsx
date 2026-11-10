import { useEffect, useId, useRef, useState } from "react";
import type { QuantityView } from "@/api/types";
import { useT } from "@/i18n";
import { TextInput } from "@/kit/TextInput";
import { cn } from "@/lib/cn";
import { withNote } from "@/lib/design";
import { useParseQuantity } from "./use-parse";

/**
 * 単位付きの量の入力欄。打鍵ごとに /units/parse で解析し、SI での値（`= 0.018 N*m*s`）か次元のエラーを
 * 入力欄の下に出す。Enter（または onBlurCommit で欄を離れたとき）で確定し、文字列のまま onCommit に渡す。
 * 次元が合わないとき・解析できないときは確定させない。
 */
export function QuantityInput({
  initial,
  dimension,
  onCommit,
  onCancel,
  autoFocus,
  label,
  className,
  compact,
  onChangeValid,
}: {
  initial: string;
  /** 期待する次元（例: "Power"）。null なら次元を問わない */
  dimension: string | null;
  /** 確定: 入力文字列と、/units/parse が読んだ値（書かれたままの数値と単位） */
  onCommit?: (text: string, q: QuantityView) => void;
  onCancel?: () => void;
  autoFocus?: boolean;
  label: string;
  className?: string;
  /** 表のセル向け（説明行を浮かせる） */
  compact?: boolean;
  /** 入力の妥当性が変わったとき（ダイアログの確定ボタンに使う） */
  onChangeValid?: (text: string, valid: boolean, q: QuantityView | null) => void;
}) {
  const t = useT();
  const [text, setText] = useState(initial);
  const { parsed, error, pending, empty } = useParseQuantity(text, dimension);
  const id = useId();
  const ref = useRef<HTMLInputElement>(null);
  const valid = !empty && !pending && !error && parsed !== null;

  useEffect(() => {
    if (autoFocus) {
      ref.current?.focus();
      ref.current?.select();
    }
  }, [autoFocus]);
  useEffect(() => {
    onChangeValid?.(text, valid, valid ? (parsed?.quantity ?? null) : null);
  }, [text, valid, parsed, onChangeValid]);

  const commit = () => {
    if (valid && parsed) onCommit?.(text.trim(), parsed.quantity);
  };

  return (
    <div className={cn("relative min-w-0", className)}>
      <TextInput
        ref={ref}
        id={id}
        size="sm"
        mono
        invalid={!!error}
        aria-label={label}
        aria-describedby={`${id}-echo`}
        value={text}
        onChange={(e) => setText(e.target.value)}
        onKeyDown={(e) => {
          if (e.key === "Enter") {
            e.preventDefault();
            commit();
          } else if (e.key === "Escape") {
            e.preventDefault();
            onCancel?.();
          }
        }}
      />
      <div
        id={`${id}-echo`}
        aria-live="polite"
        className={cn(
          "mt-0.5 truncate text-[11px]",
          compact &&
            "absolute top-full left-0 z-20 rounded-chip border border-line bg-panel px-1.5 py-0.5 shadow-pop",
          error ? "text-neg-ink" : "text-ink-3",
          compact && empty && !error && "hidden",
        )}
      >
        {error
          ? error
          : pending
            ? t.quantity.parsing
            : parsed
              ? `= ${withNote(parsed.si.display, parsed.quantity.dimension)}`
              : dimension
                ? t.quantity.dimension(dimension)
                : t.quantity.example}
      </div>
    </div>
  );
}
