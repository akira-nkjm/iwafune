import { ChevronRight } from "lucide-react";
import { NavLink } from "react-router";
import { useDesign } from "@/api/queries";
import { useT } from "@/i18n";
import { cn } from "@/lib/cn";
import { type NavSection, navItemTitle, paths } from "@/lib/nav";
import { preloadHandlers, usePreloadPlan } from "../preload";
import { RailTip } from "./RailTip";
import { railFade } from "./sidebar-motion";
import { SIDEBAR_ID } from "./sidebar-state";

/**
 * ナビの区分。開いたサイドバーでは見出しを押して区分を畳める（畳んだ区分の行は描かない）。
 * レール（畳んだサイドバー）でも行の形は同じで、名前だけが薄く消える（アイコンの位置が動かない）。
 * 見出しの行も高さを保ち、レールでは短い区切り線に替わる。
 */
export function NavSectionList({
  section,
  rail,
  closed,
  onToggle,
}: {
  section: NavSection;
  rail: boolean;
  closed: boolean;
  onToggle: () => void;
}) {
  const t = useT();
  const design = useDesign();
  const preload = usePreloadPlan();
  const itemsId = `${SIDEBAR_ID}-${section.id}`;
  const heading = section.heading ? t.nav.sections[section.id] : null;
  const showItems = rail || !closed || !heading;
  return (
    <div>
      {heading && (
        <div className="relative h-6">
          <button
            type="button"
            onClick={onToggle}
            aria-expanded={!closed}
            aria-controls={itemsId}
            tabIndex={rail ? -1 : 0}
            aria-hidden={rail}
            className={cn(
              "flex h-6 w-full items-center gap-1 overflow-hidden rounded-[6px] px-1.5 font-semibold text-[10.5px] text-ink-4 tracking-[0.08em] hover:text-ink-2",
              railFade(rail),
            )}
          >
            <ChevronRight
              size={12}
              strokeWidth={2}
              aria-hidden
              className={cn(
                "shrink-0 transition-transform duration-200 motion-reduce:transition-none",
                !closed && "rotate-90",
              )}
            />
            {heading}
          </button>
          <div
            aria-hidden
            className={cn(
              "pointer-events-none absolute top-1/2 left-2 h-px w-6 bg-line transition-opacity motion-reduce:transition-none",
              rail ? "opacity-100 delay-100 duration-200" : "opacity-0 duration-100",
            )}
          />
        </div>
      )}
      {showItems && (
        <ul id={itemsId} className="space-y-0.5">
          {section.items.map((it) => {
            const Icon = it.icon;
            const name = navItemTitle(it, t, design.data?.defs);
            return (
              <li key={it.to}>
                <RailTip rail={rail} label={heading ? `${heading} / ${name}` : name}>
                  {/* className は文字列にする（RailTip の asChild は className を文字列として結合するので、
                      関数を渡すと崩れる）。選択中は NavLink が付ける aria-current="page" で見分ける。
                      ホバー・フォーカスしたら、ルートのモジュール（React Router）とページの本体・データを先に読む */}
                  <NavLink
                    to={it.to}
                    end={it.to === paths.overview()}
                    prefetch="intent"
                    {...preloadHandlers(() => preload.item(it.key))}
                    aria-label={rail ? name : undefined}
                    className={cn(
                      "flex h-8 items-center gap-2.5 overflow-hidden rounded-control border border-transparent px-[11px] text-[13px] text-ink-2 transition-colors hover:bg-panel/70 hover:text-ink",
                      // 選択中は青の濃いめの地（accent-ink。白い字とのコントラスト 4.5:1 以上）に白い字。ホバーしても色は変えない
                      "aria-[current=page]:bg-accent-ink aria-[current=page]:font-medium aria-[current=page]:text-inverse aria-[current=page]:shadow-panel aria-[current=page]:hover:bg-accent-ink aria-[current=page]:hover:text-inverse",
                    )}
                  >
                    <Icon size={16} strokeWidth={1.6} className="shrink-0" />
                    <span className={cn("text-box-cap", railFade(rail))}>{name}</span>
                  </NavLink>
                </RailTip>
              </li>
            );
          })}
        </ul>
      )}
    </div>
  );
}
