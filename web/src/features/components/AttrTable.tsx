import { setValueOp, unsetValueOp } from "@/api/op-values";
import { useApplyOps } from "@/api/ops";
import type { DesignView, PartRow, QuantityView } from "@/api/types";
import { useT } from "@/i18n";
import { PropertyList } from "@/kit/PropertyList";
import { paramPath } from "@/lib/design";
import { AttrRow } from "./AttrRow";

/** 1 区分の属性の表。編集は SetValue、上書きを消すのは UnsetValue（どちらも POST /ops） */
export function AttrTable({
  design,
  part,
  attrIds,
}: {
  design: DesignView;
  part: PartRow;
  attrIds: string[];
}) {
  const { mutate } = useApplyOps();
  const t = useT().components.attr;
  const onSet = (attr: string, state: string | null, q: QuantityView, text: string) =>
    mutate({
      ops: [setValueOp(part.id, attr, state, q)],
      summary: t.setSummary(paramPath(part, attr, state), text),
    });
  const onUnset = (attr: string) =>
    mutate({
      ops: [unsetValueOp(part.id, attr, null)],
      summary: t.unsetSummary(paramPath(part, attr)),
    });
  return (
    <PropertyList>
      {attrIds.map((id) => (
        <AttrRow
          key={id}
          part={part}
          id={id}
          def={design.defs.attributes.find((a) => a.id === id)}
          onSet={onSet}
          onUnset={onUnset}
        />
      ))}
    </PropertyList>
  );
}
