import type { ReactNode } from "react";
import { renderToStaticMarkup } from "react-dom/server";
import { describe, expect, it, vi } from "vitest";
import type { CatalogItemView, DesignView, Op } from "@/api/types";
import { getT } from "@/i18n";
import { fixture } from "@/test/fixture";
import { AddPartDialog, initialAddPart } from "./AddPartDialog";

vi.mock("@/i18n", async (original) => ({
  ...(await original<typeof import("@/i18n")>()),
  useT: () => getT(),
}));
vi.mock("@/ops/use-op-check", () => ({
  useOpCheck: () => ({ failed: false, previewError: null, ok: true, pending: false, result: null }),
}));
vi.mock("@/ops/OpDialog", () => ({
  OpDialog: ({ ops, children }: { ops: Op[] | null; children: ReactNode }) => (
    <div data-ops={JSON.stringify(ops)}>{children}</div>
  ),
}));

const rw = {
  id: "lab/rw_10",
  scope: "lab",
  name: "rw_10",
  kind: "reaction_wheel",
  system: "aocs",
  label: "RW 10",
  label_en: "RW 10",
  manufacturer: null,
} as unknown as CatalogItemView;
const design: DesignView = {
  ...fixture,
  defs: { ...fixture.defs, systems: ["aocs", "power"] },
  catalog: [rw],
};

describe("コンポーネント追加ダイアログ（カタログの品から開く）", () => {
  it("品の系を置き場の既定にし、定義はカタログからで、品を選んだ状態にする", () => {
    const groups = [{ value: "aocs" }, { value: "power" }];
    expect(initialAddPart(design, groups, "lab/rw_10", null)).toEqual({
      by: "catalog",
      catalog: "lab/rw_10",
      group: "aocs",
    });
    // 系が置き場に無ければ先頭、絞り込みのグループを渡されていればそちら
    expect(initialAddPart(design, [{ value: "power" }], "lab/rw_10", null).group).toBe("power");
    expect(initialAddPart(design, groups, "lab/rw_10", "power").group).toBe("power");
  });

  it("選んだ品の ID と表示名を示し、ID の欄の例に name を出して、品が決まるまで確定の Op を作らない", () => {
    const html = renderToStaticMarkup(
      <AddPartDialog
        design={design}
        open
        onOpenChange={() => {}}
        onDone={() => {}}
        initialCatalog="lab/rw_10"
      />,
    );
    expect(html).toContain('data-testid="catalog-chosen"');
    expect(html).toContain("lab/rw_10");
    expect(html).toContain("RW 10");
    expect(html).toContain('placeholder="rw_10"');
    expect(html).toContain('data-ops="null"');
  });

  it("品を指定しなければ、探す欄を出す", () => {
    const html = renderToStaticMarkup(
      <AddPartDialog design={design} open onOpenChange={() => {}} onDone={() => {}} />,
    );
    expect(html).not.toContain('data-testid="catalog-chosen"');
    expect(html).toContain("ID・名前・種別で探す");
  });
});
