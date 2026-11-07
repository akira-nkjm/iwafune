/**
 * TickBar の何本を塗るか。0 より大きい比率は最低 1 本、1 未満は全部は塗らない
 * （「少し」が「なし」に、「ほぼ」が「全部」に見えないように）。
 */
export function filledTicks(ratio: number, ticks: number): number {
  if (!Number.isFinite(ratio) || ratio <= 0 || ticks <= 0) return 0;
  if (ratio >= 1) return ticks;
  return Math.min(ticks - 1, Math.max(1, Math.round(ratio * ticks)));
}
