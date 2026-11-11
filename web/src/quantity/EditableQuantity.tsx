import { Pencil } from "lucide-react";
import { useState } from "react";
import { useT } from "@/i18n";
import { cn } from "@/lib/cn";
import { useFieldError } from "@/lib/field-errors";
import { QuantityInput } from "./QuantityInput";

/**
 * 表のセルで直接編集する量。普段は display を出し、クリックで入力欄になる。
 * 確定すると onCommit(打った文字列) を呼ぶ（呼び手が文字列のまま service に渡す）。
 */
export function EditableQuantity({
  display,
  initial,
  dimension,
  onCommit,
  label,
  disabled,
  className,
  muted,
}: {
  display: string;
  initial: string;
  dimension: string | null;
  /** 確定: 打った文字列（元の文字列と同じなら呼ばない） */
  onCommit: (text: string) => void;
  label: string;
  disabled?: boolean;
  className?: string;
  /** 未入力・カタログ値など、弱く見せる */
  muted?: boolean;
}) {
  const t = useT();
  const [editing, setEditing] = useState(false);
  // label にはパラメーターのパス（RW_X.mass）を渡す約束。検証で弾かれた理由をその欄の下に出す
  const error = useFieldError(label);
  if (editing)
    return (
      <QuantityInput
        compact
        autoFocus
        initial={initial}
        dimension={dimension}
        label={label}
        className={cn("w-36", className)}
        onCommit={(text) => {
          setEditing(false);
          if (text !== initial) onCommit(text);
        }}
        onCancel={() => setEditing(false)}
      />
    );
  const button = (
    <button
      type="button"
      disabled={disabled}
      aria-label={t.quantity.edit(label)}
      onClick={() => setEditing(true)}
      className={cn(
        "group/eq inline-flex max-w-full items-center gap-1 rounded-[6px] px-1 py-0.5 text-left hover:bg-accent-tint disabled:hover:bg-transparent",
        muted ? "text-ink-4" : "text-ink",
        className,
      )}
    >
      <span className="num min-w-0 truncate" title={display}>
        {display}
      </span>
      {!disabled && (
        <Pencil size={11} className="shrink-0 text-ink-4 opacity-0 group-hover/eq:opacity-100" />
      )}
    </button>
  );
  if (!error) return button;
  return (
    <span className="inline-flex min-w-0 flex-col">
      {button}
      <span className="px-1 text-[11px] text-neg-ink">{error}</span>
    </span>
  );
}
