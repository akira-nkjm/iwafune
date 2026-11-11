import { useState } from "react";

export type SortDirection = "ascending" | "descending";
export type SortHeaderProps = { direction: SortDirection | "none"; onSort: () => void };
export type TableComparator<Row> = (a: Row, b: Row, direction: SortDirection) => number;

/** 欠損値は向きによらず末尾。比較する値と通常の昇順比較を列ごとに渡す。 */
export function tableComparator<Row, Value>(
  value: (row: Row) => Value | null | undefined,
  compare: (a: Value, b: Value) => number,
): TableComparator<Row> {
  return (a, b, direction) => {
    const av = value(a);
    const bv = value(b);
    if (av == null) return bv == null ? 0 : 1;
    if (bv == null) return -1;
    return compare(av, bv) * (direction === "ascending" ? 1 : -1);
  };
}

/** 列の昇順 → 降順 → 入力順。等しい値は入力順を保ち、入力配列は変更しない。 */
export function useTableSort<Row, Column extends string>(
  items: readonly Row[],
  comparators: Record<Column, TableComparator<Row>>,
) {
  const [sort, setSort] = useState<{ column: Column; direction: SortDirection } | null>(null);
  const rows = sort
    ? items
        .map((row, index) => ({ row, index }))
        .sort((a, b) => comparators[sort.column](a.row, b.row, sort.direction) || a.index - b.index)
        .map(({ row }) => row)
    : items;
  function headerProps(column: Column): { sort: SortHeaderProps } {
    return {
      sort: {
        direction: sort?.column === column ? sort.direction : "none",
        onSort: () =>
          setSort((current) => {
            if (current?.column !== column) return { column, direction: "ascending" };
            return current.direction === "ascending" ? { column, direction: "descending" } : null;
          }),
      },
    };
  }
  return { rows, headerProps };
}
