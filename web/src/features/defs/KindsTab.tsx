import { Plus } from "lucide-react";
import { useState } from "react";
import { useCommandOps } from "@/api/ops";
import type { CommandText, DesignView } from "@/api/types";
import { useT } from "@/i18n";
import { Field } from "@/kit/forms/Field";
import { TextInput } from "@/kit/forms/TextInput";
import { Pill } from "@/kit/status/Pill";
import { DataTable } from "@/kit/table/DataTable";
import { TablePanel } from "@/kit/table/TablePanel";
import { Td } from "@/kit/table/Td";
import { Th } from "@/kit/table/Th";
import { defName } from "@/lib/design";

import { OpDialog } from "@/ops/OpDialog";
import { useOpCheck } from "@/ops/use-op-check";
import { Button } from "@/ui/button";

/**
 * 種別定義（機器の状態の語彙・必須の属性・ポートの雛形）。新しい種別は `fune kind add` と同じ引数を打った文字列のまま
 * service に渡して CreateKind にする（状態・属性のカンマ区切りの一覧を読むのも、表示名の既定も service）
 */
export function KindsTab({ design }: { design: DesignView }) {
  const t = useT();
  const tk = t.defs.kinds;
  const [adding, setAdding] = useState(false);
  const [id, setId] = useState("");
  const [role, setRole] = useState("");
  const [states, setStates] = useState("off, on");
  const [required, setRequired] = useState("mass");
  const kinds = design.defs.kinds;
  const command: CommandText | null = id
    ? { command: "kind_add", id, role, states, required, ports: [] }
    : null;
  const made = useCommandOps(adding ? command : null);
  const ops = made.ops;
  const error = useOpCheck(adding ? ops : null).error ?? made.error;
  return (
    <TablePanel
      title={tk.title}
      caption={tk.caption(kinds.length)}
      right={
        <Button size="sm" onClick={() => setAdding(true)}>
          <Plus size={13} />
          {tk.add}
        </Button>
      }
    >
      <DataTable
        head={
          <>
            <Th>{tk.id}</Th>
            <Th>{tk.name}</Th>
            <Th>{tk.role}</Th>
            <Th>{tk.parent}</Th>
            <Th>{tk.states}</Th>
            <Th>{tk.required}</Th>
            <Th>{tk.optional}</Th>
            <Th>{tk.ports}</Th>
            <Th>{tk.origin}</Th>
          </>
        }
      >
        {kinds.map((k) => (
          <tr key={k.id}>
            <Td mono>{k.id}</Td>
            <Td>{defName(k)}</Td>
            <Td mono className="text-ink-3">
              {k.role ?? "—"}
            </Td>
            <Td mono className="text-ink-3">
              {k.parent ?? "—"}
            </Td>
            <Td mono>{k.states.join(" / ")}</Td>
            <Td mono className="text-ink-2">
              {k.required.join(", ")}
            </Td>
            <Td mono className="text-ink-3">
              {k.optional.join(", ")}
            </Td>
            <Td mono className="text-ink-3">
              {k.ports.join(", ")}
            </Td>
            <Td>
              {k.builtin ? (
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
          onOpenChange={(o) => !o && setAdding(false)}
          title={tk.dialogTitle}
          description={tk.dialogDescription}
          ops={ops}
          summary={tk.summary(id)}
          confirmLabel={tk.confirm}
        >
          <div className="grid grid-cols-2 gap-3">
            <Field label="ID" htmlFor="kind-id" error={error}>
              <TextInput
                id="kind-id"
                mono
                value={id}
                invalid={!!error}
                placeholder="magnetorquer"
                onChange={(e) => setId(e.target.value)}
              />
            </Field>
            <Field label={tk.roleField} htmlFor="kind-role" hint={tk.roleHint}>
              <TextInput
                id="kind-role"
                mono
                value={role}
                placeholder="magnetorquer"
                onChange={(e) => setRole(e.target.value)}
              />
            </Field>
          </div>
          <Field label={tk.statesField} htmlFor="kind-states">
            <TextInput
              id="kind-states"
              mono
              value={states}
              onChange={(e) => setStates(e.target.value)}
            />
          </Field>
          <Field label={tk.requiredField} htmlFor="kind-req">
            <TextInput
              id="kind-req"
              mono
              value={required}
              onChange={(e) => setRequired(e.target.value)}
            />
          </Field>
        </OpDialog>
      )}
    </TablePanel>
  );
}
