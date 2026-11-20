import { useQuery } from "@tanstack/react-query";
import { q, useDesign } from "@/api/queries";
import { useT } from "@/i18n";
import { cn } from "@/lib/cn";
import { nav } from "@/lib/nav";
import { CommitChip } from "../CommitChip";
import { LanguageSwitch } from "../LanguageSwitch";
import { BrandToggle } from "./BrandToggle";
import { CommandSearch } from "./CommandSearch";
import { NavSectionList } from "./NavSectionList";
import { RailSwap } from "./RailSwap";
import { useSidebar } from "./SidebarContext";
import { SIDEBAR_MOTION } from "./sidebar-motion";
import { SIDEBAR_ID } from "./sidebar-state";

/**
 * 左のサイドバー。開いているときはロゴ・プロジェクト・検索・区分付きのナビ・CommitChip。
 * 畳む（⌘/Ctrl+B かロゴのボタン）とアイコンのレールになり、メインのパネルが広がる。
 * 並びは 32 px の枠で揃える: アイコンは左端から 8 px、アイコンと名前の間も 8 px（名前は 32 px から始まる）。
 * 畳むと、どの項目も 32 px 四方の枠になる。
 * 開閉は幅だけを動かし、アイコンの位置はどちらでも同じにして、名前は薄く消す（中身を差し替えないので跳ねない）。
 */
export function Sidebar() {
  const t = useT();
  const design = useDesign();
  const project = useQuery(q.project());
  const { open, toggle, closedSections, toggleSection } = useSidebar();
  const rail = !open;

  const p = project.data;
  return (
    <aside
      id={SIDEBAR_ID}
      className={cn(
        "flex h-dvh shrink-0 flex-col overflow-hidden px-3 py-4",
        SIDEBAR_MOTION,
        "transition-[width]",
        // 畳んだ幅は、左右の余白 12 px と、アイコンの正方形の枠 32 px
        rail ? "w-14" : "w-sidebar",
      )}
    >
      <BrandToggle rail={rail} onToggle={toggle} />
      {/* プロジェクト名の行は高さごと畳む（下の行が跳ねないよう、高さを滑らかに変える） */}
      <div
        aria-hidden={rail}
        className={cn(
          "grid transition-[grid-template-rows,opacity]",
          SIDEBAR_MOTION,
          rail ? "grid-rows-[0fr] opacity-0" : "grid-rows-[1fr] opacity-100",
        )}
      >
        <div className="min-h-0 overflow-hidden">
          <div className="flex items-center gap-1.5 whitespace-nowrap px-2 pt-1 pb-3 text-[11.5px] text-ink-3">
            <span className="truncate font-medium text-ink-2">{p?.name ?? "—"}</span>
            <span className="truncate font-mono">{p?.branch ?? ""}</span>
          </div>
        </div>
      </div>
      <CommandSearch rail={rail} />
      <nav
        aria-label={t.nav.main}
        className="no-scrollbar min-h-0 flex-1 space-y-2 overflow-y-auto"
      >
        {nav(design.data?.defs.systems ?? []).map((sec) => (
          <NavSectionList
            key={sec.id}
            section={sec}
            rail={rail}
            closed={closedSections.has(sec.id)}
            onToggle={() => toggleSection(sec.id)}
          />
        ))}
      </nav>
      {/* 下の欄は開いたときと畳んだときで中身が違うので、高さを滑らかに変えて入れ替える（RailSwap） */}
      <div className="mt-3 border-line border-t pt-3">
        <RailSwap
          rail={rail}
          full={
            <div className="space-y-2">
              <CommitChip />
              <div className="px-2">
                <LanguageSwitch />
              </div>
            </div>
          }
          compact={
            <div className="space-y-2">
              <CommitChip rail />
              <LanguageSwitch rail />
            </div>
          }
        />
      </div>
    </aside>
  );
}
