import { Plus } from "lucide-react";
import { useState } from "react";
import { useCommandOps } from "@/api/ops";
import { useDesign } from "@/api/queries";
import type { Op } from "@/api/types";
import { useT } from "@/i18n";
import { Empty } from "@/kit/Empty";
import { Loading, SkeletonTable } from "@/kit/Loading";
import { Reveal } from "@/kit/Reveal";
import { PageHeader } from "@/shell/PageHeader";
import { Button } from "@/ui/button";
import { AddModeDialog } from "./AddModeDialog";
import { draftCommand, draftCount, type ModeDraft } from "./draft";
import { ModesSaveBar } from "./ModesSaveBar";
import { ModesTable } from "./ModesTable";

const NO_OPS: readonly Op[] = [];

/**
 * 設計 / モード: 「部品 × モード」の表。ファイルは 1 モード 1 ファイルだが、ここでは表で見て一括で直す。
 * セルの変更は下書きに貯め、変えたセルを `fune mode set|unset` と同じ引数にして service で 1 つの SetModeStates にする
 * （打った時間割合を読むのも service）。表の電力と合計はそれを書き込まずに当てた値で、「保存」で POST /ops で送る。
 */
export function ModesPage() {
  const design = useDesign();
  const t = useT();
  const tm = t.modes;
  const [draft, setDraft] = useState<ModeDraft>(new Map());
  const [adding, setAdding] = useState(false);
  const d = design.data;
  const table = d?.modes;
  const count = draftCount(draft);
  const made = useCommandOps(table ? draftCommand(table, draft) : null);
  const unassigned = table
    ? table.rows.reduce((n, r) => n + table.modes.filter((m) => !r.cells[m.id]).length, 0)
    : 0;
  return (
    <>
      <PageHeader
        eyebrow={tm.eyebrow}
        title={tm.title}
        subtitle={
          table ? tm.subtitle(table.rows.length, table.modes.length, unassigned) : undefined
        }
      />
      {design.isError ? (
        <Empty>{t.common.cannotLoadDesign(design.error.message)}</Empty>
      ) : !d || !table ? (
        <Loading className="border-line border-t">
          <SkeletonTable rows={10} cols={6} />
        </Loading>
      ) : table.modes.length === 0 ? (
        <Empty>
          {tm.empty}
          <Button variant="outline" onClick={() => setAdding(true)}>
            <Plus size={14} />
            {tm.add}
          </Button>
        </Empty>
      ) : (
        // カードは画面の高さまで伸び、表はカードの中で縦横にスクロールする（見出し・部品の列・合計は固定）
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
              <Button variant="outline" onClick={() => setAdding(true)}>
                <Plus size={14} />
                {tm.add}
              </Button>
            </div>
            <div className="min-h-0 flex-1 scroll-pt-16 scroll-pb-36 scroll-pl-52 overflow-auto">
              <ModesTable
                design={d}
                table={table}
                draft={draft}
                ops={made.latest ?? NO_OPS}
                onDraft={setDraft}
              />
            </div>
            {/* 保存の帯はカードのいちばん下。最初の下書きで滑らかに開き、保存・破棄で滑らかに閉じる */}
            <Reveal open={count > 0}>
              <ModesSaveBar
                ops={made.ops}
                error={made.error}
                count={count}
                onDone={() => setDraft(new Map())}
              />
            </Reveal>
          </section>
        </div>
      )}
      {table && adding && <AddModeDialog table={table} open onOpenChange={setAdding} />}
    </>
  );
}
