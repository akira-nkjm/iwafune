import { NavLink } from "react-router";
import { useT } from "@/i18n";
import { cn } from "@/lib/cn";

export interface PageTab {
  to: string;
  label: string;
  count?: number;
  /** 選ばれているか（URL の検索部分で切り替えるタブ用。無ければ道筋で決める） */
  active?: boolean;
}

/** ページ内のタブ（下線）。URL（道筋か `?view=` などの検索部分）に載るので、タブを開いたまま共有・再読み込みできる */
export function PageTabs({ tabs, className }: { tabs: readonly PageTab[]; className?: string }) {
  return (
    <nav
      aria-label={useT().shell.tabs}
      className={cn(
        "flex gap-1 overflow-x-auto overflow-y-hidden border-line border-b px-6",
        className,
      )}
    >
      {tabs.map((t) => (
        <NavLink
          key={t.to}
          to={t.to}
          end
          className={({ isActive }) =>
            cn(
              "-mb-px flex h-9 items-center gap-1.5 whitespace-nowrap border-b-2 px-2.5 text-[13px] transition-colors",
              (t.active ?? isActive)
                ? "border-accent font-medium text-ink"
                : "border-transparent text-ink-3 hover:text-ink",
            )
          }
        >
          {t.label}
          {t.count !== undefined && (
            <span className="num rounded-full bg-panel-2 px-1.5 text-2xs text-ink-3">
              {t.count}
            </span>
          )}
        </NavLink>
      ))}
    </nav>
  );
}
