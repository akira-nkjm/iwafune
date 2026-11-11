import { Switch as Primitive } from "radix-ui";
import type { ReactNode } from "react";
import { cn } from "@/lib/cn";

/**
 * オン・オフのスイッチ（チェックボックスの代わり）。オフは灰色の溝、オンはアクセント色の溝で、白いつまみが左右に動く。
 * `children` を渡すと右に文字を添え、文字を押しても切り替わる。文字を出さないときは `aria-label` を渡す。
 * いくつかの中から選ぶ切り替え（系統図の層など）はボタン型の Toggle を使う。
 */
export function Switch({
  value,
  onChange,
  children,
  title,
  className,
  "aria-label": ariaLabel,
}: {
  /** オンか */
  value: boolean;
  onChange: (value: boolean) => void;
  children?: ReactNode;
  title?: string;
  className?: string;
  "aria-label"?: string;
}) {
  const track = (
    <Primitive.Root
      checked={value}
      onCheckedChange={onChange}
      aria-label={ariaLabel}
      title={children ? undefined : title}
      className={cn(
        "relative inline-flex h-4 w-7 shrink-0 items-center rounded-full outline-none transition-colors",
        "focus-visible:ring-2 focus-visible:ring-accent/40",
        value ? "bg-accent" : "bg-line-strong",
        // Noto Sans JP の字面は行の箱の中心より 0.5px ほど下に来るので、文字を添えるときは溝も同じだけ下げて中心を揃える
        children && "translate-y-[0.5px]",
        !children && className,
      )}
    >
      <Primitive.Thumb
        className={cn(
          "block size-3 rounded-full bg-panel shadow-panel transition-transform",
          value ? "translate-x-3.5" : "translate-x-0.5",
        )}
      />
    </Primitive.Root>
  );
  if (!children) return track;
  return (
    // biome-ignore lint/a11y/noLabelWithoutControl: Radix のスイッチは button なので、label で包んで文字でも押せるようにする
    <label
      title={title}
      className={cn(
        "inline-flex shrink-0 cursor-pointer select-none items-center gap-1.5 font-medium text-ink-2 text-xs",
        className,
      )}
    >
      {track}
      {children}
    </label>
  );
}
