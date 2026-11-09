import { useAttrEdit } from "@/api/ops";
import type { DesignView, PartRow } from "@/api/types";
import { useT } from "@/i18n";
import { PropertyList } from "@/kit/PropertyList";
import { ResetButton } from "@/kit/ResetButton";
import { attrDef, attrLabel } from "@/lib/design";
import { AttrValueEditor } from "@/quantity/AttrValueEditor";
import { OriginBadge } from "@/quantity/OriginBadge";

/**
 * 1 区分の属性の表（ラベル / 値 / 出所 / 戻す）。打った文字列は部品のパラメーターパスのまま `fune set` と同じ service に
 * 渡し（`useAttrEdit`）、上書きを消すと `fune unset` と同じくカタログの値に戻る
 */
export function AttrTable({
  design,
  part,
  attrIds,
}: {
  design: DesignView;
  part: PartRow;
  attrIds: string[];
}) {
  const t = useT().components.attr;
  const edit = useAttrEdit({ kind: "part", owner: part });
  return (
    <PropertyList>
      {attrIds.map((id) => {
        const v = part.values[id];
        const origin = v?.origin ?? "unset";
        return (
          <AttrValueEditor
            key={id}
            layout="row"
            rowLayout="values"
            owner={part}
            attr={id}
            def={attrDef(design.defs, id)}
            value={v}
            label={attrLabel(design.defs, id)}
            source={<OriginBadge origin={origin} source={v?.source} evidence={v?.evidence} />}
            action={
              origin === "override" ? (
                <ResetButton title={t.resetTitle} onClick={() => edit.unset(id)} />
              ) : null
            }
            onSet={(state, text) => edit.set(id, state, text)}
          />
        );
      })}
    </PropertyList>
  );
}
