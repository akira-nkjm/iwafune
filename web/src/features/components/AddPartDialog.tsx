import { useState } from "react";
import type { DesignView, Op } from "@/api/types";
import { localized, useT } from "@/i18n";
import { Field } from "@/kit/forms/Field";
import { NativeSelect } from "@/kit/forms/NativeSelect";
import { TextInput } from "@/kit/forms/TextInput";
import { Segmented } from "@/kit/selection/Segmented";
import { defWithId, groupsOf, nodeName, withNote } from "@/lib/design";
import { OpDialog } from "@/ops/OpDialog";
import { useOpCheck } from "@/ops/use-op-check";

type DefBy = "catalog" | "kind";

/** コンポーネントの追加（CreatePart）。定義はカタログか種別のどちらか */
export function AddPartDialog({
  design,
  open,
  onOpenChange,
  onDone,
  initialGroup,
}: {
  design: DesignView;
  open: boolean;
  onOpenChange: (open: boolean) => void;
  /** 作ったとき（新しい部品の ID） */
  onDone: (id: string) => void;
  initialGroup?: string | null;
}) {
  const t = useT();
  const ta = t.components.addPart;
  const [id, setId] = useState("");
  const groups = groupsOf(design);
  // カタログの品（GET /design の catalog）と、部品が参照している品（共有カタログを含む）
  const known = [
    ...new Set([
      ...design.catalog.map((c) => c.id),
      ...design.parts.flatMap((p) => [...p.candidates, ...(p.catalog ? [p.catalog] : [])]),
    ]),
  ].sort();
  const catalogLabel = new Map(
    design.catalog.map((c) => [c.id, withNote(localized(c.label, c.label_en), c.kind)]),
  );
  const [group, setGroup] = useState(initialGroup ?? groups[0]?.id ?? "");
  const [by, setBy] = useState<DefBy>(known.length > 0 ? "catalog" : "kind");
  const [catalog, setCatalog] = useState("");
  const [kind, setKind] = useState("");

  const defOk = by === "catalog" ? catalog !== "" : kind !== "";
  // ID の規則と重複はサーバー（dry_run）が検査する
  const ops: Op[] | null =
    id && defOk
      ? [
          {
            op: "create_part",
            id,
            group: group || null,
            catalog: by === "catalog" ? catalog : null,
            kind: by === "kind" ? kind : null,
            count: null,
            label: null,
            mode_state: null,
          },
        ]
      : null;

  const idError = useOpCheck(ops).error;
  return (
    <OpDialog
      open={open}
      onOpenChange={onOpenChange}
      title={ta.title}
      description={ta.description}
      ops={ops}
      summary={ta.summary(id)}
      confirmLabel={ta.confirm}
      onDone={() => {
        onDone(id);
        setId("");
      }}
    >
      <Field label="ID" htmlFor="add-part-id" error={idError}>
        <TextInput
          id="add-part-id"
          mono
          value={id}
          invalid={!!idError}
          placeholder="RW_Z"
          onChange={(e) => setId(e.target.value)}
        />
      </Field>
      <Field label={ta.group} htmlFor="add-part-group">
        <NativeSelect
          id="add-part-group"
          value={group}
          onChange={(e) => setGroup(e.target.value)}
          placeholder={ta.noGroup}
          options={groups.map((g) => ({ value: g.id, label: nodeName(g) }))}
        />
      </Field>
      <div className="grid gap-1.5">
        <Segmented
          label={ta.definition}
          value={by}
          onChange={setBy}
          options={[
            { value: "catalog", label: ta.fromCatalog },
            { value: "kind", label: ta.kindOnly },
          ]}
        />
        {by === "catalog" ? (
          <>
            <TextInput
              aria-label={ta.catalog}
              mono
              list="add-part-catalogs"
              value={catalog}
              placeholder={known[0] ?? "local/rw_10mNms"}
              onChange={(e) => setCatalog(e.target.value)}
            />
            <datalist id="add-part-catalogs">
              {known.map((c) => (
                <option key={c} value={c} label={catalogLabel.get(c)} />
              ))}
            </datalist>
          </>
        ) : (
          <NativeSelect
            aria-label={ta.kind}
            value={kind}
            onChange={(e) => setKind(e.target.value)}
            placeholder={ta.pickKind}
            options={design.defs.kinds.map((k) => ({ value: k.id, label: defWithId(k) }))}
          />
        )}
      </div>
    </OpDialog>
  );
}
