import { useAttrEdit } from "@/api/ops";
import type { DefsView, PartRow } from "@/api/types";
import { useT } from "@/i18n";
import { PropertyList } from "@/kit/detail/PropertyList";
import { ResetButton } from "@/kit/forms/ResetButton";
import { attrDef, attrLabel } from "@/lib/design";
import { AttrValueEditor } from "@/quantity/AttrValueEditor";
import { OriginBadge } from "@/quantity/OriginBadge";

/**
 * 1 区分の属性の表（ラベル / 値 / 出所 / 戻す）。打った文字列は部品のパラメーターパスのまま `fune set` と同じ service に
 * 渡し（`useAttrEdit`）、上書きを消すと `fune unset` と同じくカタログの値に戻る
 */
export function AttrTable({
  defs,
  part,
  attrIds,
}: {
  /** 属性の定義（値の型・次元・表示名） */
  defs: DefsView;
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
            def={attrDef(defs, id)}
            value={v}
            label={attrLabel(defs, id)}
            source={<OriginBadge origin={origin} source={v?.source} evidence={v?.evidence} />}
            action={
              origin === "override" ? (
                <ResetButton title={t.resetTitle} onClick={() => edit.unset(id)} />
              ) : null
            }
            onCommit={(state, text) => edit.set(id, state, text)}
          />
        );
      })}
    </PropertyList>
  );
}
