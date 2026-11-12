import { Fragment } from "react";
import { useCommandText } from "@/api/ops";
import type { BudgetView, ModeHeader } from "@/api/types";
import { useT } from "@/i18n";
import { CommitInput } from "@/kit/forms/CommitInput";
import { SystemDot } from "@/kit/SystemDot";
import { cn } from "@/lib/cn";
import { nodeName } from "@/lib/design";
import type { DraftComputed } from "../draft/use-mode-draft-load";
import { ModeBudgetRows } from "./ModeBudgetRows";
import { ModeCell } from "./ModeCell";
import { ModeHeaderMenu } from "./ModeHeaderMenu";
import { STICKY } from "./matrix";
import { MatrixFocus } from "./matrix-focus";
import type { MatrixGroup, MatrixRow, OnCell } from "./matrix-rows";

/**
 * 「部品 × モード」の行列の表示。行（`matrixGroups` が作ったサブシステムごとの部品の行）と、足もとのバジェットを
 * 受け取って並べるだけ（問い合わせと行への変換は `useModeDraftLoad`・`matrixGroups`）。セルの間の矢印キーの移動は
 * `MatrixFocus` が受け持つ。スクロールしても読めるよう、見出しの行は上、部品の列は左、足もとの行は下に貼り付ける（STICKY）。
 */
export function ModesTable({
  modes,
  groups,
  budget,
  updating,
  onCell,
}: {
  /** 列（モード）の見出し */
  modes: ModeHeader[];
  /** サブシステムごとの部品の行（`matrixGroups`） */
  groups: MatrixGroup[];
  /** 足もとの合計・使用可能電力・余裕（下書きを当てた値） */
  budget: DraftComputed<BudgetView | undefined>;
  /** セルの電力を下書きで問い合わせている間（前の数字を薄くして出しておく） */
  updating: boolean;
  /** セルを書き換えたとき（下書きにする） */
  onCell: OnCell;
}) {
  const tm = useT().modes;
  return (
    <table
      className={cn(
        "w-full border-separate border-spacing-0 text-sm [&_.num]:transition-opacity",
        updating && "[&_tbody_.num]:opacity-50",
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
          {modes.map((m) => (
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
        <MatrixFocus>
          {groups.map((g) => (
            <Fragment key={g.group ?? ""}>
              {/* 系の見出しも部品の列に置く（表の幅いっぱいの 1 セルにすると、固定した列の境目の線がここで切れる） */}
              <tr>
                <th
                  scope="rowgroup"
                  className={cn(
                    STICKY.rowHead,
                    "whitespace-nowrap px-4 pt-4 pb-1 text-left font-medium text-ink-3 text-xs",
                  )}
                >
                  <span className="flex items-center gap-2">
                    <SystemDot system={g.group} />
                    {g.label}
                  </span>
                </th>
                <td colSpan={modes.length + 1} />
              </tr>
              {g.rows.map((r) => (
                <MatrixRowCells key={r.part} row={r} onCell={onCell} />
              ))}
            </Fragment>
          ))}
        </MatrixFocus>
      </tbody>
      <tfoot className={`${STICKY.foot} bg-panel-2`}>
        <ModeBudgetRows modes={modes} budget={budget} />
      </tfoot>
    </table>
  );
}

/** 列見出しのモードの名前。日本語は設計に書いた表示名の下に ID、英語（と表示名が無いとき）は ID だけ */
function ModeName({ mode }: { mode: ModeHeader }) {
  const name = nodeName(mode);
  const edit = useCommandText();
  const t = useT().modes;
  return (
    <div>
      <div className="font-medium text-ink">{name}</div>
      {name !== mode.id && <div className="font-mono text-[10px] text-ink-4">{mode.id}</div>}
      <CommitInput
        label={t.maxDuration}
        value={mode.max_duration?.display ?? ""}
        onCommit={(duration) =>
          edit.mutate({
            command: { command: "mode_duration", mode: mode.id, duration: duration || null },
            summary: t.durationSummary(mode.id),
          })
        }
      />
    </div>
  );
}

/** 部品 1 つの行（部品の名前・電力レベル・モードごとのセル） */
function MatrixRowCells({ row, onCell }: { row: MatrixRow; onCell: OnCell }) {
  return (
    <tr>
      <th scope="row" className={`${STICKY.rowHead} px-4 py-1 text-left font-normal`}>
        <div className="font-mono text-xs">
          {row.part}
          {row.count > 1 ? ` ×${row.count}` : ""}
        </div>
        <div className="text-[10px] text-ink-4">{row.kind}</div>
      </th>
      <td
        className="num whitespace-nowrap px-3 py-1 text-right text-ink-3 text-xs"
        title={row.levelsTitle}
      >
        {row.levels}
      </td>
      {row.cells.map((c) => (
        <td key={c.mode} className="px-1 py-0.5">
          <ModeCell cell={c} onCell={onCell} />
        </td>
      ))}
    </tr>
  );
}
