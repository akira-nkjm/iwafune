import { useCallback, useState } from "react";
import type { CatalogItemView, DefsView } from "@/api/types";
import { useT } from "@/i18n";
import { NativeSelect } from "@/kit/forms/NativeSelect";
import { attrDef, defWithId } from "@/lib/design";
import { QuantityInput } from "@/quantity/QuantityInput";
import { Button } from "@/ui/button";

/** カタログ品に値を足す行（属性・状態・値）。確定は呼び手が打った文字列のまま service に渡す */
export function CatalogAddValue({
  defs,
  item,
  onCommit,
}: {
  /** 属性の定義（足せる属性の選択肢と、値の次元） */
  defs: DefsView;
  item: CatalogItemView;
  /** 確定: 属性・状態（状態ごとでなければ null）・打った文字列（`useAttrEdit` の `set` に渡す） */
  onCommit: (attr: string, state: string | null, text: string) => void;
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
  const def = attrDef(defs, attr);
  return (
    <div className="grid grid-cols-[1fr_90px_1fr_auto] items-start gap-2">
      <NativeSelect
        size="sm"
        aria-label={t.catalog.attr}
        value={attr}
        placeholder={t.catalog.attr}
        onChange={(e) => setAttr(e.target.value)}
        options={defs.attributes
          .filter((a) => a.value_type === "scalar")
          .map((a) => ({ value: a.id, label: defWithId(a) }))}
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
        onClick={() => text && onCommit(attr, def?.by_state ? state : null, text)}
      >
        {t.catalog.addValue}
      </Button>
    </div>
  );
}
