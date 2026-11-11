import type { KeyboardEvent, ReactNode, SyntheticEvent } from "react";
import { cn } from "@/lib/cn";

/**
 * 選べる表の行（`DataTable` の中の `<tr>`）。クリックか Enter・Space で `onSelect`。行そのものにフォーカスを置くので、
 * 詳細のモーダルを閉じるとこの行に戻る。選ばれている行はアクセント色の淡い地で、`aria-selected` で伝える
 */
export function SelectableRow({
  selected,
  onSelect,
  children,
}: {
  selected: boolean;
  onSelect: () => void;
  children: ReactNode;
}) {
  const open = (event: SyntheticEvent<HTMLTableRowElement>) => {
    event.currentTarget.focus();
    onSelect();
  };
  const onKeyDown = (event: KeyboardEvent<HTMLTableRowElement>) => {
    if (event.key !== "Enter" && event.key !== " ") return;
    event.preventDefault();
    open(event);
  };
  return (
    <tr
      tabIndex={0}
      aria-selected={selected}
      onClick={open}
      onKeyDown={onKeyDown}
      className={cn(
        "cursor-pointer hover:bg-panel-2",
        selected && "bg-accent-tint hover:bg-accent-tint",
      )}
    >
      {children}
    </tr>
  );
}
