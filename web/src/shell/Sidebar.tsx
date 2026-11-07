import { useQuery } from "@tanstack/react-query";
import { Plus } from "lucide-react";
import { useEffect } from "react";
import { useNavigate } from "react-router";
import { q } from "@/api/queries";
import { useT } from "@/i18n";
import { cn } from "@/lib/cn";
import { nav } from "@/lib/nav";
import { Kbd } from "@/ui/kbd";
import { BrandToggle } from "./BrandToggle";
import { CommandSearch } from "./CommandSearch";
import { CommitChip } from "./CommitChip";
import { LanguageSwitch } from "./LanguageSwitch";
import { NavSectionList } from "./NavSectionList";
import { RailSwap } from "./RailSwap";
import { RailTip } from "./RailTip";
import { useSidebar } from "./SidebarContext";
import { railFade, SIDEBAR_MOTION } from "./sidebar-motion";
import { SIDEBAR_ID } from "./sidebar-state";

/**
 * 左のサイドバー。開いているときはロゴ・プロジェクト・主ボタン・検索・区分付きのナビ・CommitChip。
 * 畳む（⌘/Ctrl+B かロゴのボタン）とアイコンのレールになり、メインのパネルが広がる。
 * 開閉は幅だけを動かし、アイコンの位置はどちらでも同じにして、名前は薄く消す（中身を差し替えないので跳ねない）。
 */
export function Sidebar() {
  const t = useT();
  const project = useQuery(q.project());
  const navigate = useNavigate();
  const { open, toggle, closedSections, toggleSection } = useSidebar();
  const addPart = () => navigate("/design/components?new=1");
  const rail = !open;

  useEffect(() => {
    const onKey = (e: KeyboardEvent) => {
      const t = e.target as HTMLElement | null;
      if (t && (t.isContentEditable || /^(INPUT|TEXTAREA|SELECT)$/.test(t.tagName))) return;
      if (e.key === "n" && !e.metaKey && !e.ctrlKey && !e.altKey) {
        e.preventDefault();
        navigate("/design/components?new=1");
      }
    };
    window.addEventListener("keydown", onKey);
    return () => window.removeEventListener("keydown", onKey);
  }, [navigate]);

  const p = project.data;
  return (
    <aside
      id={SIDEBAR_ID}
      className={cn(
        "flex h-dvh shrink-0 flex-col overflow-hidden px-3 py-4",
        SIDEBAR_MOTION,
        "transition-[width]",
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
            <span className="truncate font-medium text-ink-2">{p?.name ?? "—"}</span>
            <span className="truncate font-mono">{p?.branch ?? ""}</span>
          </div>
        </div>
      </div>
      <RailTip rail={rail} label={t.shell.addPartKey}>
        <button
          type="button"
          onClick={addPart}
          aria-label={rail ? t.shell.addPartKey : undefined}
          className="mt-1 mb-3 flex h-9 w-full items-center gap-2 overflow-hidden rounded-control bg-ink px-[11px] font-medium text-[13px] text-inverse shadow-panel transition-opacity hover:opacity-90"
        >
          <Plus size={17} strokeWidth={2} className="shrink-0" />
          <span className={railFade(rail)}>{t.shell.addPart}</span>
          <Kbd className={cn("ml-auto bg-inverse/15 text-2xs text-inverse", railFade(rail))}>N</Kbd>
        </button>
      </RailTip>
      <CommandSearch rail={rail} />
      <nav
        aria-label={t.nav.main}
        className="no-scrollbar min-h-0 flex-1 space-y-2 overflow-y-auto"
      >
        {nav.map((sec) => (
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
            <div className="space-y-2 px-1">
              <CommitChip />
              <LanguageSwitch />
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
