import { Plus } from "lucide-react";
import { useState } from "react";
import { useDesign } from "@/api/queries";
import { useT } from "@/i18n";
import { Empty } from "@/kit/feedback/Empty";
import { SkeletonTable } from "@/kit/feedback/Loading";
import { QueryState } from "@/kit/feedback/QueryState";
import { AddModeDialog } from "@/ops/add-mode/AddModeDialog";
import { PageHeader } from "@/shell/PageHeader";
import { Button } from "@/ui/button";
import type { ModeDraft } from "./draft/draft";
import { ModesMatrix } from "./matrix/ModesMatrix";
import { SequencePanel } from "./SequencePanel";

/**
 * 設計 / モード: 「部品 × モード」の表。ファイルは 1 モード 1 ファイルだが、ここでは表で見て一括で直す。
 * セルの変更は下書きに貯め、変えたセルを `fune mode set|unset` と同じ引数にして service で 1 つの SetModeStates にする
 * （打った時間割合を読むのも service）。「保存」で POST /ops に送る。各系の値は系のモードタブで表示する。
 */
export function ModesPage() {
  const design = useDesign();
  const tm = useT().modes;
  const [draft, setDraft] = useState<ModeDraft>(new Map());
  const [adding, setAdding] = useState(false);
  const table = design.data?.modes;
  return (
    <>
      <PageHeader eyebrow={tm.eyebrow} title={tm.title} subtitle={tm.hint} />
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
            <>
              <SequencePanel table={d.modes} />
              <ModesMatrix
                design={d}
                draft={draft}
                onDraft={setDraft}
                onAdd={() => setAdding(true)}
              />
            </>
          )
        }
      </QueryState>
      {table && adding && <AddModeDialog table={table} open onOpenChange={setAdding} />}
    </>
  );
}
