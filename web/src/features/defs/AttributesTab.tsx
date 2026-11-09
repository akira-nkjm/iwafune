import { Plus } from "lucide-react";
import { useState } from "react";
import type { DesignView, Op } from "@/api/types";
import { type NewAttribute, NewAttributeFields } from "@/features/components/NewAttributeFields";
import { useT } from "@/i18n";
import { DataTable } from "@/kit/DataTable";
import { Panel } from "@/kit/Panel";
import { PanelHeader } from "@/kit/PanelHeader";
import { Pill } from "@/kit/Pill";
import { Td } from "@/kit/Td";
import { Th } from "@/kit/Th";
import { defName } from "@/lib/design";

import { OpDialog } from "@/ops/OpDialog";
import { useOpCheck } from "@/ops/use-op-check";
import { Button } from "@/ui/button";

const EMPTY: NewAttribute = { id: "", label: "", dimension: "", units: [], byState: false };

/** 属性定義（組み込み＋プロジェクト独自）。新しい属性は CreateAttribute（次元と許可単位を検査） */
export function AttributesTab({ design }: { design: DesignView }) {
  const t = useT();
  const ta = t.defs.attributes;
  const [adding, setAdding] = useState(false);
  const [fresh, setFresh] = useState<NewAttribute>(EMPTY);
  const attrs = design.defs.attributes;
  const ops: Op[] | null =
    fresh.id && fresh.dimension
      ? [
          {
            op: "create_attribute",
            def: {
              id: fresh.id,
              label: fresh.label || fresh.id,
              type: "scalar",
              dimension: fresh.dimension,
              units: fresh.units,
              by_state: fresh.byState,
            },
          },
        ]
      : null;
  const idError = useOpCheck(adding ? ops : null).error;
  return (
    <Panel pad={false}>
      <div className="p-4 pb-2">
        <PanelHeader
          className="mb-0"
          title={ta.title}
          caption={ta.caption(attrs.length, attrs.filter((a) => a.builtin).length)}
          right={
            <Button size="sm" onClick={() => setAdding(true)}>
              <Plus size={13} />
              {ta.add}
            </Button>
          }
        />
      </div>
      <DataTable
        head={
          <>
            <Th>{ta.id}</Th>
            <Th>{ta.name}</Th>
            <Th>{ta.valueType}</Th>
            <Th>{ta.dimension}</Th>
            <Th>{ta.units}</Th>
            <Th>{ta.displayUnit}</Th>
            <Th>{ta.byState}</Th>
            <Th>{ta.origin}</Th>
          </>
        }
      >
        {attrs.map((a) => (
          <tr key={a.id} title={a.description ?? undefined}>
            <Td mono>{a.id}</Td>
            <Td>{defName(a)}</Td>
            <Td mono className="text-ink-3">
              {a.value_type}
            </Td>
            <Td>{a.dimension ?? "—"}</Td>
            <Td mono className="text-ink-3">
              {a.units.join(", ") || ta.allUnits}
            </Td>
            <Td mono>{a.display_unit ?? "—"}</Td>
            <Td>{a.by_state ? "✓" : ""}</Td>
            <Td>
              {a.builtin ? (
                <Pill>{t.defs.builtin}</Pill>
              ) : (
                <Pill tone="accent">{t.defs.project}</Pill>
              )}
            </Td>
          </tr>
        ))}
      </DataTable>
      {adding && (
        <OpDialog
          open
          wide
          onOpenChange={(o) => {
            if (!o) {
              setAdding(false);
              setFresh(EMPTY);
            }
          }}
          title={ta.dialogTitle}
          description={ta.dialogDescription}
          ops={ops}
          summary={ta.summary(fresh.id)}
          confirmLabel={ta.confirm}
        >
          <NewAttributeFields value={fresh} onChange={setFresh} idError={idError} />
        </OpDialog>
      )}
    </Panel>
  );
}
