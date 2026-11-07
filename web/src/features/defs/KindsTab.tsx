import { Plus } from "lucide-react";
import { useState } from "react";
import type { DesignView, Op } from "@/api/types";
import { localized, useT } from "@/i18n";
import { DataTable } from "@/kit/DataTable";
import { Field } from "@/kit/Field";
import { Panel } from "@/kit/Panel";
import { PanelHeader } from "@/kit/PanelHeader";
import { Pill } from "@/kit/Pill";
import { Td } from "@/kit/Td";
import { TextInput } from "@/kit/TextInput";
import { Th } from "@/kit/Th";

import { OpDialog } from "@/ops/OpDialog";
import { useOpCheck } from "@/ops/use-op-check";
import { Button } from "@/ui/button";

// edit::list_from_text と同じ: カンマのみで分割。内部の空白は保存する。
const list = (s: string) =>
  s
    .split(",")
    .map((x) => x.trim())
    .filter(Boolean);

/** 種別定義（機器の状態の語彙・必須の属性・ポートの雛形）。新しい種別は CreateKind */
export function KindsTab({ design }: { design: DesignView }) {
  const t = useT();
  const tk = t.defs.kinds;
  const [adding, setAdding] = useState(false);
  const [id, setId] = useState("");
  const [role, setRole] = useState("");
  const [states, setStates] = useState("off, on");
  const [required, setRequired] = useState("mass");
  const kinds = design.defs.kinds;
  const ops: Op[] | null = id
    ? [
        {
          op: "create_kind",
          def: {
            id,
            label: id,
            role: role || null,
            states: list(states),
            required: list(required),
          },
        },
      ]
    : null;
  const error = useOpCheck(adding ? ops : null).error;
  return (
    <Panel pad={false}>
      <div className="p-4 pb-2">
        <PanelHeader
          className="mb-0"
          title={tk.title}
          caption={tk.caption(kinds.length)}
          right={
            <Button size="sm" onClick={() => setAdding(true)}>
              <Plus size={13} />
              {tk.add}
            </Button>
          }
        />
      </div>
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
            <Td>{localized(k.label, k.label_en)}</Td>
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
    </Panel>
  );
}
