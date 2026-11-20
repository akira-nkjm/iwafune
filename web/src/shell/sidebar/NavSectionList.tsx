import { ChevronRight } from "lucide-react";
import { matchPath, NavLink, useLocation, useNavigation } from "react-router";
import { useDesign } from "@/api/queries";
import { useT } from "@/i18n";
import { cn } from "@/lib/cn";
import { type NavSection, navItemTitle, paths } from "@/lib/nav";
import { preloadHandlers, usePreloadPlan } from "../preload";
import { RailTip } from "./RailTip";
import { railFade, railItem } from "./sidebar-motion";
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
  // いま向かっているページ（移っている途中なら行き先）。押した直後に選択中の色を移すために、行き先で見分ける。
  // ページの切り替えが終わるのを待つと、重いページでは色だけ前の項目に残って見える
  const location = useLocation();
  const goingTo = useNavigation().location?.pathname ?? location.pathname;
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
              // 矢印（12 px）はアイコンの列（幅 16 px）の中央に置き、見出しの字は項目の名前と同じ位置から始める
              "flex h-6 w-full items-center gap-[14px] overflow-hidden rounded-[6px] pr-1.5 pl-[18px] font-semibold text-[10.5px] text-ink-4 tracking-[0.08em] hover:text-ink-2",
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
              "pointer-events-none absolute top-1/2 left-3 h-px w-6 bg-line transition-opacity motion-reduce:transition-none",
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
                      関数を渡すと崩れる）。選択中の色は `data-current`（行き先で決める。上の `goingTo`）で付ける。読み上げ用の aria-current は NavLink が付ける。
                      ホバー・フォーカスしたら、ルートのモジュール（React Router）とページの本体・データを先に読む */}
                  <NavLink
                    to={it.to}
                    end={it.to === paths.overview()}
                    prefetch="intent"
                    {...preloadHandlers(() => preload.item(it.key))}
                    aria-label={rail ? name : undefined}
                    data-current={
                      matchPath({ path: it.to, end: it.to === paths.overview() }, goingTo) !== null
                    }
                    // 色は滑らかに変えない（transition を付けない）。重いページを開く間は画面の描き直しが止まるので、
                    // 色の変化が途中で止まり、前の項目の字が白いまま地だけ消える、といった半端な見た目になる
                    className={cn(
                      "flex h-8 items-center gap-3 overflow-hidden rounded-control text-[13px] text-ink-2 hover:bg-panel/70 hover:text-ink",
                      railItem(rail),
                      // 選択中は青の濃いめの地（accent-ink。白い字とのコントラスト 4.5:1 以上）に白い字。ホバーしても色は変えない
                      "data-[current=true]:bg-accent-ink data-[current=true]:font-medium data-[current=true]:text-inverse data-[current=true]:shadow-panel data-[current=true]:hover:bg-accent-ink data-[current=true]:hover:text-inverse",
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
