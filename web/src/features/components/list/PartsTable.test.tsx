import type { ComponentProps, ReactElement, ReactNode } from "react";
import { renderToStaticMarkup } from "react-dom/server";
import { beforeEach, describe, expect, it, vi } from "vitest";
import type { DiagnosticView, PartRow, ValueData, ValueView } from "@/api/types";
import { getT } from "@/i18n";
import { Th } from "@/kit/table/Th";
import { useTableSort } from "@/kit/table/useTableSort";
import { findElement, fire } from "@/test/elements";
import { fixture, qv } from "@/test/fixture";
import { PartsTable } from "./PartsTable";
import { partsComparators } from "./parts-sort";

// 既存の node 環境のヘルパーで操作し、再描画間の React state を保持する。
const state = vi.hoisted(() => ({ value: null as unknown }));
vi.mock("react", async (original) => ({
  ...(await original<typeof import("react")>()),
  useState: () => [
    state.value,
    (update: (current: unknown) => unknown) => {
      state.value = update(state.value);
    },
  ],
}));
vi.mock("@/i18n", async (original) => ({
  ...(await original<typeof import("@/i18n")>()),
  useT: () => getT(),
}));

function value(data: ValueData): ValueView {
  return {
    ...fixture.parts[0]!.values.mass!,
    attr: "mass",
    value: data,
    origin: data.type === "unset" ? "unset" : "override",
    display: "quantity",
  };
}
const scalar = (si: number) =>
  value({ type: "scalar", nominal: qv(si * 1000, "g", "Mass", si), min: null, max: null });
const quantities = [
  value({ type: "range", min: qv(10, "kg", "Mass"), max: qv(100, "kg", "Mass") }),
  scalar(2),
  value({ type: "by_state", states: { idle: qv(0, "kg", "Mass"), peak: qv(2, "kg", "Mass") } }),
  undefined,
  value({ type: "unset" }),
];
const items: PartRow[] = ["part10", "Part2", "part2", "empty", "unset"].map((id, i) => ({
  ...fixture.parts[0]!,
  id,
  system: i < 3 ? fixture.defs.systems[i === 0 ? 1 : 0]! : null,
  kind: i < 3 ? (i === 0 ? "kind10" : i === 1 ? "Kind2" : "kind2") : null,
  catalog: i < 3 ? (i === 0 ? "item10" : i === 1 ? "Item2" : "item2") : null,
  count: [10, 2, 2, 0, 1][i]!,
  values: {
    ...(quantities[i] ? { mass: quantities[i], power: quantities[i] } : {}),
    ...Object.fromEntries(
      Array.from({ length: [10, 2, 2, 0, 1][i]! }, (_, n) => [
        `unset${n}`,
        value({ type: "unset" }),
      ]),
    ),
  },
}));
// 質量・電力の未設定も件数に含めた期待値を使う。
function diagnostic(part: string, severity: DiagnosticView["severity"]): DiagnosticView {
  return {
    part,
    severity,
    path: null,
    excerpt: null,
    code: "test",
    message: "test",
    target: null,
    file: null,
    line: null,
    column: null,
  };
}
const design = {
  ...fixture,
  parts: items,
  diagnostics: [
    diagnostic("part10", "error"),
    diagnostic("part10", "error"),
    diagnostic("Part2", "error"),
    diagnostic("part2", "error"),
    diagnostic("part2", "warning"),
    diagnostic("unset", "warning"),
  ],
};
const comparators = partsComparators(design);
type Column = keyof typeof comparators;
const cases: [Column, number[], number[]][] = [
  ["id", [3, 1, 2, 0, 4], [4, 0, 1, 2, 3]],
  ...(["group", "kind", "catalog", "mass", "power"] as const).map(
    (key): [Column, number[], number[]] => [key, [1, 2, 0, 3, 4], [0, 1, 2, 3, 4]],
  ),
  ["count", [3, 4, 1, 2, 0], [0, 1, 2, 4, 3]],
  ["unset", [3, 1, 2, 4, 0], [0, 4, 1, 2, 3]],
  ["diagnostics", [3, 4, 1, 2, 0], [0, 2, 1, 4, 3]],
];

function click(node: ReactNode) {
  const button = findElement(node, (p) => p.type === "button");
  if (!button) throw new Error("missing sort button");
  fire(button, "onClick");
}
function render(column: Column) {
  // biome-ignore lint/correctness/useHookAtTopLevel: React state is mocked for the node test harness.
  const result = useTableSort(items, comparators);
  return { ...result, header: Th({ ...result.headerProps(column), children: column }) };
}
const ids = (rows: readonly PartRow[]) => rows.map((p) => p.id);
const expected = (indices: number[]) => indices.map((i) => items[i]!.id);

beforeEach(() => {
  state.value = null;
});
describe("parts table sorting", () => {
  it.each(cases)(
    "%s cycles ascending, descending, original with stable ties",
    (column, asc, desc) => {
      let result = render(column);
      expect(ids(result.rows)).toEqual(ids(items));
      for (const [direction, order] of [
        ["ascending", asc],
        ["descending", desc],
        ["none", [0, 1, 2, 3, 4]],
      ] as const) {
        click(result.header);
        result = render(column);
        expect(ids(result.rows)).toEqual(expected([...order]));
        expect(result.header.props["aria-sort"]).toBe(direction);
        const markup = renderToStaticMarkup(
          <table>
            <thead>
              <tr>{result.header}</tr>
            </thead>
          </table>,
        );
        expect(markup).toContain(`aria-sort="${direction}"`);
        expect(markup).toContain('<button type="button"');
      }
      expect(ids(items)).toEqual(["part10", "Part2", "part2", "empty", "unset"]);
    },
  );

  it("switches column to ascending and resets the previous aria-sort", () => {
    click(render("mass").header);
    click(render("mass").header);
    click(render("kind").header);
    expect(render("kind").header.props["aria-sort"]).toBe("ascending");
    expect(render("mass").header.props["aria-sort"]).toBe("none");
  });

  it("connects all nine headers and sorted rows in PartsTable", () => {
    const props = { design, items, selected: null, onSelect: vi.fn() };
    const table = PartsTable(props);
    const head = table.props.head;
    const headers: ReactElement<ComponentProps<typeof Th>>[] = head.type(head.props).props.children;
    expect(headers).toHaveLength(9);
    for (const header of headers) expect(header.props.sort?.direction).toBe("none");
    click(Th(headers[0]!.props));
    const sorted = PartsTable(props);
    const rows: ReactElement<{ p: PartRow; onSelect: () => void }>[] = sorted.props.children;
    expect(rows.map((row) => row.props.p.id)).toEqual(expected([3, 1, 2, 0, 4]));
    fire(rows[0]!, "onSelect");
    expect(props.onSelect).toHaveBeenCalledWith("empty");
  });
});
