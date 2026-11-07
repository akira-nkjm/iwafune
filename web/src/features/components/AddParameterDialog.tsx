import { useCallback, useState } from "react";
import type { DesignView, PartRow, QuantityView } from "@/api/types";
import { useT } from "@/i18n";
import { Field } from "@/kit/Field";
import { NativeSelect } from "@/kit/NativeSelect";
import { Segmented } from "@/kit/Segmented";
import { cn } from "@/lib/cn";
import { OpDialog } from "@/ops/OpDialog";
import { useOpCheck } from "@/ops/use-op-check";
import { QuantityInput } from "@/quantity/QuantityInput";
import { AttributePicker } from "./AttributePicker";
import { type NewAttribute, NewAttributeFields } from "./NewAttributeFields";
import { ProvenanceFields, provenanceOf } from "./ProvenanceFields";
import { paramOps } from "./param-ops";

type Mode = "standard" | "new";
const EMPTY: NewAttribute = { id: "", label: "", dimension: "", units: [], byState: false };

/**
 * 「パラメーター追加」: 標準（組み込み＋プロジェクト独自）の属性から選ぶか、新しく定義する（次元 → 許可単位）。
 * 値は /units/parse で 1 打鍵ごとに検査し、ID などの規則は dry_run で検査する。
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
  const [value, setValue] = useState<QuantityView | null>(null);
  const [prov, setProv] = useState({ source: "", note: "" });
  const onValid = useCallback(
    (_t: string, ok: boolean, q: QuantityView | null) => setValue(ok ? q : null),
    [],
  );
  const def = design.defs.attributes.find((a) => a.id === picked);
  const attrId = mode === "standard" ? picked : fresh.id;
  const dimension = mode === "standard" ? (def?.dimension ?? null) : fresh.dimension || null;
  const byState = mode === "standard" ? (def?.by_state ?? false) : fresh.byState;
  const scalarLike = mode === "new" || !def || def.value_type === "scalar" || def.by_state;
  const ops = paramOps({
    part: part.id,
    mode,
    attrId,
    hasDef: !!def,
    fresh,
    state: byState ? state || null : null,
    value,
    from: provenanceOf(prov),
  });
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
        <NewAttributeFields value={fresh} onChange={setFresh} idError={check.error} />
      )}
      {scalarLike ? (
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
          <Field label={tp.value} hint={dimension ? tp.dimension(dimension) : undefined}>
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
          {tp.notScalar(def?.value_type ?? "", `${part.id}.${attrId}`)}
        </p>
      )}
      <ProvenanceFields value={prov} onChange={setProv} />
    </OpDialog>
  );
}
