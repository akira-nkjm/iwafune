import { Plus } from "lucide-react";
import type { DesignView } from "@/api/types";
import { useT } from "@/i18n";
import { Reveal } from "@/kit/Reveal";
import { Button } from "@/ui/button";
import { draftCount, type ModeDraft, withCell } from "./draft";
import { ModesSaveBar } from "./ModesSaveBar";
import { ModesTable } from "./ModesTable";
import { matrixGroups } from "./matrix-rows";
import { useModeDraftLoad } from "./use-mode-draft-load";

/**
 * モード表のカード: 見出しの帯（凡例・モード追加）、行列、下書きの保存の帯。下書きを当てた計算は `useModeDraftLoad`、
 * 行への変換は `matrixGroups`、表示は `ModesTable`。カードは画面の高さまで伸び、表はカードの中で縦横にスクロールする。
 */
export function ModesMatrix({
  design,
  draft,
  onDraft,
  onAdd,
}: {
  design: DesignView;
  draft: ModeDraft;
  onDraft: (draft: ModeDraft) => void;
  onAdd: () => void;
}) {
  const tm = useT().modes;
  const table = design.modes;
  const computed = useModeDraftLoad(table, draft);
  const count = draftCount(draft);
  return (
    <div className="flex min-h-0 flex-1 flex-col p-6">
      <section
        aria-label={tm.matrixTitle}
        className="flex min-h-0 flex-col overflow-hidden rounded-card border border-line bg-panel"
      >
        <div className="flex shrink-0 flex-wrap items-center gap-x-4 gap-y-2 border-line border-b px-4 py-3">
          <h2 className="font-semibold text-sm">{tm.matrixTitle}</h2>
          <p className="text-xs text-ink-3">{tm.hint}</p>
          <div className="ml-auto flex gap-3 whitespace-nowrap text-xs text-ink-3">
            <span>{tm.legendOn}</span>
            <span>{tm.legendDuty}</span>
            <span>{tm.legendOff}</span>
          </div>
          <Button variant="outline" onClick={onAdd}>
            <Plus size={14} />
            {tm.add}
          </Button>
        </div>
        <div className="min-h-0 flex-1 scroll-pt-16 scroll-pb-36 scroll-pl-52 overflow-auto">
          <ModesTable
            modes={table.modes}
            groups={matrixGroups(design, draft, computed.load.data, tm)}
            budget={computed.budget}
            updating={computed.load.updating}
            onCell={({ mode, part }, cell) => onDraft(withCell(table, draft, mode, part, cell))}
          />
        </div>
        {/* 保存の帯はカードのいちばん下。最初の下書きで滑らかに開き、保存・破棄で滑らかに閉じる */}
        <Reveal open={count > 0}>
          <ModesSaveBar
            ops={computed.save.ops}
            error={computed.save.error}
            count={count}
            onDone={() => onDraft(new Map())}
          />
        </Reveal>
      </section>
    </div>
  );
}
