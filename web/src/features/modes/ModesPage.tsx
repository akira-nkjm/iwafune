import { Plus } from "lucide-react";
import { useState } from "react";
import { useDesign } from "@/api/queries";
import { useT } from "@/i18n";
import { Empty } from "@/kit/feedback/Empty";
import { SkeletonTable } from "@/kit/feedback/Loading";
import { QueryState } from "@/kit/feedback/QueryState";
import { PageHeader } from "@/shell/PageHeader";
import { Button } from "@/ui/button";
import { AddModeDialog } from "./AddModeDialog";
import type { ModeDraft } from "./draft/draft";
import { ModesMatrix } from "./matrix/ModesMatrix";
import { unassignedCells } from "./matrix/matrix";

/**
 * 設計 / モード: 「部品 × モード」の表。ファイルは 1 モード 1 ファイルだが、ここでは表で見て一括で直す。
 * セルの変更は下書きに貯め、変えたセルを `fune mode set|unset` と同じ引数にして service で 1 つの SetModeStates にする
 * （打った時間割合を読むのも service）。表の電力と合計はそれを書き込まずに当てた値で、「保存」で POST /ops で送る。
 */
export function ModesPage() {
  const design = useDesign();
  const tm = useT().modes;
  const [draft, setDraft] = useState<ModeDraft>(new Map());
  const [adding, setAdding] = useState(false);
  const table = design.data?.modes;
  return (
    <>
      <PageHeader
        eyebrow={tm.eyebrow}
        title={tm.title}
        subtitle={
          table
            ? tm.subtitle(table.rows.length, table.modes.length, unassignedCells(table))
            : undefined
        }
      />
      <QueryState
        query={design}
        className="border-line border-t"
        skeleton={<SkeletonTable rows={10} cols={6} />}
      >
        {(d) =>
          d.modes.modes.length === 0 ? (
            <Empty>
              {tm.empty}
              <Button variant="outline" onClick={() => setAdding(true)}>
                <Plus size={14} />
                {tm.add}
              </Button>
            </Empty>
          ) : (
            <ModesMatrix
              design={d}
              draft={draft}
              onDraft={setDraft}
              onAdd={() => setAdding(true)}
            />
          )
        }
      </QueryState>
      {table && adding && <AddModeDialog table={table} open onOpenChange={setAdding} />}
    </>
  );
}
