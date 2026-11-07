import { useCallback, useState } from "react";
import type { CatalogItemView, DesignView } from "@/api/types";
import { localized, useT } from "@/i18n";
import { NativeSelect } from "@/kit/NativeSelect";
import { QuantityInput } from "@/quantity/QuantityInput";
import { Button } from "@/ui/button";

/** カタログ品に値を足す行（属性・状態・値）。確定は呼び手が打った文字列のまま service に渡す */
export function CatalogAddValue({
  design,
  item,
  onAdd,
}: {
  design: DesignView;
  item: CatalogItemView;
  onAdd: (attr: string, state: string | null, text: string) => void;
}) {
  const t = useT();
  const states = item.states;
  const [attr, setAttr] = useState("");
  const [state, setState] = useState(states[0] ?? "");
  const [text, setText] = useState<string | null>(null);
  const onValid = useCallback(
    (typed: string, ok: boolean) => setText(ok ? typed.trim() : null),
    [],
  );
  const def = design.defs.attributes.find((a) => a.id === attr);
  return (
    <div className="grid grid-cols-[1fr_90px_1fr_auto] items-start gap-2">
      <NativeSelect
        size="sm"
        aria-label={t.catalog.attr}
        value={attr}
        placeholder={t.catalog.attr}
        onChange={(e) => setAttr(e.target.value)}
        options={design.defs.attributes
          .filter((a) => a.value_type === "scalar")
          .map((a) => ({ value: a.id, label: `${a.id}（${localized(a.label, a.label_en)}）` }))}
      />
      <NativeSelect
        size="sm"
        aria-label={t.catalog.state}
        value={def?.by_state ? state : ""}
        disabled={!def?.by_state}
        onChange={(e) => setState(e.target.value)}
        placeholder="—"
        options={states.map((s) => ({ value: s, label: s }))}
      />
      <QuantityInput
        key={attr}
        initial=""
        dimension={def?.dimension ?? null}
        label={t.catalog.value}
        onChangeValid={onValid}
      />
      <Button
        size="sm"
        disabled={!attr || !text || (def?.by_state && !state)}
        onClick={() => text && onAdd(attr, def?.by_state ? state : null, text)}
      >
        {t.catalog.addValue}
      </Button>
    </div>
  );
}
