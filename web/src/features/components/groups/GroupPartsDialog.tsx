import { useId, useState } from "react";
import type { DesignView, PartRow } from "@/api/types";
import { useT } from "@/i18n";
import { Field } from "@/kit/forms/Field";
import { TextInput } from "@/kit/forms/TextInput";
import { nodeLabel } from "@/lib/design";
import { OpDialog } from "@/ops/OpDialog";
import { useOpCheck } from "@/ops/use-op-check";
import { groupOps, siblingParts } from "./group-ops";

/**
 * 同じ親の部品をグループにまとめる（部品ごとの MovePart）。開いた部品に印を付けた状態で、同じ親の部品から足し引きする。
 * グループの ID が無ければ親の下に作り、あれば（同じ親の下の）そのグループへ入れる。ID の規則と重複はサーバー（dry_run）が検査する。
 */
export function GroupPartsDialog({
  design,
  part,
  open,
  onOpenChange,
  onDone,
}: {
  design: DesignView;
  /** 開いた部品（最初から印が付く） */
  part: PartRow;
  open: boolean;
  onOpenChange: (open: boolean) => void;
  /** まとめたとき（グループの ID） */
  onDone: (group: string) => void;
}) {
  const tg = useT().components.group;
  const idField = useId();
  const [group, setGroup] = useState("");
  const [checked, setChecked] = useState<ReadonlySet<string>>(new Set([part.id]));
  const siblings = siblingParts(design, part);
  const parent = part.parent;
  const ids = siblings.map((p) => p.id).filter((id) => checked.has(id));
  const ops = parent && group && ids.length > 0 ? groupOps(parent, group, ids) : null;
  const error = useOpCheck(open ? ops : null).error;
  const toggle = (id: string) =>
    setChecked((prev) => {
      const next = new Set(prev);
      if (!next.delete(id)) next.add(id);
      return next;
    });
  return (
    <OpDialog
      open={open}
      onOpenChange={onOpenChange}
      title={tg.title}
      description={tg.description(parent ? nodeLabel(design, parent) : "")}
      ops={ops}
      summary={tg.summary(group, ids.length)}
      confirmLabel={tg.confirm}
      onDone={() => onDone(group)}
    >
      <Field label={tg.idLabel} htmlFor={idField} error={error} hint={tg.idHint}>
        <TextInput
          id={idField}
          mono
          value={group}
          invalid={!!error}
          placeholder="RW"
          onChange={(e) => setGroup(e.target.value)}
        />
      </Field>
      <fieldset className="grid gap-1">
        <legend className="mb-1 font-medium text-ink-2 text-xs">{tg.partsLabel}</legend>
        <div className="grid max-h-48 gap-1 overflow-auto">
          {siblings.map((p) => (
            <label key={p.id} className="flex items-center gap-2 font-mono text-xs">
              <input
                type="checkbox"
                checked={checked.has(p.id)}
                onChange={() => toggle(p.id)}
                className="accent-accent"
              />
              {p.id}
            </label>
          ))}
        </div>
      </fieldset>
    </OpDialog>
  );
}
