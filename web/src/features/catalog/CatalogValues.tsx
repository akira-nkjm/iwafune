import type { ReactNode } from "react";
import { useSetText, useUnsetText } from "@/api/ops";
import type { CatalogItemView, DesignView, ValueView } from "@/api/types";
import { localized, useT } from "@/i18n";
import { PropertyList } from "@/kit/PropertyList";
import { PropertyRow } from "@/kit/PropertyRow";
import { ResetButton } from "@/kit/ResetButton";
import { paramPath } from "@/lib/design";
import { EditableQuantity } from "@/quantity/EditableQuantity";
import { quantityInputText, scalarOf, stateValue, valueText } from "@/quantity/format";
import { CatalogAddValue } from "./CatalogAddValue";

/**
 * カタログ品の値（部品の値と同じ API の表示を出し、セルで直接直す）。打った文字列はそのまま `POST /ops/from_text`
 * （消すときは `/ops/unset_from_text`）に渡し、値の型と単位の読み方は `fune catalog set` と同じ service が決める
 */
export function CatalogValues({ design, item }: { design: DesignView; item: CatalogItemView }) {
  const t = useT();
  const setText = useSetText();
  const unsetText = useUnsetText();
  const readOnly = !item.editable;
  const defs = new Map(design.defs.attributes.map((a) => [a.id, a]));
  const set = (attr: string, state: string | null, text: string) => {
    const path = paramPath(item, attr, state);
    setText.mutate({ item: item.id, path, value: text, summary: t.catalog.summarySet(path, text) });
  };
  const unset = (attr: string) => {
    const path = paramPath(item, attr);
    unsetText.mutate({ item: item.id, path, summary: t.catalog.summaryUnset(path) });
  };
  const label = (a: string) => {
    const d = defs.get(a);
    return d ? localized(d.label, d.label_en) || a : a;
  };
  const reset = (a: string) =>
    readOnly ? null : <ResetButton title={t.catalog.reset} onClick={() => unset(a)} />;
  const entries = Object.entries(item.values);
  return (
    <div className="space-y-2">
      <PropertyList>
        {entries.length === 0 && (
          <p className="px-3 py-2 text-ink-3 text-xs">{t.catalog.noValues}</p>
        )}
        {entries.map(([a, v]) => (
          <ValueRow
            key={a}
            item={item}
            attr={a}
            value={v}
            label={label(a)}
            dimension={defs.get(a)?.dimension ?? scalarOf(v)?.dimension ?? null}
            readOnly={readOnly}
            action={reset(a)}
            onSet={set}
          />
        ))}
      </PropertyList>
      {!readOnly && <CatalogAddValue design={design} item={item} onAdd={set} />}
    </div>
  );
}

/** 値 1 つの行。状態ごとの値は状態ごとの子の行に分け、それぞれ直接編集できる（部品の AttrRow と同じ形） */
function ValueRow({
  item,
  attr,
  value: v,
  label,
  dimension,
  readOnly,
  action,
  onSet,
}: {
  item: CatalogItemView;
  attr: string;
  value: ValueView;
  label: string;
  dimension: string | null;
  readOnly: boolean;
  action: ReactNode;
  onSet: (attr: string, state: string | null, text: string) => void;
}) {
  if (v.value.type === "by_state") {
    const states = item.states.length > 0 ? item.states : Object.keys(v.value.states);
    return (
      <div className="py-1">
        <PropertyRow layout="action" label={label} title={attr} action={action}>
          <span className="truncate text-[11px] text-ink-4" title={valueText(v)}>
            {valueText(v)}
          </span>
        </PropertyRow>
        {states.map((s) => {
          const q = stateValue(v, s);
          return (
            <PropertyRow key={s} layout="action" label={s} indent>
              <EditableQuantity
                display={q ? q.display : "—"}
                initial={quantityInputText(q)}
                dimension={dimension}
                label={paramPath(item, attr, s)}
                muted={!q}
                disabled={readOnly}
                onCommit={(_q, text) => onSet(attr, s, text)}
              />
            </PropertyRow>
          );
        })}
      </div>
    );
  }
  return (
    <PropertyRow layout="action" label={label} title={attr} action={action}>
      {v.value.type === "scalar" ? (
        <EditableQuantity
          display={valueText(v)}
          initial={quantityInputText(scalarOf(v))}
          dimension={dimension}
          label={paramPath(item, attr)}
          disabled={readOnly}
          onCommit={(_q, text) => onSet(attr, null, text)}
        />
      ) : (
        <span className="block truncate font-mono text-[12px]" title={valueText(v)}>
          {valueText(v)}
        </span>
      )}
    </PropertyRow>
  );
}
