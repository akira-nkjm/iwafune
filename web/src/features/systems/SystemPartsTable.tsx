import { Link } from "react-router";
import { useAttrEdit } from "@/api/ops";
import type { DesignView, PartRow, System } from "@/api/types";
import { useT } from "@/i18n";
import { DataTable } from "@/kit/DataTable";
import { Empty } from "@/kit/Empty";
import { SubsystemDot } from "@/kit/SubsystemDot";
import { Td } from "@/kit/Td";
import { Th } from "@/kit/Th";
import { attrDef, attrLabel } from "@/lib/design";
import { paths } from "@/lib/nav";
import { AttrValueEditor } from "@/quantity/AttrValueEditor";
import { relevantParts } from "./relevant";

/** 状態ごとの属性は、状態の数がこれ以下なら状態ごとの列に開く（電力の off / on / tx …） */
const MAX_STATE_COLUMNS = 6;

type Column = { attr: string; state: string | null; byState: boolean };

function columnsOf(design: DesignView, parts: PartRow[], attrs: string[]): Column[] {
  return attrs.flatMap((a): Column[] => {
    const byState =
      attrDef(design.defs, a)?.by_state ||
      parts.some((p) => p.values[a]?.value.type === "by_state");
    if (!byState) return [{ attr: a, state: null, byState: false }];
    const states = [
      ...new Set(parts.filter((p) => p.values[a] !== undefined).flatMap((p) => p.states)),
    ];
    if (states.length === 0 || states.length > MAX_STATE_COLUMNS)
      return [{ attr: a, state: null, byState: true }];
    return states.map((s) => ({ attr: a, state: s, byState: true }));
  });
}

/**
 * 系のページの「部品 × この系の属性」の表。セルで直接直す（部品の詳細と同じ `AttrValueEditor` と `useAttrEdit`）。
 * 状態ごとの値は状態の列に開く
 */
export function SystemPartsTable({ design, system }: { design: DesignView; system: System }) {
  const t = useT();
  const { parts, attrs } = relevantParts(design, system);
  if (parts.length === 0) return <Empty>{t.systems.noParts}</Empty>;
  const cols = columnsOf(design, parts, attrs);
  return (
    <DataTable
      head={
        <>
          <Th>{t.systems.part}</Th>
          <Th>{t.systems.kind}</Th>
          {cols.map((c) => (
            <Th key={`${c.attr}:${c.state ?? ""}`} title={c.state ? c.state : c.attr}>
              {attrLabel(design.defs, c.attr)}
              {c.state && <span className="ml-1 font-mono text-ink-4">{c.state}</span>}
            </Th>
          ))}
        </>
      }
    >
      {parts.map((p) => (
        <PartCells key={p.id} design={design} part={p} cols={cols} />
      ))}
    </DataTable>
  );
}

/** 部品 1 つの行 */
function PartCells({ design, part, cols }: { design: DesignView; part: PartRow; cols: Column[] }) {
  const edit = useAttrEdit({ kind: "part", owner: part });
  return (
    <tr className="hover:bg-panel-2/60">
      <Td>
        <Link to={paths.component(part.id)} className="flex items-center gap-2 hover:underline">
          <SubsystemDot group={part.subsystem} />
          <span className="font-mono text-[12.5px]">{part.id}</span>
        </Link>
      </Td>
      <Td mono className="text-ink-3">
        {part.kind}
      </Td>
      {cols.map((c) => {
        const key = `${c.attr}:${c.state ?? ""}`;
        // 状態の列で、この部品の種別にその状態が無ければ空
        if (c.state && !part.states.includes(c.state))
          return <Td key={key} className="text-ink-4" />;
        return (
          <Td key={key}>
            <AttrValueEditor
              layout="cell"
              owner={part}
              attr={c.attr}
              state={c.state}
              def={attrDef(design.defs, c.attr)}
              value={part.values[c.attr]}
              onCommit={(state, text) => edit.set(c.attr, state, text)}
            />
          </Td>
        );
      })}
    </tr>
  );
}
