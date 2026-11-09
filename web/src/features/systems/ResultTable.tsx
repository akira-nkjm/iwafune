import { ChevronRight } from "lucide-react";
import { Fragment, useState } from "react";
import type { Verdict } from "@/api/types";
import { useT } from "@/i18n";
import { DataTable } from "@/kit/DataTable";
import { Td } from "@/kit/Td";
import { Th } from "@/kit/Th";
import { VerdictPill } from "@/kit/VerdictPill";
import { FieldValue } from "./FieldValue";
import { fieldLabel } from "./fields";
import { type JsonObject, type JsonPath, jsonPointer, nestedTables, tableColumns } from "./output";

const isVerdict = (v: unknown): v is Verdict => v === "ok" || v === "warning" || v === "violation";

/** 解析の出力の「オブジェクトの配列」を表にする。行の中の入れ子の配列は行を開いて見る */
export function ResultTable({
  rows,
  at,
  dims,
  depth = 0,
}: {
  rows: JsonObject[];
  /** この表の出力の中の位置（`["cases"]`）。セルの次元を dims から引くのに使う（pointer は `jsonPointer` が作る） */
  at: JsonPath;
  dims: Record<string, string> | undefined;
  depth?: number;
}) {
  const tr = useT();
  const cols = tableColumns(rows);
  const [open, setOpen] = useState<Set<number>>(new Set());
  const hasNested = rows.some((r) => nestedTables(r).length > 0);
  const toggle = (i: number) => {
    const next = new Set(open);
    if (next.has(i)) next.delete(i);
    else next.add(i);
    setOpen(next);
  };
  return (
    <DataTable
      className={depth > 0 ? "rounded-control border border-line" : undefined}
      head={
        <>
          {hasNested && <Th className="w-6" />}
          {cols.map((c) => (
            <Th key={c} align={typeof rows[0]?.[c] === "number" ? "right" : "left"} title={c}>
              {fieldLabel(c)}
            </Th>
          ))}
        </>
      }
    >
      {rows.map((r, i) => {
        const nested = nestedTables(r);
        const key =
          typeof r.id === "string" ? r.id : typeof r.part === "string" ? r.part : String(i);
        return (
          // biome-ignore lint/suspicious/noArrayIndexKey: 解析の出力の行は並びそのものが識別子（ID を持たない行もある）
          <Fragment key={`${key}-${i}`}>
            <tr className="hover:bg-panel-2/60">
              {hasNested && (
                <Td className="w-6 px-1">
                  {nested.length > 0 && (
                    <button
                      type="button"
                      aria-label={tr.systems.openBreakdown}
                      aria-expanded={open.has(i)}
                      onClick={() => toggle(i)}
                      className="rounded p-0.5 text-ink-3 hover:bg-panel-2"
                    >
                      <ChevronRight size={13} className={open.has(i) ? "rotate-90" : ""} />
                    </button>
                  )}
                </Td>
              )}
              {cols.map((c) => {
                const v = r[c];
                if (c === "verdict" && isVerdict(v))
                  return (
                    <Td key={c}>
                      <VerdictPill verdict={v} />
                    </Td>
                  );
                return (
                  <Td
                    key={c}
                    align={typeof v === "number" ? "right" : "left"}
                    mono={typeof v !== "number"}
                  >
                    <FieldValue
                      field={c}
                      value={v}
                      dimension={dims?.[jsonPointer([...at, i, c])]}
                    />
                  </Td>
                );
              })}
            </tr>
            {open.has(i) &&
              nested.map(([k, sub]) => (
                <tr key={k}>
                  <td colSpan={cols.length + 1} className="bg-panel-2/50 px-3 py-2">
                    <div className="mb-1 font-medium text-[11px] text-ink-3">
                      {fieldLabel(k)}（{sub.length}）
                    </div>
                    <ResultTable rows={sub} at={[...at, i, k]} dims={dims} depth={depth + 1} />
                  </td>
                </tr>
              ))}
          </Fragment>
        );
      })}
    </DataTable>
  );
}
