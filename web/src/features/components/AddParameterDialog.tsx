import { useCallback, useState } from "react";
import { useTextOp } from "@/api/ops";
import type { DesignView, Op, PartRow, SetText } from "@/api/types";
import { useT } from "@/i18n";
import { Field } from "@/kit/Field";
import { NativeSelect } from "@/kit/NativeSelect";
import { Segmented } from "@/kit/Segmented";
import { cn } from "@/lib/cn";
import { attrDef, newAttrPath, paramPath } from "@/lib/design";
import { OpDialog } from "@/ops/OpDialog";
import { useOpCheck } from "@/ops/use-op-check";
import { attrEditMode } from "@/quantity/attr-mode";
import { QuantityInput } from "@/quantity/QuantityInput";
import { AttributePicker } from "./AttributePicker";
import { type NewAttribute, NewAttributeFields } from "./NewAttributeFields";
import { ProvenanceFields } from "./ProvenanceFields";
import { createAttributeOp } from "./param-ops";

type Mode = "standard" | "new";
const EMPTY: NewAttribute = { id: "", label: "", dimension: "", units: [], byState: false };

/**
 * 「パラメーター追加」: 標準（組み込み＋プロジェクト独自）の属性から選ぶか、新しく定義する（次元 → 許可単位）。
 * 値は /units/parse で 1 打鍵ごとに検査し、打った文字列をパラメーターパスと一緒に `POST /ops/from_text` で Op にする
 * （`fune set` と同じ service。新しく定義する属性は、その定義を当てた設計で読む）。ID などの規則は dry_run で検査する。
 */
export function AddParameterDialog({
  design,
  part,
  open,
  onOpenChange,
}: {
  design: DesignView;
  part: PartRow;
  open: boolean;
  onOpenChange: (open: boolean) => void;
}) {
  const tp = useT().components.param;
  const [mode, setMode] = useState<Mode>("standard");
  const [picked, setPicked] = useState("");
  const [fresh, setFresh] = useState<NewAttribute>(EMPTY);
  const [state, setState] = useState(part.states[0] ?? "");
  const [value, setValue] = useState<string | null>(null);
  const [prov, setProv] = useState({ source: "", note: "" });
  const onValid = useCallback(
    (typed: string, ok: boolean) => setValue(ok ? typed.trim() : null),
    [],
  );
  const def = attrDef(design.defs, picked);
  const create = mode === "new" ? createAttributeOp(fresh) : null;
  const attrId = mode === "standard" ? picked : fresh.id;
  const dimension = mode === "standard" ? (def?.dimension ?? null) : fresh.dimension || null;
  const byState = mode === "standard" ? (def?.by_state ?? false) : fresh.byState;
  const editable = mode === "new" || !def || attrEditMode(def) !== "display";
  const cell = byState ? state || null : null;
  // 定義のある属性は API の paths、同じ編集で定義を作る属性（create は新しく定義するときだけ）はつないだパス
  const path = create
    ? newAttrPath(part, fresh.id, cell)
    : mode === "standard" && def
      ? paramPath(part, picked, cell)
      : null;
  const text: SetText | null =
    path && value && editable
      ? {
          item: null,
          path,
          value,
          source: prov.source || null,
          note: prov.note || null,
          min: null,
          max: null,
          ops: create ? [create] : [],
        }
      : null;
  const textOp = useTextOp(open ? text : null);
  const ops: Op[] | null = textOp.op ? [...(create ? [create] : []), textOp.op] : null;
  const check = useOpCheck(ops);
  return (
    <OpDialog
      open={open}
      onOpenChange={onOpenChange}
      wide
      title={tp.title(part.id)}
      description={tp.description}
      ops={ops}
      summary={tp.summary(part.id, attrId)}
      confirmLabel={tp.confirm}
    >
      <Segmented
        label={tp.how}
        value={mode}
        onChange={setMode}
        options={[
          { value: "standard", label: tp.standard },
          { value: "new", label: tp.define },
        ]}
      />
      {mode === "standard" ? (
        <AttributePicker
          attributes={design.defs.attributes}
          part={part}
          picked={picked}
          onPick={setPicked}
        />
      ) : (
        <NewAttributeFields
          value={fresh}
          onChange={setFresh}
          idError={check.error ?? textOp.error}
        />
      )}
      {editable ? (
        <div className={cn("grid gap-3", byState ? "grid-cols-[140px_1fr]" : "grid-cols-1")}>
          {byState && (
            <Field label={tp.state} htmlFor="param-state">
              <NativeSelect
                id="param-state"
                value={state}
                onChange={(e) => setState(e.target.value)}
                options={part.states.map((s) => ({ value: s, label: s }))}
              />
            </Field>
          )}
          <Field
            label={tp.value}
            error={mode === "standard" ? textOp.error : null}
            hint={dimension ? tp.dimension(dimension) : undefined}
          >
            <QuantityInput
              key={`${attrId}-${dimension}`}
              initial=""
              dimension={dimension}
              label={tp.value}
              onChangeValid={onValid}
            />
          </Field>
        </div>
      ) : (
        <p className="text-ink-3 text-xs">
          {tp.notScalar(def?.value_type ?? "", paramPath(part, picked))}
        </p>
      )}
      <ProvenanceFields value={prov} onChange={setProv} />
    </OpDialog>
  );
}
