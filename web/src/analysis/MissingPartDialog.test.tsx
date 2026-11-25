import type { ReactNode } from "react";
import { renderToStaticMarkup } from "react-dom/server";
import { expect, it, vi } from "vitest";
import type { Op } from "@/api/types";
import { getT } from "@/i18n";
import { MissingPartDialog } from "./MissingPartDialog";

vi.mock("@/i18n", async (original) => ({
  ...(await original<typeof import("@/i18n")>()),
  useT: () => getT(),
}));
vi.mock("@/api/ops", () => ({ useTextOp: () => ({ op: null, error: null }) }));
vi.mock("@/ops/OpDialog", () => ({
  OpDialog: ({
    ops,
    title,
    children,
  }: {
    ops: Op[] | null;
    title: string;
    children: ReactNode;
  }) => (
    <div data-ops={JSON.stringify(ops)} data-title={title}>
      {children}
    </div>
  ),
}));

it("割り当てる候補は、API が返した部品と種別だけを選ばせる（種別が 1 つなら選ばせない）", () => {
  const html = renderToStaticMarkup(
    <MissingPartDialog
      fix={{
        kind: "assign_part",
        roles: ["battery"],
        kinds: ["battery"],
        parts: ["OBC", "EPS"],
        command: "",
      }}
      onOpenChange={() => {}}
    />,
  );
  expect(html).toContain('data-title="既存の部品を割り当てる"');
  expect(html).toContain(">OBC</option>");
  expect(html).toContain(">EPS</option>");
  expect(html).not.toContain("missing-part-kind");
});

it("値を入れる候補は、候補の部品を選ばせて閉じた語彙の値を選択肢にする", () => {
  const html = renderToStaticMarkup(
    <MissingPartDialog
      fix={{
        kind: "set_value_on_part",
        attr: "chemistry",
        dimension: null,
        example: null,
        choices: [{ value: "li_ion", label: "Li-ion", label_en: null }],
        parts: ["BAT"],
        command: "",
      }}
      onOpenChange={() => {}}
    />,
  );
  expect(html).toContain(">BAT</option>");
  expect(html).toContain(">Li-ion</option>");
});
