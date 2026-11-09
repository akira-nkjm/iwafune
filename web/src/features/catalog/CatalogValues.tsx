import { useAttrEdit } from "@/api/ops";
import type { CatalogItemView, DesignView } from "@/api/types";
import { useT } from "@/i18n";
import { PropertyList } from "@/kit/PropertyList";
import { ResetButton } from "@/kit/ResetButton";
import { attrDef, attrLabel } from "@/lib/design";
import { AttrValueEditor } from "@/quantity/AttrValueEditor";
import { CatalogAddValue } from "./CatalogAddValue";

/**
 * カタログ品の値（部品の値と同じ API の表示と、同じ `AttrValueEditor` で直す）。打った文字列はそのまま
 * `POST /ops/from_text`（消すときは `/ops/unset_from_text`）に渡し、値の型と単位の読み方は `fune catalog set` と同じ
 * service が決める（`useAttrEdit`）
 */
export function CatalogValues({ design, item }: { design: DesignView; item: CatalogItemView }) {
  const t = useT();
  const edit = useAttrEdit({ kind: "catalog", owner: item });
  const readOnly = !item.editable;
  const entries = Object.entries(item.values);
  return (
    <div className="space-y-2">
      <PropertyList>
        {entries.length === 0 && (
          <p className="px-3 py-2 text-ink-3 text-xs">{t.catalog.noValues}</p>
        )}
        {entries.map(([a, v]) => (
          <AttrValueEditor
            key={a}
            layout="row"
            rowLayout="action"
            owner={item}
            attr={a}
            def={attrDef(design.defs, a)}
            value={v}
            label={attrLabel(design.defs, a)}
            disabled={readOnly}
            action={
              readOnly ? null : (
                <ResetButton title={t.catalog.reset} onClick={() => edit.unset(a)} />
              )
            }
            onSet={(state, text) => edit.set(a, state, text)}
          />
        ))}
      </PropertyList>
      {!readOnly && <CatalogAddValue design={design} item={item} onAdd={edit.set} />}
    </div>
  );
}
