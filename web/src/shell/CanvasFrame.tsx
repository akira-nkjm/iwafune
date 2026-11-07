import { PanelRightClose, PanelRightOpen } from "lucide-react";
import type { ReactNode } from "react";
import { cn } from "@/lib/cn";
import { PANEL_MOTION } from "@/lib/motion";

/** 右の詳細欄の開閉（開閉の状態は使う側が持つ） */
export interface InspectorPanel {
  open: boolean;
  onToggle: () => void;
  /** 欄の見出し（例: 「詳細」） */
  title: string;
  /** 開く・閉じるボタンの説明 */
  showLabel: string;
  hideLabel: string;
  content: ReactNode;
}

/**
 * 系統図と 3D のページ: メインパネル全面をキャンバスにし、上にツールバー、右に 480px の詳細欄。
 * 詳細欄の開閉は欄そのものに付ける: 開いているときは見出しの右に閉じるボタン、畳んだときは右端に縦のつまみ
 * （アイコンと縦書きの見出し）を出し、押すと開く。幅はサイドバーと同じ動きで滑らかに変える。
 */
export function CanvasFrame({
  toolbar,
  inspector,
  children,
}: {
  toolbar?: ReactNode;
  inspector?: InspectorPanel;
  children: ReactNode;
}) {
  return (
    <div className="flex min-h-0 flex-1 flex-col border-line border-t">
      {toolbar && (
        <div className="flex shrink-0 flex-wrap items-center gap-2 border-line border-b bg-panel-2 px-4 py-2">
          {toolbar}
        </div>
      )}
      <div className="flex min-h-0 flex-1">
        <div className="relative min-h-0 min-w-0 flex-1">{children}</div>
        {inspector && (
          <aside
            id="canvas-inspector"
            className={cn(
              "shrink-0 overflow-hidden border-line border-l bg-panel transition-[width]",
              PANEL_MOTION,
              inspector.open ? "w-[480px]" : "w-9",
            )}
          >
            {inspector.open ? (
              <div className="flex h-full w-[480px] flex-col">
                <div className="flex shrink-0 items-center justify-between border-line-soft border-b py-1.5 pr-1.5 pl-4">
                  <span className="font-medium text-ink-3 text-xs">{inspector.title}</span>
                  <button
                    type="button"
                    aria-expanded
                    aria-controls="canvas-inspector"
                    aria-label={inspector.hideLabel}
                    title={inspector.hideLabel}
                    onClick={inspector.onToggle}
                    className="flex size-7 items-center justify-center rounded-control text-ink-3 hover:bg-panel-2 hover:text-ink"
                  >
                    <PanelRightClose size={15} strokeWidth={1.7} />
                  </button>
                </div>
                <div className="min-h-0 flex-1 overflow-y-auto">{inspector.content}</div>
              </div>
            ) : (
              <button
                type="button"
                aria-expanded={false}
                aria-controls="canvas-inspector"
                aria-label={inspector.showLabel}
                title={inspector.showLabel}
                onClick={inspector.onToggle}
                className="flex h-full w-full flex-col items-center gap-2 pt-3 text-ink-3 hover:bg-panel-2 hover:text-ink"
              >
                <PanelRightOpen size={15} strokeWidth={1.7} />
                <span className="text-xs [writing-mode:vertical-rl]">{inspector.title}</span>
              </button>
            )}
          </aside>
        )}
      </div>
    </div>
  );
}
