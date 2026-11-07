import type { AttributeDefView, PartRow, QuantityView, ValueView } from "@/api/types";
import { localized, useT } from "@/i18n";
import { PropertyRow } from "@/kit/PropertyRow";
import { ResetButton } from "@/kit/ResetButton";
import { paramPath } from "@/lib/design";
import { EditableQuantity } from "@/quantity/EditableQuantity";
import { quantityInputText, scalarOf, stateValue, valueText } from "@/quantity/format";
import { OriginBadge } from "@/quantity/OriginBadge";

/** 量として直接編集できる値か（ベクトル・列挙などは CLI か TOML で）。型は API の value_type */
function quantityLike(v: ValueView | undefined, valueType: string | undefined): boolean {
  const t = v?.value.type ?? "unset";
  if (t === "scalar" || t === "by_state") return true;
  return (
    t === "unset" && (valueType === undefined || valueType === "scalar" || valueType === "by_state")
  );
}

/** 属性 1 つの行（ラベル / 値 / 出所 / 戻す）。状態ごとの値は状態ごとの子の行に分け、それぞれ直接編集できる */
export function AttrRow({
  part,
  id,
  def,
  onSet,
  onUnset,
}: {
  part: PartRow;
  id: string;
  def: AttributeDefView | undefined;
  onSet: (attr: string, state: string | null, q: QuantityView, text: string) => void;
  onUnset: (attr: string) => void;
}) {
  const t = useT().components.attr;
  const v = part.values[id];
  const label = def ? localized(def.label, def.label_en) || id : id;
  const dimension = def?.dimension ?? scalarOf(v)?.dimension ?? null;
  const origin = v?.origin ?? "unset";
  const reset =
    origin === "override" ? <ResetButton title={t.resetTitle} onClick={() => onUnset(id)} /> : null;
  const badge = <OriginBadge origin={origin} source={v?.source} evidence={v?.evidence} />;
  if (v?.value.type === "by_state" || def?.by_state) {
    return (
      <div className="py-1">
        <PropertyRow layout="values" label={label} title={id} source={badge} action={reset}>
          <span className="text-[11px] text-ink-4">{t.byState}</span>
        </PropertyRow>
        {part.states.map((s) => {
          const q = stateValue(v, s);
          const path = paramPath(part, id, s);
          return (
            <PropertyRow key={s} layout="values" label={s} indent className="py-0.5">
              <EditableQuantity
                display={q ? q.display : "—"}
                initial={quantityInputText(q)}
                dimension={dimension}
                label={path}
                muted={!q}
                onCommit={(qv, text) => onSet(id, s, qv, text)}
              />
            </PropertyRow>
          );
        })}
      </div>
    );
  }
  return (
    <PropertyRow layout="values" label={label} title={id} source={badge} action={reset}>
      {quantityLike(v, def?.value_type) ? (
        <EditableQuantity
          display={valueText(v)}
          initial={quantityInputText(scalarOf(v))}
          dimension={dimension}
          label={paramPath(part, id)}
          muted={origin === "unset"}
          onCommit={(qv, text) => onSet(id, null, qv, text)}
        />
      ) : (
        <span className="block truncate font-mono text-[12px]" title={valueText(v)}>
          {valueText(v)}
        </span>
      )}
    </PropertyRow>
  );
}
