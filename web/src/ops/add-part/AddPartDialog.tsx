import { useState } from "react";
import type { DesignView, Op } from "@/api/types";
import { useT } from "@/i18n";
import { Field } from "@/kit/forms/Field";
import { NativeSelect } from "@/kit/forms/NativeSelect";
import { TextInput } from "@/kit/forms/TextInput";
import { Segmented } from "@/kit/selection/Segmented";
import { defWithId, parentOptions } from "@/lib/design";
import { OpDialog } from "@/ops/OpDialog";
import { useOpCheck } from "@/ops/use-op-check";
import { CatalogPicker } from "./CatalogPicker";
import { groupForSystem, pickOptions } from "./catalog-pick";

type DefBy = "catalog" | "kind";

/** 開いたときの定義の選び方・カタログの品・置き場（カタログの品から開いたときは、その品の系を置き場にする） */
export function initialAddPart(
  design: DesignView,
  groups: readonly { value: string }[],
  initialCatalog: string | null | undefined,
  initialGroup: string | null | undefined,
): { by: DefBy; catalog: string; group: string } {
  const catalog = initialCatalog ?? "";
  const system = design.catalog.find((c) => c.id === catalog)?.system;
  return {
    by: catalog !== "" || pickOptions(design).length > 0 ? "catalog" : "kind",
    catalog,
    group: initialGroup ?? groupForSystem(groups, system) ?? groups[0]?.value ?? "",
  };
}

/**
 * コンポーネントの追加（CreatePart）。定義はカタログか種別のどちらか。
 * `initialCatalog` を渡すと、その品を選んだ状態で開く（ID は人が打つ。欄に合わせる）
 */
export function AddPartDialog({
  design,
  open,
  onOpenChange,
  onDone,
  initialGroup,
  initialCatalog,
  narrow,
}: {
  design: DesignView;
  open: boolean;
  onOpenChange: (open: boolean) => void;
  /** 作ったとき（新しい部品の ID） */
  onDone: (id: string) => void;
  initialGroup?: string | null;
  /** 選んだ状態で開くカタログの品の ID */
  initialCatalog?: string | null;
  /** 選べる種別とカタログの品を絞る（足りない入力の直しの「その役割の部品を足す」。API が候補を返す。両方空なら絞らない） */
  narrow?: { kinds: readonly string[]; catalogItems: readonly string[] };
}) {
  const t = useT();
  const ta = t.components.addPart;
  const [id, setId] = useState("");
  const groups = parentOptions(design);
  const allowCatalog = narrow?.catalogItems ?? [];
  const allowKinds = narrow?.kinds ?? [];
  const options = pickOptions(design).filter(
    (o) => allowCatalog.length === 0 || allowCatalog.includes(o.id),
  );
  const kindOptions = design.defs.kinds.filter(
    (k) => allowKinds.length === 0 || allowKinds.includes(k.id),
  );
  const init = initialAddPart(design, groups, initialCatalog, initialGroup);
  const [group, setGroup] = useState(init.group);
  // 置き場を人が選んだら、品を選び直しても上書きしない
  const [groupTouched, setGroupTouched] = useState(false);
  const [by, setBy] = useState<DefBy>(options.length === 0 ? "kind" : init.by);
  const [catalog, setCatalog] = useState(init.catalog);
  const [kind, setKind] = useState("");
  const pickCatalog = (next: string) => {
    setCatalog(next);
    const system = design.catalog.find((c) => c.id === next)?.system;
    const target = groupForSystem(groups, system);
    if (!groupTouched && target) setGroup(target);
  };
  // ID の欄の例（カタログの品の `name`。API が分けて返す）
  const namePlaceholder = design.catalog.find((c) => c.id === catalog)?.name || "RW_Z";

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

  const failed = useOpCheck(ops).failed;
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
      <Field label="ID" htmlFor="add-part-id">
        <TextInput
          id="add-part-id"
          mono
          value={id}
          invalid={failed}
          placeholder={namePlaceholder}
          autoFocus={!!initialCatalog}
          onChange={(e) => setId(e.target.value)}
        />
      </Field>
      <Field label={ta.group} htmlFor="add-part-group">
        <NativeSelect
          id="add-part-group"
          value={group}
          onChange={(e) => {
            setGroup(e.target.value);
            setGroupTouched(true);
          }}
          placeholder={ta.noGroup}
          options={groups}
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
          <CatalogPicker options={options} value={catalog} onChange={pickCatalog} />
        ) : (
          <NativeSelect
            aria-label={ta.kind}
            value={kind}
            onChange={(e) => setKind(e.target.value)}
            placeholder={ta.pickKind}
            options={kindOptions.map((k) => ({ value: k.id, label: defWithId(k) }))}
          />
        )}
      </div>
    </OpDialog>
  );
}
