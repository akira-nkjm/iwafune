import { PanelLeftClose, PanelLeftOpen } from "lucide-react";
import { type MouseEvent, type ReactNode, type RefObject, useRef } from "react";
import { useT } from "@/i18n";
import { cn } from "@/lib/cn";
import { Tooltip, TooltipContent, TooltipTrigger } from "@/ui/tooltip";
import { BrandMark } from "../brand/BrandMark";
import { BrandWordmark } from "../brand/BrandWordmark";
import { LOCKUP } from "../brand/brand-art";
import { railFade } from "./sidebar-motion";
import { SIDEBAR_ID } from "./sidebar-state";

/** マークの幅 [px]。名前の大きさはここから LOCKUP の比で決まる（32 px → 16 px） */
const MARK_WIDTH = 32;

/**
 * ロゴと名前、サイドバーの開閉ボタン。開いているときは名前の右に閉じるボタンを置く。畳んだレールでは
 * ロゴそのものが開くボタンを兼ね、カーソルを載せる（キーボードで選ぶ）とロゴが開くアイコンに変わる。
 * ロゴの位置は開いていても畳んでいても同じ（レールの中央）。キーボードで切り替えたら、反対側のボタンへフォーカスを
 * 移す（スクロールはさせない）。マウスで押したときは移さない: 幅が広がる途中でまだ見えていないボタンへフォーカスを
 * 移すと、ブラウザがそれを見せようとしてサイドバーをスクロールし、開く動きがカクッとする（⌘B では起きない）。
 * マークと名前の並べ方（大きさの比・縦の中心）は brand-art.ts の LOCKUP で、README の SVG と同じ。
 */
export function BrandToggle({ rail, onToggle }: { rail: boolean; onToggle: () => void }) {
  const t = useT();
  const openRef = useRef<HTMLButtonElement>(null);
  const closeRef = useRef<HTMLButtonElement>(null);
  const toggleTo = (
    next: RefObject<HTMLButtonElement | null>,
    e: MouseEvent<HTMLButtonElement>,
  ) => {
    onToggle();
    // detail が 0 のクリックはキーボード（Enter・Space）で押したもの
    if (e.detail === 0) requestAnimationFrame(() => next.current?.focus({ preventScroll: true }));
  };
  return (
    // マーク（幅 32 px）は 32 px の枠いっぱいに置く。中心がナビのアイコンの列の中心と合う
    <div className="flex h-10 shrink-0 items-center gap-2">
      {rail ? (
        <Tip label={t.shell.openSidebar}>
          <button
            ref={openRef}
            type="button"
            onClick={(e) => toggleTo(closeRef, e)}
            aria-label={t.shell.openSidebar}
            aria-expanded={false}
            aria-controls={SIDEBAR_ID}
            className="group/brand relative flex size-8 shrink-0 items-center justify-center rounded-control transition-colors hover:bg-panel focus-visible:bg-panel"
          >
            <span className="flex transition-[opacity,scale] duration-150 ease-out group-hover/brand:scale-90 group-hover/brand:opacity-0 group-focus-visible/brand:opacity-0 motion-reduce:transition-none">
              <BrandMark size={MARK_WIDTH} tight />
            </span>
            <PanelLeftOpen
              size={18}
              strokeWidth={1.7}
              aria-hidden
              className="absolute scale-90 text-ink-2 opacity-0 transition-[opacity,scale] duration-150 ease-out group-hover/brand:scale-100 group-hover/brand:opacity-100 group-focus-visible/brand:scale-100 group-focus-visible/brand:opacity-100 motion-reduce:transition-none"
            />
          </button>
        </Tip>
      ) : (
        <span className="flex size-8 shrink-0 items-center justify-center">
          <BrandMark size={MARK_WIDTH} tight />
        </span>
      )}
      {/* 名前の SVG は縦の中心が x ハイトの帯の中心。items-center だけでマークの外枠の中心とそろう */}
      <span aria-hidden={rail} className={cn("flex min-w-0", railFade(rail))}>
        <BrandWordmark size={LOCKUP.emPerMarkWidth * MARK_WIDTH} />
      </span>
      {/* 閉じるボタンはレールでは名前と一緒に薄れて消える（幅の外に出るので押せない） */}
      <Tip label={t.shell.closeSidebar}>
        <button
          ref={closeRef}
          type="button"
          onClick={(e) => toggleTo(openRef, e)}
          aria-label={t.shell.closeSidebar}
          aria-expanded={true}
          aria-controls={SIDEBAR_ID}
          aria-hidden={rail}
          tabIndex={rail ? -1 : undefined}
          className={cn(
            "ml-auto flex size-8 shrink-0 items-center justify-center rounded-control text-ink-3 transition-colors hover:bg-panel hover:text-ink focus-visible:bg-panel",
            railFade(rail),
          )}
        >
          <PanelLeftClose size={18} strokeWidth={1.7} aria-hidden />
        </button>
      </Tip>
    </div>
  );
}

function Tip({ label, children }: { label: string; children: ReactNode }) {
  return (
    <Tooltip>
      <TooltipTrigger asChild>{children}</TooltipTrigger>
      <TooltipContent side="right">{label}</TooltipContent>
    </Tooltip>
  );
}
