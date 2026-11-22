import type { ComponentProps, ReactNode } from "react";
import { cn } from "@/lib/cn";

/** 図・3D の視点操作（拡大・縮小・全体など）のボタンを並べる枠。縁と影のある小さな帯 */
export function ViewToolbar({ className, ...props }: ComponentProps<"div">) {
  return (
    <div
      role="toolbar"
      className={cn(
        "flex items-center gap-0.5 rounded-control border border-line bg-panel p-0.5 shadow-panel",
        className,
      )}
      {...props}
    />
  );
}

/** 視点操作の正方形のボタン（アイコン 1 つ。名前は label で aria-label とツールチップに使う。Radix の asChild にも渡せる） */
export function ViewButton({
  label,
  pressed,
  className,
  children,
  ...props
}: Omit<ComponentProps<"button">, "children" | "title" | "aria-label"> & {
  label: string;
  pressed?: boolean;
  children: ReactNode;
}) {
  return (
    <button
      type="button"
      title={label}
      aria-label={label}
      aria-pressed={pressed}
      className={cn(
        "flex size-7 items-center justify-center rounded-[7px] text-ink-2 hover:bg-sunken hover:text-ink focus-visible:outline-2 focus-visible:outline-accent disabled:pointer-events-none disabled:opacity-40",
        pressed && "bg-accent-tint text-accent-ink hover:bg-accent-tint",
        className,
      )}
      {...props}
    >
      {children}
    </button>
  );
}
