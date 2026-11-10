import { Fragment } from "react";
import { analysisOutput } from "@/api/analysis-output";
import type { DesignView, ModeHeader, ModeLoadOutput, ModeRow, ModeTable, Op } from "@/api/types";
import { useAnalyses } from "@/features/systems/use-analyses";
import { useT } from "@/i18n";
import { SubsystemDot } from "@/kit/SubsystemDot";
import { cn } from "@/lib/cn";
import { kindLabel, nodeLabel, nodeName, partById } from "@/lib/design";
import { cellOf, type ModeDraft, withCell } from "./draft";
import { ModeBudgetRows } from "./ModeBudgetRows";
import { ModeCell } from "./ModeCell";
import { ModeHeaderMenu } from "./ModeHeaderMenu";
import { powerLevels, STICKY, savedCellText } from "./matrix";

/**
 * 「部品 × モード」の行列。行はサブシステム（部品の Group）ごとにまとめ、電力を使う部品だけを出す。
 * セルの電力は power.mode_load の出力、足もとの合計と余裕は電力のバジェットの値。
 * スクロールしても読めるよう、見出しの行は上、部品の列は左、足もとの行は下に貼り付ける（STICKY）。
 */
export function ModesTable({
  design,
  table,
  draft,
  ops,
  onDraft,
}: {
  design: DesignView;
  table: ModeTable;
  draft: ModeDraft;
  /** 下書きを service が Op にしたもの（無ければ空）。これを書き込まずに当てて電力と合計を計算する */
  ops: readonly Op[];
  onDraft: (d: ModeDraft) => void;
}) {
  const tm = useT().modes;
  // 下書きを当てた電力と合計（書き込まずにサーバーが計算する）
  const [load] = useAnalyses(["power.mode_load"], null, ops);
  const output = analysisOutput(load?.result, "power.mode_load");
  const parts = partById(design);
  const groupOf = (row: ModeRow) => parts.get(row.part)?.subsystem ?? null;
  const rows = table.rows
    .filter((r) => powerLevels(parts.get(r.part)).some(([, q]) => q.si > 0))
    .sort(
      (a, b) => (groupOf(a) ?? "").localeCompare(groupOf(b) ?? "") || a.part.localeCompare(b.part),
    );
  return (
    <table
      className={cn(
        "w-full border-separate border-spacing-0 text-sm [&_.num]:transition-opacity",
        // 下書きの電力を問い合わせている間は、前の数字を薄くして出しておく
        load?.updating && "[&_tbody_.num]:opacity-50",
      )}
    >
      <thead className="text-ink-3 text-xs">
        <tr>
          <th scope="col" className={`${STICKY.corner} min-w-48 px-4 py-3 text-left font-medium`}>
            {tm.part}
          </th>
          <th scope="col" className={`${STICKY.head} min-w-36 px-3 py-3 text-right font-medium`}>
            {tm.powerLevels}
          </th>
          {table.modes.map((m) => (
            <th
              key={m.id}
              scope="col"
              title={m.description ?? undefined}
              className={`${STICKY.head} min-w-32 px-2 py-3 font-normal`}
            >
              <div className="flex items-center justify-center gap-1">
                <ModeName mode={m} />
                <ModeHeaderMenu mode={m} />
              </div>
            </th>
          ))}
        </tr>
      </thead>
      <tbody>
        {rows.map((r, index) => {
          const group = groupOf(r);
          const first = index === 0 || groupOf(rows[index - 1] as ModeRow) !== group;
          return (
            <Fragment key={r.part}>
              {first && (
                // 系の見出しも部品の列に置く（表の幅いっぱいの 1 セルにすると、固定した列の境目の線がここで切れる）
                <tr>
                  <th
                    scope="rowgroup"
                    className={cn(
                      STICKY.rowHead,
                      "whitespace-nowrap px-4 pt-4 pb-1 text-left font-medium text-ink-3 text-xs",
                    )}
                  >
                    <span className="flex items-center gap-2">
                      <SubsystemDot group={group} />
                      {nodeLabel(design, group)}
                    </span>
                  </th>
                  <td colSpan={table.modes.length + 1} />
                </tr>
              )}
              <MatrixRow
                design={design}
                table={table}
                row={r}
                index={index}
                draft={draft}
                output={output}
                onDraft={onDraft}
              />
            </Fragment>
          );
        })}
      </tbody>
      <tfoot className={`${STICKY.foot} bg-panel-2`}>
        <ModeBudgetRows modes={table.modes} ops={ops} />
      </tfoot>
    </table>
  );
}

/** 列見出しのモードの名前。日本語は設計に書いた表示名の下に ID、英語（と表示名が無いとき）は ID だけ */
function ModeName({ mode }: { mode: ModeHeader }) {
  const name = nodeName(mode);
  return (
    <div>
      <div className="font-medium text-ink">{name}</div>
      {name !== mode.id && <div className="font-mono text-[10px] text-ink-4">{mode.id}</div>}
    </div>
  );
}

function MatrixRow({
  design,
  table,
  row,
  index,
  draft,
  output,
  onDraft,
}: {
  design: DesignView;
  table: ModeTable;
  row: ModeRow;
  index: number;
  draft: ModeDraft;
  output: ModeLoadOutput | null;
  onDraft: (d: ModeDraft) => void;
}) {
  const part = design.parts.find((p) => p.id === row.part);
  const levels = powerLevels(part);
  const words = useT().modes;
  return (
    <tr>
      <th scope="row" className={`${STICKY.rowHead} px-4 py-1 text-left font-normal`}>
        <div className="font-mono text-xs">
          {row.part}
          {part && part.count > 1 ? ` ×${part.count}` : ""}
        </div>
        <div className="text-[10px] text-ink-4">{row.kind && kindLabel(design.defs, row.kind)}</div>
      </th>
      <td
        className="num whitespace-nowrap px-3 py-1 text-right text-ink-3 text-xs"
        title={levels.map(([state, q]) => `${state}: ${q.display}`).join(" / ")}
      >
        {levels.map(([, q]) => q.display).join(" / ")}
      </td>
      {table.modes.map((m, column) => {
        const cell = cellOf(table, draft, m.id, row.part);
        const power =
          output?.modes.find((mode) => mode.id === m.id)?.parts.find((p) => p.part === row.part)
            ?.mean ?? null;
        return (
          <td key={m.id} className="px-1 py-0.5">
            <ModeCell
              part={row.part}
              mode={m.id}
              row={index}
              column={column}
              states={row.states}
              cell={cell}
              display={savedCellText(row, m.id, words)}
              changed={draft.get(m.id)?.has(row.part) ?? false}
              power={power}
              shade={levels.findIndex(([state]) => state === cell?.state)}
              onChange={(c) => onDraft(withCell(table, draft, m.id, row.part, c))}
            />
          </td>
        );
      })}
    </tr>
  );
}
