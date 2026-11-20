import type { ReactNode } from "react";
import { renderToStaticMarkup } from "react-dom/server";
import { describe, expect, it, vi } from "vitest";
import type { DesignView, Op, PartRow, TreeNode } from "@/api/types";
import { getT } from "@/i18n";
import { fixture } from "@/test/fixture";
import { GroupActions } from "./GroupActions";
import { GroupPartsDialog } from "./GroupPartsDialog";

vi.mock("@/i18n", async (original) => ({
  ...(await original<typeof import("@/i18n")>()),
  useT: () => getT(),
}));
vi.mock("@/ops/use-op-check", () => ({
  useOpCheck: () => ({ error: null, ok: true, pending: false, result: null }),
}));
vi.mock("@/ops/OpDialog", () => ({
  OpDialog: ({ ops, children }: { ops: Op[] | null; children: ReactNode }) => (
    <div data-ops={JSON.stringify(ops)}>{children}</div>
  ),
}));

const part = (id: string, parent: string | null) => ({ id, parent }) as PartRow;
const design: DesignView = {
  ...fixture,
  parts: [part("RW_X", "aocs"), part("RW_Y", "aocs"), part("SAP", "power")],
};

describe("グループにまとめるダイアログ", () => {
  it("同じ親の部品だけを並べ、開いた部品に印を付け、グループの ID が決まるまで Op を作らない", () => {
    const html = renderToStaticMarkup(
      <GroupPartsDialog
        design={design}
        part={part("RW_X", "aocs")}
        open
        onOpenChange={() => {}}
        onDone={() => {}}
      />,
    );
    expect(html).toContain("RW_X");
    expect(html).toContain("RW_Y");
    expect(html).not.toContain("SAP");
    expect(html).toContain("checked");
    expect(html).toContain('data-ops="null"');
  });
});

describe("グループの枠の操作", () => {
  const node = (id: string, kind: "group" | "part", parent: string | null, children: string[]) =>
    ({ id, node: kind, label: id, label_en: null, system: null, parent, children }) as TreeNode;
  const nodes = new Map(
    [
      node("RW", "group", "aocs", ["RW_X"]),
      node("RW_X", "part", "RW", []),
      node("outer", "group", "aocs", ["RW"]),
    ].map((n) => [n.id, n]),
  );
  const t = getT().components.group;

  it("部品だけのグループは名前の変更と解除のボタンを出す", () => {
    const html = renderToStaticMarkup(
      <GroupActions group={nodes.get("RW") as TreeNode} nodes={nodes} />,
    );
    expect(html).toContain(`aria-label="${t.renameAction}"`);
    expect(html).toContain(`aria-label="${t.ungroupAction}"`);
  });

  it("入れ子のグループを含むグループは、解除のボタンを出さない", () => {
    const html = renderToStaticMarkup(
      <GroupActions group={nodes.get("outer") as TreeNode} nodes={nodes} />,
    );
    expect(html).toContain(`aria-label="${t.renameAction}"`);
    expect(html).not.toContain(t.ungroupAction);
  });
});
