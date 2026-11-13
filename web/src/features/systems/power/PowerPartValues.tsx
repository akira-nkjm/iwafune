import { useAttrEdit } from "@/api/ops";
import type { DefsView, PartRow } from "@/api/types";
import { attrDef, attrLabel } from "@/lib/design";
import { AttrValueEditor } from "@/quantity/AttrValueEditor";

/** 部品と同じ編集経路で状態ごとの単体値も表示する。 */
export function PowerPartValues({
  part,
  attrs,
  defs,
}: {
  part: PartRow;
  attrs: string[];
  defs: DefsView;
}) {
  const edit = useAttrEdit({ kind: "part", owner: part });
  return (
    <dl className="mt-2 grid gap-3 sm:grid-cols-2 xl:grid-cols-3">
      {attrs.map((attr) => {
        const value = part.values[attr];
        const states = value?.value.type === "by_state" ? Object.keys(value.value.states) : [null];
        return states.map((state) => (
          <div key={`${attr}:${state}`}>
            <dt className="text-xs text-ink-3">
              {attrLabel(defs, attr)} {state && <span className="font-mono">{state}</span>}
            </dt>
            <dd className="mt-1 text-sm">
              <AttrValueEditor
                layout="cell"
                owner={part}
                attr={attr}
                state={state}
                def={attrDef(defs, attr)}
                value={value}
                onCommit={(state, text) => edit.set(attr, state, text)}
              />
            </dd>
          </div>
        ));
      })}
    </dl>
  );
}
