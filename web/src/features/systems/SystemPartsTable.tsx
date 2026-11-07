import { Link } from "react-router";
import { setValueOp } from "@/api/op-values";
import { useApplyOps } from "@/api/ops";
import type { DesignView, PartRow, QuantityView, ValueView } from "@/api/types";
import { localized, useT } from "@/i18n";
import { DataTable } from "@/kit/DataTable";
import { Empty } from "@/kit/Empty";
import { SubsystemDot } from "@/kit/SubsystemDot";
import { Td } from "@/kit/Td";
import { Th } from "@/kit/Th";
import { paramPath } from "@/lib/design";
import { EditableQuantity } from "@/quantity/EditableQuantity";
import { quantityInputText, scalarOf, stateValue, valueText } from "@/quantity/format";
import { ByStateCell } from "./ByStateCell";
import { relevantParts } from "./relevant";
import type { SystemSpec } from "./systems";

/** 状態ごとの属性は、状態の数がこれ以下なら状態ごとの列に開く（電力の off / on / tx …） */
const MAX_STATE_COLUMNS = 6;

type Column = { attr: string; state: string | null; byState: boolean };

function columnsOf(design: DesignView, parts: PartRow[], attrs: string[]): Column[] {
  const defs = new Map(design.defs.attributes.map((a) => [a.id, a]));
  return attrs.flatMap((a): Column[] => {
    const byState =
      defs.get(a)?.by_state || parts.some((p) => p.values[a]?.value.type === "by_state");
    if (!byState) return [{ attr: a, state: null, byState: false }];
    const states = [
      ...new Set(parts.filter((p) => p.values[a] !== undefined).flatMap((p) => p.states)),
    ];
    if (states.length === 0 || states.length > MAX_STATE_COLUMNS)
      return [{ attr: a, state: null, byState: true }];
    return states.map((s) => ({ attr: a, state: s, byState: true }));
  });
}

/** 系のページの「部品 × この系の属性」の表。セルで直接編集（SetValue）。状態ごとの値は状態の列に開く */
export function SystemPartsTable({ design, spec }: { design: DesignView; spec: SystemSpec }) {
  const t = useT();
  const apply = useApplyOps();
  const { parts, attrs } = relevantParts(design, spec);
  if (parts.length === 0) return <Empty>{t.systems.noParts}</Empty>;
  const defs = new Map(design.defs.attributes.map((a) => [a.id, a]));
  const cols = columnsOf(design, parts, attrs);
  const set = (part: string, attr: string, state: string | null, q: QuantityView, t: string) =>
    apply.mutate({
      ops: [setValueOp(part, attr, state, q)],
      summary: `${paramPath(design.parts.find((p) => p.id === part) ?? { paths: {} }, attr, state)} = ${t}`,
    });
  const dimOf = (a: string, v: ValueView | undefined) =>
    defs.get(a)?.dimension ?? scalarOf(v)?.dimension ?? null;

  return (
    <DataTable
      head={
        <>
          <Th>{t.systems.part}</Th>
          <Th>{t.systems.kind}</Th>
          {cols.map((c) => (
            <Th key={`${c.attr}:${c.state ?? ""}`} title={c.state ? c.state : c.attr}>
              {localized(defs.get(c.attr)?.label, defs.get(c.attr)?.label_en) || c.attr}
              {c.state && <span className="ml-1 font-mono text-ink-4">{c.state}</span>}
            </Th>
          ))}
        </>
      }
    >
      {parts.map((p) => (
        <tr key={p.id} className="hover:bg-panel-2/60">
          <Td>
            <Link
              to={`/design/components/${p.id}`}
              className="flex items-center gap-2 hover:underline"
            >
              <SubsystemDot group={p.subsystem} role={p.role} />
              <span className="font-mono text-[12.5px]">{p.id}</span>
            </Link>
          </Td>
          <Td mono className="text-ink-3">
            {p.kind}
          </Td>
          {cols.map((c) => {
            const v = p.values[c.attr];
            const key = `${c.attr}:${c.state ?? ""}`;
            const dim = dimOf(c.attr, v);
            if (c.byState && c.state) {
              if (!p.states.includes(c.state)) return <Td key={key} className="text-ink-4" />;
              const q = stateValue(v, c.state);
              const path = paramPath(p, c.attr, c.state);
              return (
                <Td key={key}>
                  <EditableQuantity
                    display={q?.display ?? "—"}
                    initial={quantityInputText(q)}
                    dimension={dim}
                    label={path}
                    muted={!q}
                    onCommit={(q2, t) => set(p.id, c.attr, c.state, q2, t)}
                  />
                </Td>
              );
            }
            if (c.byState)
              return (
                <Td key={key}>
                  <ByStateCell part={p} attr={c.attr} value={v} dimension={dim} />
                </Td>
              );
            const path = paramPath(p, c.attr);
            const editable = !v || v.value.type === "scalar" || v.value.type === "unset";
            return (
              <Td key={key}>
                {editable ? (
                  <EditableQuantity
                    display={valueText(v)}
                    initial={quantityInputText(scalarOf(v))}
                    dimension={dim}
                    label={path}
                    muted={!v || v.origin === "unset"}
                    onCommit={(q2, t) => set(p.id, c.attr, null, q2, t)}
                  />
                ) : (
                  <span className="font-mono text-[12px]">{valueText(v)}</span>
                )}
              </Td>
            );
          })}
        </tr>
      ))}
    </DataTable>
  );
}
