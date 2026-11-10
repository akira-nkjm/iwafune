import type { ModeRow, PartRow } from "@/api/types";
import type { modesJa } from "@/i18n/modes.ja";
import type { CellText } from "./draft";

/** セルの文言の並べ方（i18n の `modes` の辞書） */
type CellWords = Pick<
  (typeof modesJa)["modes"],
  "cellShare" | "cellShareSeparator" | "cellWithShares"
>;

/**
 * 保存済みのセルの文言（`rx（tx 10.0 %）`）。状態は `ModeRow.cells`、割合の `%` は API の整形（`ModeRow.shares`）で、
 * 並べ方は i18n（`modes.cellShare`・`cellWithShares`）。そのモードに書かれていなければ undefined。
 */
export function savedCellText(row: ModeRow, mode: string, words: CellWords): string | undefined {
  const cell = row.cells[mode];
  if (!cell) return undefined;
  const shares = Object.entries(row.shares[mode] ?? {}).flatMap(([state, q]) =>
    q ? [words.cellShare(state, q.display)] : [],
  );
  if (shares.length === 0) return cell.state;
  return words.cellWithShares(cell.state, shares.join(words.cellShareSeparator));
}

/** セルを押してオンにするときの状態: 語彙に `on` があれば on、無ければ off 以外の最初の状態（送受信機なら rx） */
export function mainState(states: readonly string[]): string | null {
  if (states.includes("on")) return "on";
  return states.find((s) => s !== "off") ?? null;
}

/**
 * セルを押したときの次の状態。オフ（未指定・off。時間割合があっても）なら主な状態へ、それ以外なら off へ。
 * 時間割合は前の状態に対するものなので持ち越さない。ほかの状態（idle・peak など）はポップオーバーで選ぶ。
 * 語彙に off が無ければオンのまま。
 */
export function toggleCell(states: readonly string[], cell: CellText | null): CellText | null {
  const off = !cell || cell.state === "off";
  const next = off ? mainState(states) : states.includes("off") ? "off" : cell.state;
  return next ? { state: next, duty: {} } : cell;
}

/** セルの記号: ◐ 間欠（ほかの状態の時間割合がある）、● オン（off 以外）、— オフ・未指定 */
export function cellSymbol(cell: CellText | null): "●" | "◐" | "—" {
  if (!cell) return "—";
  if (Object.keys(cell.duty).length > 0) return "◐";
  return cell.state === "off" ? "—" : "●";
}

/**
 * 部品の状態ごとの電力（API の量）を小さい順に。「電力レベル」の列と、セルの濃さの段階に使う
 * （並べるだけで、値の計算はしない）。電力が 1 つの値なら `on` の 1 段だけ。
 */
export function powerLevels(part: PartRow | undefined) {
  const value = part?.values.power?.value;
  if (value?.type === "by_state") {
    return Object.entries(value.states).sort(([, a], [, b]) => a.si - b.si);
  }
  if (value?.type === "scalar") return [["on", value.nominal] as const];
  return [];
}

/** オンのセルの塗り。電力の段階が高い状態ほど濃くする（どのモードで何を回しているかを色で読む） */
export const ON_SHADES = ["bg-accent/8", "bg-accent/15", "bg-accent/25", "bg-accent/35"] as const;

/**
 * 行列の中で貼り付ける列・行（見出しの行は上、部品の列は左、足もとの行は下）。位置と重なり順に加えて
 * 地の色を塗り、下を流れるセルを隠す。
 */
export const STICKY = {
  head: "sticky top-0 z-20 bg-panel-2",
  corner: "sticky top-0 left-0 z-30 border-line border-r bg-panel-2",
  rowHead: "sticky left-0 z-10 border-line border-r bg-panel",
  foot: "sticky bottom-0 z-20",
} as const;
