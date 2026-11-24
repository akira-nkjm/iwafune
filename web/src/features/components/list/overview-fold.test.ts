import { describe, expect, it } from "vitest";
import type { TreeNode } from "@/api/types";
import { foldable, foldedByDefault, partsWithin } from "./overview-fold";

function node(
  id: string,
  kind: TreeNode["node"],
  parent: string | null,
  children: string[],
): TreeNode {
  return {
    id,
    node: kind,
    parent,
    children,
    system: "power",
    label: id,
    label_en: null,
  } as TreeNode;
}

// 電源 > WING（グループ）> SAP（組立品）> SAP_A・SAP_B、電源 > CELLS（グループ）> CELL_1・CELL_2
const list = [
  node("power", "group", null, ["WING", "CELLS"]),
  node("WING", "group", "power", ["SAP"]),
  node("SAP", "part", "WING", ["SAP_A", "SAP_B"]),
  node("SAP_A", "part", "SAP", []),
  node("SAP_B", "part", "SAP", []),
  node("CELLS", "group", "power", ["CELL_1", "CELL_2"]),
  node("CELL_1", "part", "CELLS", []),
  node("CELL_2", "part", "CELLS", []),
];
const nodes = new Map(list.map((n) => [n.id, n]));
const all = { nodes, shown: () => true };
const get = (id: string) => nodes.get(id) as TreeNode;

describe("一覧のグループのたたみ方", () => {
  it("組立品を持つ入れ子のグループもたためる（ふだんは開いておく）", () => {
    expect(foldable(get("WING"), all)).toBe(true);
    expect(foldedByDefault(get("WING"), all)).toBe(false);
  });

  it("中が部品だけの入れ子のグループは、ふだんたたんでおく", () => {
    expect(foldable(get("CELLS"), all)).toBe(true);
    expect(foldedByDefault(get("CELLS"), all)).toBe(true);
  });

  it("系の枠（親の無いグループ）と組立品はたたまない", () => {
    expect(foldable(get("power"), all)).toBe(false);
    expect(foldable(get("SAP"), all)).toBe(false);
  });

  it("中の部品は、組立品とその子まで数える", () => {
    expect(partsWithin(get("WING"), all)).toEqual(["SAP", "SAP_A", "SAP_B"]);
  });

  it("絞り込みで出すものが無いグループはたたまない", () => {
    const none = { nodes, shown: (id: string) => !["SAP", "SAP_A", "SAP_B"].includes(id) };
    expect(foldable(get("WING"), none)).toBe(false);
  });
});
