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
 * 並びを揃える: アイコンの左（サイドバーの端からアイコンまで）と右（アイコンから名前まで）の空きは、どちらも 16 px。
 * アイコンは幅 16 px の枠の中央に置く（行の左端から 8 px。名前は行の左端から 40 px）。
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
        "flex h-dvh shrink-0 flex-col overflow-hidden px-2 py-4",
        SIDEBAR_MOTION,
        "transition-[width]",
        // 畳んだ幅は 64 px: ロゴ（32 px）とアイコンの正方形の枠（32 px）の左右に 16 px ずつ。
        // サイドバーの余白（8 px）は開閉で変えない。ロゴの行はいつも同じ位置（端から 16 px）で、
        // ロゴより下だけを、畳んだときに 8 px 内側へ寄せる（下の `rail ? "px-2"`）
        rail ? "w-16" : "w-sidebar",
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
            {/* プロジェクトの名前だけ（ブランチと版は下の欄に出る） */}
            <span className="truncate font-medium text-ink-2">{p?.name ?? "—"}</span>
          </div>
        </div>
      </div>
      <div
        className={cn(
          "flex min-h-0 flex-1 flex-col transition-[padding]",
          SIDEBAR_MOTION,
          rail ? "px-2" : "px-0",
        )}
      >
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
      </div>
    </aside>
  );
}
