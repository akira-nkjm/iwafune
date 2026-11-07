import type { ReactNode } from "react";

/** 詳細パネルの区分の見出し + 中身 */
export function DetailSection({
  title,
  right,
  children,
}: {
  title: string;
  right?: ReactNode;
  children: ReactNode;
}) {
  return (
    <section className="space-y-1.5" aria-label={title}>
      <div className="flex items-center justify-between gap-2 px-0.5">
        <h3 className="font-semibold text-[12.5px] text-ink">{title}</h3>
        {right}
      </div>
      {children}
    </section>
  );
}
