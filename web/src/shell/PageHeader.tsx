import type { ReactNode } from "react";

/** ページ上部: 22px のタイトル + 説明、右にケース切替や操作ボタン */
export function PageHeader({
  title,
  subtitle,
  actions,
  eyebrow,
}: {
  title: ReactNode;
  subtitle?: ReactNode;
  actions?: ReactNode;
  eyebrow?: ReactNode;
}) {
  return (
    <header className="flex shrink-0 flex-wrap items-end justify-between gap-3 px-6 pt-5 pb-3">
      <div className="flex min-w-0 items-end gap-3">
        {/* サイドバーの開閉はサイドバーのロゴのボタンと ⌘/Ctrl+B（ページの見出しには置かない） */}
        <div className="min-w-0">
          {eyebrow && <div className="mb-0.5 text-[11px] text-ink-3">{eyebrow}</div>}
          <h1 className="truncate font-semibold text-[22px] text-ink">{title}</h1>
          {subtitle && <p className="mt-0.5 text-ink-3 text-md">{subtitle}</p>}
        </div>
      </div>
      {actions && <div className="flex flex-wrap items-center gap-2">{actions}</div>}
    </header>
  );
}
