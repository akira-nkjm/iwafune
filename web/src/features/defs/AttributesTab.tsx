import { Plus } from "lucide-react";
import { useState } from "react";
import { useCommandOps } from "@/api/ops";
import type { DesignView } from "@/api/types";
import {
  type NewAttribute,
  NewAttributeFields,
} from "@/features/components/parameters/NewAttributeFields";
import { attrAddCommand } from "@/features/components/parameters/param-ops";
import { useT } from "@/i18n";
import { Pill } from "@/kit/status/Pill";
import { DataTable } from "@/kit/table/DataTable";
import { TablePanel } from "@/kit/table/TablePanel";
import { Td } from "@/kit/table/Td";
import { Th } from "@/kit/table/Th";
import { defName } from "@/lib/design";

import { OpDialog } from "@/ops/OpDialog";
import { useOpCheck } from "@/ops/use-op-check";
import { Button } from "@/ui/button";

const EMPTY: NewAttribute = { id: "", label: "", dimension: "", units: [], byState: false };

/**
 * 属性定義（組み込み＋プロジェクト独自）。新しい属性は `fune attr add` と同じ引数を service で CreateAttribute にする
 * （`attrAddCommand`。パラメーター追加と共通。次元と許可単位は適用のときに検査）
 */
export function AttributesTab({ design }: { design: DesignView }) {
  const t = useT();
  const ta = t.defs.attributes;
  const [adding, setAdding] = useState(false);
  const [fresh, setFresh] = useState<NewAttribute>(EMPTY);
  const attrs = design.defs.attributes;
  const made = useCommandOps(adding ? attrAddCommand(fresh) : null);
  const ops = made.ops;
  const idError = useOpCheck(adding ? ops : null).error ?? made.error;
  return (
    <TablePanel
      title={ta.title}
      caption={ta.caption(attrs.length, attrs.filter((a) => a.builtin).length)}
      right={
        <Button size="sm" onClick={() => setAdding(true)}>
          <Plus size={13} />
          {ta.add}
        </Button>
      }
    >
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
    </TablePanel>
  );
}
