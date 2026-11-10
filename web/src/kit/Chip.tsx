import type { ReactNode } from "react";
import { Link } from "react-router";
import { cn } from "@/lib/cn";
import { PRESSED } from "./pressed";

const CHIP = "rounded-chip border px-1.5 py-0.5 text-[11px]";
const IDLE = "border-line hover:bg-panel-2";

/**
 * 選べる ID のチップ（小さなボタン）。押すと `onSelect`。`selected` を渡すと切り替えのボタンになり、`aria-pressed` で
 * 状態を伝え、選ばれているときは `Toggle` と同じ見た目（アクセント色の淡い地と枠）。渡さなければただの操作のボタン。
 * 中身は既定で等幅（ID）。名前を出すときは `mono={false}`
 */
export function Chip({
  selected,
  onSelect,
  mono = true,
  title,
  children,
}: {
  selected?: boolean;
  onSelect: () => void;
  mono?: boolean;
  title?: string;
  children: ReactNode;
}) {
  return (
    <button
      type="button"
      aria-pressed={selected}
      title={title}
      onClick={onSelect}
      className={cn(CHIP, mono && "font-mono", selected ? PRESSED : IDLE)}
    >
      {children}
    </button>
  );
}

/** 開き先（ページ）のある ID のチップ。見た目は選ばれていない `Chip` と同じ */
export function ChipLink({
  to,
  mono = true,
  title,
  children,
}: {
  to: string;
  mono?: boolean;
  title?: string;
  children: ReactNode;
}) {
  return (
    <Link to={to} title={title} className={cn(CHIP, mono && "font-mono", IDLE)}>
      {children}
    </Link>
  );
}
